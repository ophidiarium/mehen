//! Tree-sitter-go walker producing per-space metric output that matches
//! the pre-1.0 `legacy::metrics::*::compute for GoCode` arms exactly.
//!
//! The walker drives its own tree-sitter cursor recursion (rather than
//! the generic `mehen-tree-sitter::walker::LanguageRules` plug-in) so it
//! can do parent-aware classification — Go's `is_else_if` and
//! `default_case in select` predicates both inspect the parent node,
//! which the generic walker cannot express.
//!
//! Metric coverage:
//! - **Cyclomatic**: every `if_statement`, `for_statement`,
//!   `expression_case`, `type_case`, `communication_case`, every `&&`/`||`
//!   token, plus a `default_case` whose parent is `select_statement`
//!   (legacy: `Cyclomatic for GoCode`).
//! - **Cognitive**: nesting on `if_statement` (skipping the inner `if`
//!   of an `else if`), `for_statement`,
//!   `expression_switch_statement`, `type_switch_statement`,
//!   `select_statement`; flat `+1` on every `else` keyword;
//!   boolean-sequence reset on every statement-shape node;
//!   `not_operator("!")` for unary `!` operators; per-`&&`/`||`
//!   sequence collapse via the shared `BoolSequence` (legacy:
//!   `Cognitive for GoCode`).
//! - **ABC**: assignments via `assignment_statement` /
//!   `short_var_declaration` (target count from the `left` field) and
//!   `inc_statement` / `dec_statement` (one each), `receive_statement`
//!   / `range_clause` only when the `left` field is present, `var_spec`
//!   with an `=` token, and `const_spec`. Branches: every
//!   `call_expression`. Conditions: `if`, `for`, every `case` arm,
//!   plus the comparison + boolean operator tokens. (Legacy:
//!   `Abc for GoCode`.)
//! - **NExit**: `return_statement` (legacy: `Exit for GoCode`).
//! - **NArgs**: per-`parameter_declaration` /
//!   `variadic_parameter_declaration` count = `max(1, identifier_count)`
//!   (legacy: `compute_go_args`).
//! - **NOM**: every `function_declaration` / `method_declaration` →
//!   function space; every `func_literal` → closure space.
//! - **LOC**: PLOC (every node default arm), LLOC (the legacy 26-kind
//!   set), CLOC (every `comment` node). Legacy: `Loc for GoCode`.
//! - **Halstead**: per-node operator/operand emission using the
//!   legacy `Getter::get_op_type for GoCode` table. Operands dedup by
//!   text only (kind = `"Operand"`) so semantically-equal text from
//!   different identifier-shaped nodes (e.g. `Identifier`,
//!   `Identifier2`, `Identifier3`, `BlankIdentifier`,
//!   `FieldIdentifier`, `LabelName`, `PackageIdentifier`,
//!   `TypeIdentifier`) merges into a single bucket — matching the
//!   legacy raw-byte-slice key.
//! - **NPA / NPM / WMC**: Go has no class-like constructs; all three
//!   are intentionally no-ops, matching the legacy
//!   `impl X for GoCode` empty bodies.

use mehen_core::{LineIndex, MetricSpace, SourceSpan, SpaceKind};
use mehen_metrics::{
    HalsteadOperand, HalsteadOperator, MetricTreeBuilder, State, apply_state_to, finalize_state,
    merge_child_into_parent,
};
use smol_str::SmolStr;
use tree_sitter::Node;

use crate::grammar::Go;

/// Drive the walker over the parsed Go tree and return the populated
/// `MetricSpace`. Mirrors `mehen_ruby::walker::walk_program` and the
/// legacy `spaces::metrics<GoParser>` entry point.
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
    kinds: Vec<SpaceKind>,
    cognitive: CognitiveContext,
}

impl<'a> Visitor<'a> {
    fn current(&mut self) -> &mut State {
        self.stack.last_mut().expect("walker stack empty")
    }

    fn visit(&mut self, node: Node<'_>) {
        // Detect space-open. Side effects on space open run before the
        // per-node classification so the per-space accumulator the
        // child observations land on is the *new* one.
        let saved_cognitive = self.cognitive;
        let opened = self.maybe_open_space(&node);
        if opened {
            self.on_space_enter();
        }

        // Per-node classification. The order does not matter for
        // metric arithmetic, only for the cognitive `boolean_seq` /
        // `nesting` state machine — which is bounded entirely by the
        // single `match` arm below.
        self.classify(&node);

        // Recurse into children. Mirrors the legacy
        // `cursor.goto_first_child + goto_next_sibling` walk.
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
            self.cognitive = saved_cognitive;
        }
    }

    // ---------------------------------------------------------------
    // Space management
    // ---------------------------------------------------------------

    fn maybe_open_space(&mut self, node: &Node<'_>) -> bool {
        match Go::from(node.kind_id()) {
            Go::FunctionDeclaration | Go::MethodDeclaration => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, self.source).to_string());
                let span = node_span(node, self.line_index);
                let mut child = State::new();
                child.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                child.nom.record_function();
                let argc = count_go_args(node);
                child.nargs.record_function_args(argc);
                self.tree.open(SpaceKind::Function, span, name);
                self.stack.push(child);
                self.kinds.push(SpaceKind::Function);
                true
            }
            Go::FuncLiteral => {
                let span = node_span(node, self.line_index);
                let mut child = State::new();
                child.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                child.nom.record_closure();
                let argc = count_go_args(node);
                child.nargs.record_closure_args(argc);
                self.tree.open(SpaceKind::Closure, span, None);
                self.stack.push(child);
                self.kinds.push(SpaceKind::Closure);
                true
            }
            _ => false,
        }
    }

    fn on_space_enter(&mut self) {
        let kind = self.kinds.last().expect("kinds stack empty");
        match kind {
            SpaceKind::Function => {
                // Legacy `Cognitive for GoCode`'s `FunctionDeclaration | MethodDeclaration`
                // arm: reset nesting, bump function-depth when nested.
                let nested_inside_function = self
                    .kinds
                    .iter()
                    .rev()
                    .skip(1)
                    .any(|k| matches!(k, SpaceKind::Function));
                self.cognitive.nesting = 0;
                if nested_inside_function {
                    self.cognitive.depth = self.cognitive.depth.saturating_add(1);
                }
            }
            SpaceKind::Closure => {
                // Legacy `FuncLiteral` arm: bump lambda counter only;
                // nesting/depth pass through unchanged.
                self.cognitive.lambda = self.cognitive.lambda.saturating_add(1);
            }
            _ => {}
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
            merge_child_into_parent(parent, &state);
        }
        self.tree.close();
    }

    // ---------------------------------------------------------------
    // Per-node classification
    // ---------------------------------------------------------------

    fn classify(&mut self, node: &Node<'_>) {
        let kind = Go::from(node.kind_id());

        // Cyclomatic — legacy `Cyclomatic for GoCode`. `default_case`
        // inside a `select` is a real communication branch; inside a
        // `switch` it's fallthrough and does not count.
        let is_decision = matches!(
            kind,
            Go::IfStatement
                | Go::ForStatement
                | Go::ExpressionCase
                | Go::TypeCase
                | Go::CommunicationCase
                | Go::AMPAMP
                | Go::PIPEPIPE
        ) || (matches!(kind, Go::DefaultCase)
            && parent_kind(node) == Some(Go::SelectStatement));
        if is_decision {
            self.current().cyclomatic.record_decision();
        }

        // Cognitive — legacy `Cognitive for GoCode`. Side effects on the
        // boolean_seq state machine and on the cognitive context (which
        // is per-recursion-frame, not per-space) live here.
        self.classify_cognitive(node, kind);

        // ABC — legacy `Abc for GoCode`.
        self.classify_abc(node, kind);

        // NExit — legacy `Exit for GoCode`.
        if matches!(kind, Go::ReturnStatement) {
            self.current().nexit.record_exit();
        }

        // LOC — legacy `Loc for GoCode`.
        self.classify_loc(node, kind);

        // Halstead — legacy `Getter::get_op_type for GoCode`.
        self.classify_halstead(node, kind);
    }

    fn classify_cognitive(&mut self, node: &Node<'_>, kind: Go) {
        match kind {
            // The else-if form (`IfStatement` whose direct parent is
            // also an `IfStatement`) is a no-op in legacy — the outer
            // `if` already opened a nesting level and the connecting
            // `else` keyword adds the flat `+1`.
            Go::IfStatement if !is_else_if(node) => {
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                self.cognitive.nesting = self.cognitive.nesting.saturating_add(1);
            }
            Go::IfStatement => {}
            Go::ForStatement
            | Go::ExpressionSwitchStatement
            | Go::TypeSwitchStatement
            | Go::SelectStatement => {
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                self.cognitive.nesting = self.cognitive.nesting.saturating_add(1);
            }
            Go::Else => {
                self.current().cognitive.increment_by_one();
            }
            Go::ExpressionStatement
            | Go::SendStatement
            | Go::ReceiveStatement
            | Go::IncStatement
            | Go::DecStatement
            | Go::AssignmentStatement
            | Go::ShortVarDeclaration
            | Go::VarSpec
            | Go::ConstSpec
            | Go::ReturnStatement => {
                self.current().cognitive.boolean_seq.reset();
            }
            // Legacy passes the literal node kind_id (the top-level
            // `unary_expression` kind, not the operator child) into
            // `boolean_seq.not_operator`. We forward a stable `"!"`
            // marker — same effect because `eval_based_on_prev` only
            // cares whether the recorded last_op equals the new
            // boolean operator string.
            Go::UnaryExpression if has_child_kind(node, Go::BANG) => {
                self.current().cognitive.boolean_seq.not_operator("!");
            }
            Go::BinaryExpression => {
                // Legacy `compute_booleans::<Go>`: walk the children;
                // for each `&&` / `||` operator child, feed the
                // sequence collapser. The actual punctuation is one of
                // the binary expression's children.
                for child in iter_children(node) {
                    match Go::from(child.kind_id()) {
                        Go::AMPAMP => self.current().cognitive.observe_boolean("&&"),
                        Go::PIPEPIPE => self.current().cognitive.observe_boolean("||"),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn classify_abc(&mut self, node: &Node<'_>, kind: Go) {
        match kind {
            Go::AssignmentStatement | Go::ShortVarDeclaration => {
                let count = go_assignment_target_count(node);
                self.current().abc.assignments =
                    self.current().abc.assignments.saturating_add(count);
            }
            Go::ReceiveStatement | Go::RangeClause
                if node.child_by_field_name("left").is_some() =>
            {
                let count = go_assignment_target_count(node);
                self.current().abc.assignments =
                    self.current().abc.assignments.saturating_add(count);
            }
            Go::IncStatement | Go::DecStatement => {
                self.current().abc.record_assignment();
            }
            Go::ConstSpec => {
                let count = go_spec_name_count(node);
                self.current().abc.assignments =
                    self.current().abc.assignments.saturating_add(count);
            }
            Go::VarSpec if has_child_kind(node, Go::EQ) => {
                let count = go_spec_name_count(node);
                self.current().abc.assignments =
                    self.current().abc.assignments.saturating_add(count);
            }
            Go::CallExpression => {
                self.current().abc.record_branch();
            }
            Go::IfStatement
            | Go::ForStatement
            | Go::ExpressionCase
            | Go::DefaultCase
            | Go::TypeCase
            | Go::CommunicationCase
            | Go::EQEQ
            | Go::BANGEQ
            | Go::LT
            | Go::LTEQ
            | Go::GT
            | Go::GTEQ
            | Go::AMPAMP
            | Go::PIPEPIPE
            | Go::BANG => {
                self.current().abc.record_condition();
            }
            _ => {}
        }
    }

    fn classify_loc(&mut self, node: &Node<'_>, kind: Go) {
        match kind {
            Go::SourceFile => {}
            Go::Comment => {
                let start = node.start_position().row as u32;
                let end = node.end_position().row as u32;
                self.current().loc.observe_comment(start, end);
            }
            Go::ExpressionStatement
            | Go::SendStatement
            | Go::ReceiveStatement
            | Go::IncStatement
            | Go::DecStatement
            | Go::AssignmentStatement
            | Go::ShortVarDeclaration
            | Go::ImportSpec
            | Go::VarSpec
            | Go::ConstSpec
            | Go::TypeSpec
            | Go::EmptyStatement
            | Go::LabeledStatement
            | Go::LabeledStatement2
            | Go::GoStatement
            | Go::DeferStatement
            | Go::ReturnStatement
            | Go::BreakStatement
            | Go::ContinueStatement
            | Go::GotoStatement
            | Go::FallthroughStatement
            | Go::IfStatement
            | Go::ExpressionSwitchStatement
            | Go::TypeSwitchStatement
            | Go::SelectStatement
            | Go::ForStatement => {
                self.current().loc.observe_lloc();
            }
            _ => {
                let start = node.start_position().row as u32;
                self.current().loc.observe_code_line(start);
            }
        }
    }

    fn classify_halstead(&mut self, node: &Node<'_>, kind: Go) {
        // Halstead routes to the *current* (innermost) space so nested
        // function bodies carry their own counts; the close path's
        // `merge_child_into_parent` rolls these up into the enclosing
        // scope and the unit (set-union for `n1`/`n2`, sum for
        // `N1`/`N2`).
        match halstead_op_type(kind) {
            HalsteadType::Operator => {
                let kind_label: &'static str = kind.into();
                self.current().halstead.observe_operator(HalsteadOperator {
                    kind: SmolStr::new(kind_label),
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
// Halstead classification (legacy `Getter::get_op_type for GoCode`).
// --------------------------------------------------------------------

enum HalsteadType {
    Operator,
    Operand,
    Unknown,
}

fn halstead_op_type(kind: Go) -> HalsteadType {
    match kind {
        // Operators: keywords and control-flow tokens.
        // Note: `Go::Go` is the `go` keyword (goroutine launch), not the
        // language identifier.
        Go::Func
        | Go::Go
        | Go::Defer
        | Go::Return
        | Go::If
        | Go::Else
        | Go::For
        | Go::Range
        | Go::Switch
        | Go::Select
        | Go::Case
        | Go::Default
        | Go::Break
        | Go::Continue
        | Go::Goto
        | Go::Fallthrough
        | Go::Chan
        | Go::Map
        | Go::Struct
        | Go::Interface
        | Go::Type
        | Go::Var
        | Go::Const
        | Go::Package
        | Go::Import
        // Punctuation operators.
        | Go::DOT
        | Go::COMMA
        | Go::SEMI
        | Go::COLON
        | Go::COLONEQ
        | Go::EQ
        | Go::PLUSEQ
        | Go::DASHEQ
        | Go::STAREQ
        | Go::SLASHEQ
        | Go::PERCENTEQ
        | Go::AMPEQ
        | Go::PIPEEQ
        | Go::CARETEQ
        | Go::LTLTEQ
        | Go::GTGTEQ
        | Go::AMPCARETEQ
        // Arithmetic / logic operators.
        | Go::PLUS
        | Go::DASH
        | Go::STAR
        | Go::SLASH
        | Go::PERCENT
        | Go::AMP
        | Go::PIPE
        | Go::CARET
        | Go::LTLT
        | Go::GTGT
        | Go::AMPAMP
        | Go::PIPEPIPE
        | Go::AMPCARET
        | Go::PLUSPLUS
        | Go::DASHDASH
        | Go::LTDASH
        | Go::TILDE
        | Go::EQEQ
        | Go::BANGEQ
        | Go::LT
        | Go::LTEQ
        | Go::GT
        | Go::GTEQ
        | Go::BANG
        | Go::LPAREN
        | Go::LBRACK
        | Go::LBRACE
        | Go::DOTDOTDOT => HalsteadType::Operator,

        // Operands: identifiers, type identifiers, and literals.
        Go::Identifier
        | Go::Identifier2
        | Go::Identifier3
        | Go::BlankIdentifier
        | Go::FieldIdentifier
        | Go::LabelName
        | Go::PackageIdentifier
        | Go::TypeIdentifier
        | Go::IntLiteral
        | Go::FloatLiteral
        | Go::ImaginaryLiteral
        | Go::RuneLiteral
        | Go::RawStringLiteral
        | Go::InterpretedStringLiteral
        | Go::True
        | Go::False
        | Go::Nil
        | Go::Iota => HalsteadType::Operand,

        _ => HalsteadType::Unknown,
    }
}

// --------------------------------------------------------------------
// ABC helpers — direct ports of legacy `go_*` helper fns.
// --------------------------------------------------------------------

fn go_expression_list_len(node: &Node<'_>) -> u32 {
    iter_children(node)
        .filter(|child| !matches!(Go::from(child.kind_id()), Go::COMMA | Go::Comment))
        .count() as u32
}

fn go_assignment_target_count(node: &Node<'_>) -> u32 {
    node.child_by_field_name("left")
        .map(|child| go_expression_list_len(&child))
        .unwrap_or(1)
}

fn go_spec_name_count(node: &Node<'_>) -> u32 {
    let mut count: u32 = 0;
    for child in iter_children(node) {
        match Go::from(child.kind_id()) {
            Go::EQ => break,
            Go::Identifier | Go::Identifier2 | Go::Identifier3 | Go::BlankIdentifier => {
                count = count.saturating_add(1)
            }
            _ => {}
        }
    }
    count.max(1)
}

// --------------------------------------------------------------------
// NArgs helper — direct port of legacy `compute_go_args`.
// --------------------------------------------------------------------

fn count_go_args(node: &Node<'_>) -> u32 {
    let Some(params) = node.child_by_field_name("parameters") else {
        return 0;
    };
    let mut total: u32 = 0;
    for child in iter_children(&params) {
        match Go::from(child.kind_id()) {
            Go::ParameterDeclaration | Go::VariadicParameterDeclaration => {
                let mut names: u32 = 0;
                for inner in iter_children(&child) {
                    if matches!(
                        Go::from(inner.kind_id()),
                        Go::Identifier | Go::Identifier2 | Go::Identifier3 | Go::BlankIdentifier
                    ) {
                        names = names.saturating_add(1);
                    }
                }
                total = total.saturating_add(names.max(1));
            }
            _ => {}
        }
    }
    total
}

// --------------------------------------------------------------------
// Tree-sitter helpers
// --------------------------------------------------------------------

fn parent_kind(node: &Node<'_>) -> Option<Go> {
    node.parent().map(|p| Go::from(p.kind_id()))
}

fn is_else_if(node: &Node<'_>) -> bool {
    if Go::from(node.kind_id()) != Go::IfStatement {
        return false;
    }
    parent_kind(node) == Some(Go::IfStatement)
}

fn has_child_kind(node: &Node<'_>, kind: Go) -> bool {
    iter_children(node).any(|c| Go::from(c.kind_id()) == kind)
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
