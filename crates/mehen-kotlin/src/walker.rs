//! Tree-sitter-kotlin walker producing per-space metric output that
//! matches the pre-1.0 `legacy::metrics::*::compute for KotlinCode` arms.
//!
//! Modelled after `mehen-go/src/walker.rs` and `mehen-ruby/src/walker.rs`:
//! the walker drives its own tree-sitter cursor recursion so it can do
//! parent-aware classification — Kotlin's `is_else_if` predicate
//! requires walking parent and grandparent and consulting the outer
//! `if_expression`'s `alternative` field, which the generic
//! `mehen-tree-sitter::walker::LanguageRules` plug-in cannot express.
//!
//! Metric coverage mirrors the pre-1.0 SonarKotlin-aligned rules:
//! - **Cyclomatic**: every `if_expression`, every loop
//!   (`for`/`while`/`do-while`), every `when_entry`, plus each
//!   short-circuit `&&`/`||` operator (legacy
//!   `Cyclomatic for KotlinCode`).
//! - **Cognitive**: nesting on `if_expression` (skipping the inner `if`
//!   of an `else if`), loops, `when_expression`, `catch_block`; flat `+1`
//!   on every `else` keyword and on label-qualified
//!   `break@label`/`continue@label`; per-`&&`/`||` sequence collapse via
//!   the shared `BoolSequence` (separate runs for `ConjunctionExpression`
//!   / `DisjunctionExpression`); statement-shape boolean reset on
//!   `property_declaration`/`assignment`/`call_expression`/`jump_expression`;
//!   `prefix_expression` flips `not_operator` (legacy
//!   `Cognitive for KotlinCode`).
//! - **ABC**: assignments via `assignment` and `property_declaration`
//!   with an `=` token; branches via every `call_expression`;
//!   conditions via `if_expression`/`when_entry`/`catch_block`/
//!   loops/comparison & equality operators / `&&`/`||`/`?:`/`?.`/`!!`
//!   (legacy `Abc for KotlinCode`).
//! - **NExit**: `jump_expression` whose lead keyword child is `return`
//!   or `throw` (legacy `Exit for KotlinCode`).
//! - **NArgs**: per `compute_kotlin_args` — each
//!   `function_value_parameters`/`lambda_parameters` child contributes
//!   one per parameter-shaped grandchild (`class_parameter`,
//!   `function_value_parameter`, `parameter`, `parameter_with_optional_type`,
//!   `variable_declaration`); `setter`'s direct
//!   `parameter_with_optional_type` adds 1.
//! - **NOM**: every `function_declaration`, `anonymous_function`,
//!   `secondary_constructor`, `getter`, `setter` → function space (with
//!   `nom.record_function()`); every `lambda_literal` → function-shaped
//!   space but counted as `nom.record_closure()`/`closure_args`
//!   (legacy `is_func` / `is_closure` split).
//! - **LOC**: PLOC (set of code-line rows), LLOC (legacy SonarKotlin
//!   declaration- and statement-shape kinds plus
//!   statement-position `call_expression`), CLOC for
//!   `line_comment`/`multiline_comment`. Legacy `Loc for KotlinCode`.
//! - **Halstead**: per-node operator/operand emission using the legacy
//!   `Getter::get_op_type for KotlinCode` table. Operands dedup by text
//!   only (kind = `"Operand"`) so semantically-equal text from
//!   identifier-shaped nodes (`SimpleIdentifier`, `Identifier`,
//!   `TypeIdentifier`, `Field`, …) merges into one bucket — matching
//!   legacy's raw-byte-slice key.
//! - **NPA / NPM / WMC**: class-vs-interface routing via the
//!   declaration's leading keyword child (a `ClassDeclaration` containing
//!   an `Interface` keyword child is an interface). NPA counts
//!   `property_declaration` direct under a `class_body`/`enum_class_body`
//!   plus `class_parameter` with a `binding_pattern_kind` child (primary
//!   constructor properties). NPM counts `function_declaration`,
//!   `secondary_constructor`, `getter`, `setter` direct under a class /
//!   enum body, with public/non-public determined by explicit visibility
//!   modifier (default = public).

use mehen_core::{LineIndex, MetricSpace, SourceSpan, SpaceKind};
use mehen_metrics::{
    ContainerKind, HalsteadOperand, HalsteadOperator, MetricTreeBuilder, State, apply_state_to,
    finalize_state, merge_child_into_parent,
};
use smol_str::SmolStr;
use tree_sitter::Node;

use crate::grammar::Kotlin;

/// Drive the walker over the parsed Kotlin tree and return the populated
/// `MetricSpace`. Crate-internal entry point — only
/// `mehen_kotlin::KotlinAnalyzer::analyze` calls this.
pub(crate) fn walk_program(root: Node<'_>, source: &[u8], line_index: &LineIndex) -> MetricSpace {
    let unit_span = node_span(&root, line_index);

    let mut unit_state = State::new();
    unit_state.loc.set_span(
        root.start_position().row as u32,
        root.end_position().row as u32,
        true,
    );

    let mut visitor = Visitor {
        line_index,
        source,
        tree: MetricTreeBuilder::new(unit_span),
        stack: vec![unit_state],
        kinds: vec![SpaceKind::Unit],
        cognitive: CognitiveContext::default(),
    };
    visitor.visit(root);

    let mut unit_state = visitor.stack.pop().expect("walker stack underflow");
    finalize_state(&mut unit_state);
    apply_state_to(unit_state, visitor.tree.metrics_mut());
    visitor.tree.finish()
}

#[derive(Clone, Copy, Debug, Default)]
struct CognitiveContext {
    nesting: u32,
    depth: u32,
    lambda: u32,
}

struct Visitor<'a> {
    line_index: &'a LineIndex,
    source: &'a [u8],
    tree: MetricTreeBuilder,
    stack: Vec<State>,
    /// Parallel to `stack`: the SpaceKind of each frame so we can route
    /// closing-method WMC contributions and choose the correct ABC /
    /// NPA / NPM container without re-walking the AST.
    kinds: Vec<SpaceKind>,
    cognitive: CognitiveContext,
}

impl<'a> Visitor<'a> {
    fn current(&mut self) -> &mut State {
        self.stack.last_mut().expect("walker stack empty")
    }

    fn visit(&mut self, node: Node<'_>) {
        // Save the inherited cognitive context so any nesting bumps
        // applied by `classify` (or by `on_space_enter` for function
        // entry) scope to this node's subtree only — siblings see the
        // parent's pre-bump context. This mirrors how the legacy walker
        // looked up `(nesting, depth, lambda)` from a per-node-id map
        // keyed on the parent's id (`get_nesting_from_map`); when we
        // leave a subtree, the next sibling reads from the parent
        // again, not from the leaving subtree's bumped state.
        let saved_cognitive = self.cognitive;

        // NPA / NPM membership classification has to look at the
        // *enclosing* space's kind, not the soon-to-be-pushed one. Run
        // it before we open the function/method space so an inner
        // function's `kinds` stack still has the class on top.
        self.classify_class_members(&node, Kotlin::from(node.kind_id()));

        let opened = self.maybe_open_space(&node);
        if opened {
            self.on_space_enter();
        }

        // Per-node classification (everything except class membership,
        // which already ran above).
        self.classify(&node);

        // Recurse into children.
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.visit(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        if opened {
            self.close_space();
        }
        self.cognitive = saved_cognitive;
    }

    // ---------------------------------------------------------------
    // Space management
    // ---------------------------------------------------------------

    fn maybe_open_space(&mut self, node: &Node<'_>) -> bool {
        let kind_id = node.kind_id();
        let kind = Kotlin::from(kind_id);
        match kind {
            Kotlin::FunctionDeclaration
            | Kotlin::AnonymousFunction
            | Kotlin::SecondaryConstructor
            | Kotlin::Getter
            | Kotlin::Setter => {
                let name = func_name(node, self.source);
                let span = node_span(node, self.line_index);
                let mut child = State::new();
                child.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                child.nom.record_function();
                let argc = count_kotlin_args(node);
                child.nargs.record_function_args(argc);
                self.tree.open(SpaceKind::Function, span, name);
                self.stack.push(child);
                self.kinds.push(SpaceKind::Function);
                true
            }
            Kotlin::LambdaLiteral => {
                let span = node_span(node, self.line_index);
                let mut child = State::new();
                child.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                // Legacy `is_closure(LambdaLiteral) = true` and
                // `is_func(LambdaLiteral) = false`, so NOM/NArgs route to
                // the closure dimension. The space itself is still
                // SpaceKind::Function per legacy `get_space_kind`.
                child.nom.record_closure();
                let argc = count_kotlin_args(node);
                child.nargs.record_closure_args(argc);
                self.tree.open(SpaceKind::Function, span, None);
                self.stack.push(child);
                self.kinds.push(SpaceKind::Function);
                true
            }
            Kotlin::ClassDeclaration => {
                let name = func_name(node, self.source);
                let space_kind = if has_child_kind(node, Kotlin::Interface) {
                    SpaceKind::Interface
                } else {
                    SpaceKind::Class
                };
                let span = node_span(node, self.line_index);
                let mut child = State::new();
                child.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                if matches!(space_kind, SpaceKind::Interface) {
                    child.npa.record_class_like();
                    child.npm.record_class_like();
                } else {
                    child.npa.record_class_like();
                    child.npm.record_class_like();
                    child.wmc.record_class_like();
                }
                self.tree.open(space_kind.clone(), span, name);
                self.stack.push(child);
                self.kinds.push(space_kind);
                true
            }
            Kotlin::ObjectDeclaration | Kotlin::CompanionObject => {
                let name = func_name(node, self.source);
                let span = node_span(node, self.line_index);
                let mut child = State::new();
                child.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                child.npa.record_class_like();
                child.npm.record_class_like();
                child.wmc.record_class_like();
                self.tree.open(SpaceKind::Class, span, name);
                self.stack.push(child);
                self.kinds.push(SpaceKind::Class);
                true
            }
            _ => false,
        }
    }

    fn on_space_enter(&mut self) {
        let kind = self.kinds.last().cloned().expect("kinds stack empty");
        // Legacy `Cognitive for KotlinCode`'s
        // `FunctionDeclaration | AnonymousFunction | SecondaryConstructor`
        // arm: reset nesting / lambda, bump function-depth when nested
        // inside another function. ClassDeclaration / ObjectDeclaration
        // / CompanionObject open class-like spaces but don't carry
        // cognitive context themselves.
        if matches!(kind, SpaceKind::Function) {
            let nested_inside_function = self
                .kinds
                .iter()
                .rev()
                .skip(1)
                .any(|k| matches!(k, SpaceKind::Function));
            self.cognitive.nesting = 0;
            self.cognitive.lambda = 0;
            if nested_inside_function {
                self.cognitive.depth = self.cognitive.depth.saturating_add(1);
            }
        }
    }

    fn close_space(&mut self) {
        let closed_kind = self.kinds.pop().expect("kinds stack underflow");
        let mut state = self.stack.pop().expect("walker stack underflow");
        if matches!(closed_kind, SpaceKind::Function) {
            state.wmc.set_cyclomatic(state.cyclomatic.cyclomatic + 1);
        }
        finalize_state(&mut state);
        apply_state_to(state.clone(), self.tree.metrics_mut());
        if let Some(parent) = self.stack.last_mut() {
            let parent_kind = self.kinds.last().cloned().unwrap_or(SpaceKind::Unit);
            merge_child_into_parent(parent, &state);
            if matches!(closed_kind, SpaceKind::Function) {
                let container = match parent_kind {
                    SpaceKind::Class | SpaceKind::Impl => ContainerKind::Class,
                    SpaceKind::Interface | SpaceKind::Trait => ContainerKind::Interface,
                    _ => ContainerKind::Other,
                };
                state.wmc.finalize_method_into(container, &mut parent.wmc);
            }
        }
        self.tree.close();
    }

    // ---------------------------------------------------------------
    // Per-node classification
    // ---------------------------------------------------------------

    fn classify(&mut self, node: &Node<'_>) {
        let kind = Kotlin::from(node.kind_id());

        // Cyclomatic — legacy `Cyclomatic for KotlinCode`. The decision
        // set is aligned with SonarKotlin's CyclomaticComplexityVisitor:
        // `if`, every loop, every `when_entry`, every short-circuit
        // `&&`/`||`. `catch` is intentionally excluded.
        if matches!(
            kind,
            Kotlin::IfExpression
                | Kotlin::ForStatement
                | Kotlin::WhileStatement
                | Kotlin::DoWhileStatement
                | Kotlin::WhenEntry
                | Kotlin::AMPAMP
                | Kotlin::PIPEPIPE
        ) {
            self.current().cyclomatic.record_decision();
        }

        // Cognitive — legacy `Cognitive for KotlinCode`.
        self.classify_cognitive(node, kind);

        // ABC — legacy `Abc for KotlinCode`.
        self.classify_abc(node, kind);

        // NExit — legacy `Exit for KotlinCode`. JumpExpression with
        // lead keyword `return` or `throw` (filters out
        // `continue`/`break`/`return@label`).
        if kind == Kotlin::JumpExpression {
            let lead_kind = node.child(0).map(|c| Kotlin::from(c.kind_id()));
            if matches!(lead_kind, Some(Kotlin::Return) | Some(Kotlin::Throw)) {
                self.current().nexit.record_exit();
            }
        }

        // LOC — legacy `Loc for KotlinCode`.
        self.classify_loc(node, kind);

        // Halstead — legacy `Getter::get_op_type for KotlinCode`.
        self.classify_halstead(node, kind);
    }

    fn classify_cognitive(&mut self, node: &Node<'_>, kind: Kotlin) {
        match kind {
            // Nesting structures: `if` (not else-if), loops, `when`,
            // `catch_block`. `try` itself does NOT bump nesting; only
            // `catch_block` does.
            Kotlin::IfExpression if !is_else_if(node) => {
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                self.cognitive.nesting = self.cognitive.nesting.saturating_add(1);
            }
            Kotlin::IfExpression => {}
            Kotlin::ForStatement
            | Kotlin::WhileStatement
            | Kotlin::DoWhileStatement
            | Kotlin::WhenExpression
            | Kotlin::CatchBlock => {
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                self.cognitive.nesting = self.cognitive.nesting.saturating_add(1);
            }
            Kotlin::Else => {
                // `else` (covers `else if`) adds +1 without nesting.
                self.current().cognitive.increment_by_one();
            }
            // Label-qualified `break@label` / `continue@label` add +1
            // without nesting — they break linear flow like a goto.
            Kotlin::JumpExpression => {
                let lead_kind = node.child(0).map(|c| Kotlin::from(c.kind_id()));
                if matches!(lead_kind, Some(Kotlin::BreakAT) | Some(Kotlin::ContinueAT)) {
                    self.current().cognitive.increment_by_one();
                }
                // Statement-boundary boolean-sequence reset.
                self.current().cognitive.boolean_seq.reset();
            }
            Kotlin::PropertyDeclaration | Kotlin::Assignment | Kotlin::CallExpression => {
                self.current().cognitive.boolean_seq.reset();
            }
            Kotlin::PrefixExpression => {
                // Legacy passes the PrefixExpression's kind_id as the
                // not_id; any subsequent boolean operator will compare
                // unequal and trigger a +1 — same effect with a stable
                // string label.
                self.current().cognitive.boolean_seq.not_operator("!");
            }
            Kotlin::ConjunctionExpression => {
                // Legacy `compute_booleans::<Kotlin>(node, stats, &AMPAMP, &AMPAMP)`
                // — every `&&` child of this conjunction folds into the
                // sequence collapser. `compute_booleans` walks direct
                // children only and only matches on AMPAMP.
                for child in iter_children(node) {
                    if Kotlin::from(child.kind_id()) == Kotlin::AMPAMP {
                        self.current().cognitive.observe_boolean("&&");
                    }
                }
            }
            Kotlin::DisjunctionExpression => {
                for child in iter_children(node) {
                    if Kotlin::from(child.kind_id()) == Kotlin::PIPEPIPE {
                        self.current().cognitive.observe_boolean("||");
                    }
                }
            }
            _ => {}
        }
    }

    fn classify_abc(&mut self, _node: &Node<'_>, kind: Kotlin) {
        match kind {
            Kotlin::Assignment => {
                self.current().abc.record_assignment();
            }
            Kotlin::PropertyDeclaration if has_child_kind(_node, Kotlin::EQ) => {
                self.current().abc.record_assignment();
            }
            Kotlin::CallExpression => {
                self.current().abc.record_branch();
            }
            Kotlin::IfExpression
            | Kotlin::WhenEntry
            | Kotlin::CatchBlock
            | Kotlin::ForStatement
            | Kotlin::WhileStatement
            | Kotlin::DoWhileStatement
            | Kotlin::EQEQ
            | Kotlin::BANGEQ
            | Kotlin::EQEQEQ
            | Kotlin::BANGEQEQ
            | Kotlin::LT
            | Kotlin::LTEQ
            | Kotlin::GT
            | Kotlin::GTEQ
            | Kotlin::AMPAMP
            | Kotlin::PIPEPIPE
            | Kotlin::QMARKCOLON
            | Kotlin::QMARKDOT
            | Kotlin::BANGBANG => {
                self.current().abc.record_condition();
            }
            _ => {}
        }
    }

    fn classify_class_members(&mut self, node: &Node<'_>, kind: Kotlin) {
        // Mirrors legacy `Npa for KotlinCode` / `Npm for KotlinCode`.
        // We look at the immediate parent kind to decide whether the
        // current node is a direct member of a class-like body.
        let parent_kind = self.kinds.last().cloned().unwrap_or(SpaceKind::Unit);
        let in_class_like = matches!(
            parent_kind,
            SpaceKind::Class | SpaceKind::Interface | SpaceKind::Impl | SpaceKind::Trait
        );

        if !in_class_like {
            return;
        }

        match kind {
            Kotlin::PropertyDeclaration => {
                let Some(parent) = node.parent() else {
                    return;
                };
                if !matches!(
                    Kotlin::from(parent.kind_id()),
                    Kotlin::ClassBody | Kotlin::EnumClassBody
                ) {
                    return;
                }
                let Some(container) = kotlin_member_container(&parent) else {
                    return;
                };
                let public = kotlin_member_is_public(node, self.source);
                let container_kind = match container {
                    SpaceKind::Class | SpaceKind::Impl => ContainerKind::Class,
                    SpaceKind::Interface | SpaceKind::Trait => ContainerKind::Interface,
                    _ => return,
                };
                self.current().npa.record_attribute(container_kind, public);
            }
            Kotlin::ClassParameter => {
                // Constructor property: only `class C(val x: Int)` /
                // `(var x: Int)` count as class attributes.
                if !has_child_kind(node, Kotlin::BindingPatternKind) {
                    return;
                }
                let Some(container) = kotlin_constructor_param_container(node) else {
                    return;
                };
                let public = kotlin_member_is_public(node, self.source);
                let container_kind = match container {
                    SpaceKind::Class | SpaceKind::Impl => ContainerKind::Class,
                    SpaceKind::Interface | SpaceKind::Trait => ContainerKind::Interface,
                    _ => return,
                };
                self.current().npa.record_attribute(container_kind, public);
            }
            Kotlin::FunctionDeclaration
            | Kotlin::SecondaryConstructor
            | Kotlin::Getter
            | Kotlin::Setter => {
                let Some(parent) = node.parent() else {
                    return;
                };
                if !matches!(
                    Kotlin::from(parent.kind_id()),
                    Kotlin::ClassBody | Kotlin::EnumClassBody
                ) {
                    return;
                }
                let Some(container) = kotlin_member_container(&parent) else {
                    return;
                };
                let public = if matches!(kind, Kotlin::Getter | Kotlin::Setter) {
                    kotlin_accessor_is_public(node, self.source)
                } else {
                    kotlin_member_is_public(node, self.source)
                };
                let container_kind = match container {
                    SpaceKind::Class | SpaceKind::Impl => ContainerKind::Class,
                    SpaceKind::Interface | SpaceKind::Trait => ContainerKind::Interface,
                    _ => return,
                };
                self.current().npm.record_method(container_kind, public);
            }
            _ => {}
        }
    }

    fn classify_loc(&mut self, node: &Node<'_>, kind: Kotlin) {
        match kind {
            Kotlin::SourceFile | Kotlin::Statements | Kotlin::StringLiteral => {}
            Kotlin::LineComment | Kotlin::MultilineComment => {
                let start = node.start_position().row as u32;
                let end = node.end_position().row as u32;
                self.current().loc.observe_comment(start, end);
            }
            Kotlin::FunctionDeclaration
            | Kotlin::ClassDeclaration
            | Kotlin::ObjectDeclaration
            | Kotlin::CompanionObject
            | Kotlin::SecondaryConstructor
            | Kotlin::PropertyDeclaration
            | Kotlin::Getter
            | Kotlin::Setter
            | Kotlin::Assignment
            | Kotlin::ForStatement
            | Kotlin::WhileStatement
            | Kotlin::DoWhileStatement
            | Kotlin::IfExpression
            | Kotlin::WhenExpression
            | Kotlin::TryExpression
            | Kotlin::JumpExpression => {
                self.current().loc.observe_lloc();
            }
            Kotlin::CallExpression
                if node.parent().is_some_and(|p| {
                    matches!(
                        Kotlin::from(p.kind_id()),
                        Kotlin::Statements | Kotlin::ControlStructureBody
                    )
                }) =>
            {
                self.current().loc.observe_lloc();
            }
            _ => {
                let row = node.start_position().row as u32;
                self.current().loc.observe_code_line(row);
            }
        }
    }

    fn classify_halstead(&mut self, node: &Node<'_>, kind: Kotlin) {
        match halstead_op_type(kind) {
            HalsteadType::Operator => {
                let label: &'static str = kind.into();
                self.current().halstead.observe_operator(HalsteadOperator {
                    kind: SmolStr::new(label),
                    text: None,
                });
            }
            HalsteadType::Operand => {
                let text = text_of(node, self.source);
                self.current().halstead.observe_operand(HalsteadOperand {
                    kind: SmolStr::new("Operand"),
                    text: Some(SmolStr::new(text)),
                });
            }
            HalsteadType::Unknown => {}
        }
    }
}

// --------------------------------------------------------------------
// Halstead classification (legacy `Getter::get_op_type for KotlinCode`).
// --------------------------------------------------------------------

enum HalsteadType {
    Operator,
    Operand,
    Unknown,
}

fn halstead_op_type(kind: Kotlin) -> HalsteadType {
    match kind {
        // Keywords and control-flow tokens.
        Kotlin::Fun
        | Kotlin::Val
        | Kotlin::Var
        | Kotlin::Class
        | Kotlin::Interface
        | Kotlin::Object
        | Kotlin::Enum
        | Kotlin::Data
        | Kotlin::Sealed
        | Kotlin::Open
        | Kotlin::Abstract
        | Kotlin::Final
        | Kotlin::Override
        | Kotlin::Private
        | Kotlin::Public
        | Kotlin::Protected
        | Kotlin::Internal
        | Kotlin::Inner
        | Kotlin::Companion
        | Kotlin::Init
        | Kotlin::Constructor
        | Kotlin::Typealias
        | Kotlin::Import
        | Kotlin::Package
        | Kotlin::If
        | Kotlin::Else
        | Kotlin::When
        | Kotlin::Try
        | Kotlin::Catch
        | Kotlin::Finally
        | Kotlin::Throw
        | Kotlin::Return
        | Kotlin::Continue
        | Kotlin::Break
        | Kotlin::For
        | Kotlin::While
        | Kotlin::Do
        | Kotlin::In
        | Kotlin::Is
        | Kotlin::As
        | Kotlin::AsQMARK
        | Kotlin::By
        | Kotlin::Where
        | Kotlin::Suspend
        | Kotlin::Inline
        | Kotlin::Infix
        | Kotlin::Operator
        | Kotlin::Tailrec
        | Kotlin::External
        | Kotlin::Lateinit
        | Kotlin::Noinline
        | Kotlin::Crossinline
        | Kotlin::Vararg
        | Kotlin::Out
        | Kotlin::Get
        | Kotlin::Set
        // Assignment / augmented assignment.
        | Kotlin::EQ
        | Kotlin::PLUSEQ
        | Kotlin::DASHEQ
        | Kotlin::STAREQ
        | Kotlin::SLASHEQ
        | Kotlin::PERCENTEQ
        // Comparison / arithmetic / logical operators.
        | Kotlin::PLUS
        | Kotlin::DASH
        | Kotlin::STAR
        | Kotlin::SLASH
        | Kotlin::PERCENT
        | Kotlin::AMPAMP
        | Kotlin::PIPEPIPE
        | Kotlin::BANG
        | Kotlin::BANGBANG
        | Kotlin::LT
        | Kotlin::GT
        | Kotlin::LTEQ
        | Kotlin::GTEQ
        | Kotlin::EQEQ
        | Kotlin::BANGEQ
        | Kotlin::EQEQEQ
        | Kotlin::BANGEQEQ
        | Kotlin::BANGin
        | Kotlin::BANGis
        | Kotlin::QMARKCOLON
        | Kotlin::QMARKDOT
        // Structural punctuation.
        | Kotlin::LPAREN
        | Kotlin::LBRACE
        | Kotlin::LBRACK
        | Kotlin::DOT
        | Kotlin::COMMA
        | Kotlin::SEMI
        | Kotlin::COLON
        | Kotlin::COLONCOLON
        | Kotlin::DASHGT
        | Kotlin::DOTDOT
        | Kotlin::PLUSPLUS
        | Kotlin::DASHDASH => HalsteadType::Operator,

        // Operands: identifiers, literals, this/super, null, field.
        Kotlin::SimpleIdentifier
        | Kotlin::Identifier
        | Kotlin::TypeIdentifier
        | Kotlin::IntegerLiteral
        | Kotlin::HexLiteral
        | Kotlin::BinLiteral
        | Kotlin::LongLiteral
        | Kotlin::RealLiteral
        | Kotlin::UnsignedLiteral
        | Kotlin::CharacterLiteral
        | Kotlin::StringLiteral
        | Kotlin::True
        | Kotlin::False
        | Kotlin::BooleanLiteral
        | Kotlin::NullLiteral
        | Kotlin::This
        | Kotlin::ThisExpression
        | Kotlin::Super
        | Kotlin::SuperExpression
        | Kotlin::Field => HalsteadType::Operand,

        _ => HalsteadType::Unknown,
    }
}

// --------------------------------------------------------------------
// NPM / NPA helpers — direct ports of legacy `kotlin_*` functions.
// --------------------------------------------------------------------

/// Resolve the class-vs-interface container for a member whose parent
/// node is a `class_body` / `enum_class_body`. The decision is based on
/// the leading keyword of the enclosing declaration: a
/// `class_declaration` containing an `interface` keyword child is an
/// interface-like container.
fn kotlin_member_container(body_parent: &Node<'_>) -> Option<SpaceKind> {
    let decl = body_parent.parent()?;
    match Kotlin::from(decl.kind_id()) {
        Kotlin::ClassDeclaration => {
            if has_child_kind(&decl, Kotlin::Interface) {
                Some(SpaceKind::Interface)
            } else {
                Some(SpaceKind::Class)
            }
        }
        Kotlin::ObjectDeclaration | Kotlin::CompanionObject => Some(SpaceKind::Class),
        _ => None,
    }
}

/// Resolve the class-vs-interface container for a `class_parameter`
/// (primary constructor parameter). The parent chain is
/// `class_parameter > primary_constructor > class_declaration`.
fn kotlin_constructor_param_container(node: &Node<'_>) -> Option<SpaceKind> {
    let primary = node.parent()?;
    let decl = primary.parent()?;
    match Kotlin::from(decl.kind_id()) {
        Kotlin::ClassDeclaration => {
            if has_child_kind(&decl, Kotlin::Interface) {
                Some(SpaceKind::Interface)
            } else {
                Some(SpaceKind::Class)
            }
        }
        _ => None,
    }
}

/// Explicit visibility on a Kotlin declaration-like node. Returns
/// `Some(true)` for `public`, `Some(false)` for `private`/`protected`/
/// `internal`, and `None` when there is no visibility modifier.
fn kotlin_member_visibility(node: &Node<'_>, source: &[u8]) -> Option<bool> {
    for child in iter_children(node) {
        if !matches!(
            Kotlin::from(child.kind_id()),
            Kotlin::Modifiers | Kotlin::ParameterModifiers
        ) {
            continue;
        }
        for m in iter_children(&child) {
            if Kotlin::from(m.kind_id()) != Kotlin::VisibilityModifier {
                continue;
            }
            let text = &source[m.start_byte()..m.end_byte()];
            if text == b"private" || text == b"protected" || text == b"internal" {
                return Some(false);
            }
            if text == b"public" {
                return Some(true);
            }
        }
    }
    None
}

/// Default-public unless an explicit modifier overrides.
fn kotlin_member_is_public(node: &Node<'_>, source: &[u8]) -> bool {
    kotlin_member_visibility(node, source).unwrap_or(true)
}

/// Visibility for property accessors: explicit modifier on the accessor
/// > property's modifier > default (public).
fn kotlin_accessor_is_public(node: &Node<'_>, source: &[u8]) -> bool {
    if let Some(v) = kotlin_member_visibility(node, source) {
        return v;
    }
    if let Some(v) = kotlin_previous_property_visibility(node, source) {
        return v;
    }
    true
}

fn kotlin_previous_property_visibility(node: &Node<'_>, source: &[u8]) -> Option<bool> {
    let parent = node.parent()?;
    let mut property_visibility = None;
    let target_id = node.id();
    for child in iter_children(&parent) {
        if child.id() == target_id {
            break;
        }
        if Kotlin::from(child.kind_id()) == Kotlin::PropertyDeclaration {
            property_visibility = kotlin_member_visibility(&child, source);
        }
    }
    property_visibility
}

// --------------------------------------------------------------------
// NArgs helper — direct port of legacy `compute_kotlin_args`.
// --------------------------------------------------------------------

fn count_kotlin_args(node: &Node<'_>) -> u32 {
    let host_kind = Kotlin::from(node.kind_id());
    let mut total: u32 = 0;
    for child in iter_children(node) {
        match Kotlin::from(child.kind_id()) {
            Kotlin::FunctionValueParameters | Kotlin::LambdaParameters => {
                for p in iter_children(&child) {
                    if matches!(
                        Kotlin::from(p.kind_id()),
                        Kotlin::ClassParameter
                            | Kotlin::FunctionValueParameter
                            | Kotlin::Parameter
                            | Kotlin::ParameterWithOptionalType
                            | Kotlin::VariableDeclaration
                    ) {
                        total = total.saturating_add(1);
                    }
                }
            }
            Kotlin::ParameterWithOptionalType if host_kind == Kotlin::Setter => {
                total = total.saturating_add(1);
            }
            _ => {}
        }
    }
    total
}

// --------------------------------------------------------------------
// Tree-sitter helpers
// --------------------------------------------------------------------

/// Kotlin's tree-sitter grammar tags class/interface/object/fun
/// declarations' names as plain `simple_identifier`/`type_identifier`/
/// `identifier` children rather than via a `name` field, so we look at
/// the first identifier-shaped direct child.
fn func_name(node: &Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(name) = node.child_by_field_name("name") {
        return std::str::from_utf8(&source[name.start_byte()..name.end_byte()])
            .ok()
            .map(|s| s.to_string());
    }
    for child in iter_children(node) {
        if matches!(
            Kotlin::from(child.kind_id()),
            Kotlin::SimpleIdentifier | Kotlin::TypeIdentifier | Kotlin::Identifier
        ) {
            return std::str::from_utf8(&source[child.start_byte()..child.end_byte()])
                .ok()
                .map(|s| s.to_string());
        }
    }
    None
}

fn is_else_if(node: &Node<'_>) -> bool {
    if Kotlin::from(node.kind_id()) != Kotlin::IfExpression {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    if Kotlin::from(parent.kind_id()) != Kotlin::ControlStructureBody {
        return false;
    }
    let Some(grand) = parent.parent() else {
        return false;
    };
    if Kotlin::from(grand.kind_id()) != Kotlin::IfExpression {
        return false;
    }
    grand
        .child_by_field_name("alternative")
        .is_some_and(|alt| alt.id() == parent.id())
}

fn has_child_kind(node: &Node<'_>, kind: Kotlin) -> bool {
    iter_children(node).any(|c| Kotlin::from(c.kind_id()) == kind)
}

fn iter_children<'tree>(node: &Node<'tree>) -> impl Iterator<Item = Node<'tree>> {
    let mut cursor = node.walk();
    let mut nodes = Vec::new();
    if cursor.goto_first_child() {
        loop {
            nodes.push(cursor.node());
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    nodes.into_iter()
}

fn node_span(node: &Node<'_>, line_index: &LineIndex) -> SourceSpan {
    let start_byte = node.start_byte() as u32;
    let end_byte = node.end_byte() as u32;
    SourceSpan {
        start_byte,
        end_byte,
        start_line: line_index.line_at(start_byte),
        end_line: line_index.line_at(end_byte.saturating_sub(1).max(start_byte)),
    }
}

fn text_of<'src>(node: &Node<'_>, source: &'src [u8]) -> &'src str {
    let start = node.start_byte();
    let end = node.end_byte().min(source.len());
    if start >= end {
        return "";
    }
    core::str::from_utf8(&source[start..end]).unwrap_or("")
}
