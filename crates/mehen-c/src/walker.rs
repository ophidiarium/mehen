//! Tree-sitter-c walker producing per-space metric output that matches
//! the pre-1.0 `legacy::metrics::*::compute for CCode` arms exactly.
//!
//! The walker drives its own tree-sitter cursor recursion (rather than
//! the generic `mehen-tree-sitter::walker::LanguageRules` plug-in) so it
//! can do parent-aware classification — C's `is_else_if` predicate
//! (an `if_statement` whose direct parent is the `else_clause`) and the
//! `function_definition > function_declarator > parameter_list` walk
//! for NArgs both inspect ancestor / descendant chains the generic
//! plug-in cannot express.
//!
//! Metric coverage:
//! - **Cyclomatic** (legacy `cyclomatic.rs:112-135`): one decision per
//!   `if_statement | case_statement | for_statement | while_statement |
//!   do_statement | conditional_expression | && | ||`. `switch` itself
//!   is not a decision (`case` arms are); `default` is fallthrough.
//! - **Cognitive** (legacy `cognitive.rs:234-294`):
//!   * Increase nesting on `if_statement` (skipping the inner `if` of an
//!     `else if` whose parent is `else_clause`), `for_statement |
//!     while_statement | do_statement | switch_statement |
//!     conditional_expression`. `function_definition` /
//!     `function_definition2` reset nesting and bump function depth.
//!   * `else_clause`: flat `+1` plus `boolean_seq.reset()`.
//!   * `expression_statement | expression_statement2 | return_statement
//!     | declaration`: `boolean_seq.reset()`.
//!   * `binary_expression | binary_expression2`: drive the
//!     `BoolSequence` collapser per `&&`/`||` operator child.
//!   * No closures or lambdas in C.
//! - **ABC** (legacy `abc.rs:233-281`):
//!   * Assignments: `assignment_expression`, `init_declarator` with an
//!     `=` direct child, `update_expression`.
//!   * Branches: every `call_expression | call_expression2`.
//!   * Conditions: `if_statement | else_clause | case_statement |
//!     for_statement | while_statement | do_statement |
//!     conditional_expression | == | != | < | <= | > | >= | && | || |
//!     !`.
//! - **NExit** (legacy `exit.rs:117-128`): `return_statement` only.
//!   `break`/`continue`/`goto` are intra-function flow.
//! - **NArgs** (legacy `nargs.rs:230-275`, `compute_c_args`): walk
//!   `child_by_field_name("declarator")` inward through
//!   `function_declarator` chains until a `parameter_list` is found,
//!   filter `parameter_declaration` children, exclude `variadic_parameter`
//!   (`...`), and apply the `(void)` rule (lone parameter whose source
//!   text equals `void` → 0 args).
//! - **NOM** (legacy `nom.rs:180-189`): every `function_definition` /
//!   `function_definition2` opens a function space. C has no closures.
//! - **LOC** (legacy `loc.rs:567-613`): PLOC default arm,
//!   LLOC for the 36-variant statement / preprocessor-container set,
//!   CLOC for `comment` nodes via `observe_comment`.
//! - **Halstead** (legacy `getter.rs::get_op_type for CCode`): operators
//!   for ~70 keyword/punctuation kinds, operands for the 14 identifier-
//!   shaped + literal kinds. Operands dedup by text only (kind =
//!   `"Operand"`); operators dedup by kind. Same convention as
//!   `mehen-go/src/walker.rs`.
//! - **NPA / NPM / WMC** (legacy `npa.rs:202-205`, `npm.rs:203-206`,
//!   `wmc.rs:142-145`): C is excluded from class-aware metrics; all
//!   three are intentionally no-ops here.
//! - **MI**: derived in `mehen_metrics::state::apply_state_to` from
//!   loc/cyclomatic/halstead — no C-specific logic.

use mehen_core::{LineIndex, MetricSpace, SourceSpan, SpaceKind};
use mehen_metrics::{
    HalsteadOperand, HalsteadOperator, MetricTreeBuilder, State, apply_state_to, finalize_state,
    merge_child_into_parent,
};
use smol_str::SmolStr;
use tree_sitter::Node;

use crate::grammar::C;

/// Drive the walker over the parsed C tree and return the populated
/// `MetricSpace`. Mirrors `mehen_go::walker::walk_program`.
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
    /// C has no closures; field kept for parity with the Go/Ruby walkers
    /// so the `effective` arithmetic stays uniform.
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
        let saved_cognitive = self.cognitive;
        let opened = self.maybe_open_space(&node);
        if opened {
            self.on_space_enter();
        }

        self.classify(&node);

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
        match C::from(node.kind_id()) {
            C::FunctionDefinition | C::FunctionDefinition2 => {
                let name = function_name(node, self.source).map(|s| s.to_string());
                let span = node_span(node, self.line_index);
                let mut child = State::new();
                child.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                child.nom.record_function();
                let argc = count_c_args(node, self.source);
                child.nargs.record_function_args(argc);
                self.tree.open(SpaceKind::Function, span, name);
                self.stack.push(child);
                self.kinds.push(SpaceKind::Function);
                true
            }
            _ => false,
        }
    }

    fn on_space_enter(&mut self) {
        let kind = self.kinds.last().expect("kinds stack empty");
        if matches!(kind, SpaceKind::Function) {
            // Legacy `Cognitive for CCode`'s `FunctionDefinition |
            // FunctionDefinition2` arm: reset nesting; bump function
            // depth when nested. C nested-function syntax is GCC-only
            // and rare, but the depth bump is preserved for parity.
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
    }

    fn close_space(&mut self) {
        let closed_kind = self.kinds.pop().expect("kinds stack underflow");
        let mut state = self.stack.pop().expect("walker stack underflow");
        if matches!(closed_kind, SpaceKind::Function) {
            // Mirrors the legacy `wmc::Stats` close path. WMC is
            // class-aware; C has no classes, so this value is never
            // published — but the bookkeeping is kept so per-space
            // walker shape stays uniform with Go/Kotlin.
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
        let kind = C::from(node.kind_id());

        // Cyclomatic — legacy `Cyclomatic for CCode`.
        if matches!(
            kind,
            C::IfStatement
                | C::CaseStatement
                | C::ForStatement
                | C::WhileStatement
                | C::DoStatement
                | C::ConditionalExpression
                | C::AMPAMP
                | C::PIPEPIPE
        ) {
            self.current().cyclomatic.record_decision();
        }

        self.classify_cognitive(node, kind);
        self.classify_abc(node, kind);

        // NExit — legacy `Exit for CCode`. Only `return_statement`;
        // break/continue/goto are intra-function flow.
        if matches!(kind, C::ReturnStatement) {
            self.current().nexit.record_exit();
        }

        self.classify_loc(node, kind);
        self.classify_halstead(node, kind);
    }

    fn classify_cognitive(&mut self, node: &Node<'_>, kind: C) {
        match kind {
            // Outer `if`. `is_else_if` checks parent == ElseClause: when
            // true, the structural +1 is paid by the surrounding `else
            // clause` arm and only the boolean-seq reset stays here
            // (defense-in-depth duplicate of the ElseClause reset).
            C::IfStatement if !is_else_if(node) => {
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                self.cognitive.nesting = self.cognitive.nesting.saturating_add(1);
            }
            C::IfStatement => {
                self.current().cognitive.boolean_seq.reset();
            }
            C::ForStatement
            | C::WhileStatement
            | C::DoStatement
            | C::SwitchStatement
            | C::ConditionalExpression => {
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                self.cognitive.nesting = self.cognitive.nesting.saturating_add(1);
            }
            C::ElseClause => {
                self.current().cognitive.increment_by_one();
                self.current().cognitive.boolean_seq.reset();
            }
            C::ExpressionStatement
            | C::ExpressionStatement2
            | C::ReturnStatement
            | C::Declaration => {
                self.current().cognitive.boolean_seq.reset();
            }
            C::BinaryExpression | C::BinaryExpression2 => {
                // Legacy `compute_booleans::<C>(node, &AMPAMP, &PIPEPIPE)`:
                // walk the children and feed each `&&`/`||` operator
                // into the sequence collapser.
                for child in iter_children(node) {
                    match C::from(child.kind_id()) {
                        C::AMPAMP => self.current().cognitive.observe_boolean("&&"),
                        C::PIPEPIPE => self.current().cognitive.observe_boolean("||"),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn classify_abc(&mut self, node: &Node<'_>, kind: C) {
        match kind {
            C::AssignmentExpression | C::UpdateExpression => {
                self.current().abc.record_assignment();
            }
            C::InitDeclarator if has_child_kind(node, C::EQ) => {
                self.current().abc.record_assignment();
            }
            C::CallExpression | C::CallExpression2 => {
                self.current().abc.record_branch();
            }
            C::IfStatement
            | C::ElseClause
            | C::CaseStatement
            | C::ForStatement
            | C::WhileStatement
            | C::DoStatement
            | C::ConditionalExpression
            | C::EQEQ
            | C::BANGEQ
            | C::LT
            | C::LTEQ
            | C::GT
            | C::GTEQ
            | C::AMPAMP
            | C::PIPEPIPE
            | C::BANG => {
                self.current().abc.record_condition();
            }
            _ => {}
        }
    }

    fn classify_loc(&mut self, node: &Node<'_>, kind: C) {
        match kind {
            // Containers and string internals must not contribute their
            // own physical line. Mirrors the legacy `Loc for CCode`'s
            // explicit no-op arm.
            C::TranslationUnit
            | C::StringLiteral
            | C::ConcatenatedString
            | C::CharLiteral
            | C::CompoundStatement
            | C::StringContent
            | C::EscapeSequence => {}
            C::Comment => {
                let start = node.start_position().row as u32;
                let end = node.end_position().row as u32;
                self.current().loc.observe_comment(start, end);
            }
            // LLOC kind set: 36 statement-shaped + preprocessor-container
            // variants (legacy `loc.rs:583-616`). Each occurrence
            // contributes one logical line.
            C::Declaration
            | C::TypeDefinition
            | C::ExpressionStatement
            | C::ExpressionStatement2
            | C::IfStatement
            | C::SwitchStatement
            | C::CaseStatement
            | C::WhileStatement
            | C::DoStatement
            | C::ForStatement
            | C::ReturnStatement
            | C::BreakStatement
            | C::ContinueStatement
            | C::GotoStatement
            | C::LabeledStatement
            | C::SehTryStatement
            | C::SehLeaveStatement
            | C::FunctionDefinition
            | C::FunctionDefinition2
            | C::PreprocInclude
            | C::PreprocDef
            | C::PreprocFunctionDef
            | C::PreprocCall
            | C::PreprocIf
            | C::PreprocIf2
            | C::PreprocIf3
            | C::PreprocIf4
            | C::PreprocIfdef
            | C::PreprocIfdef2
            | C::PreprocIfdef3
            | C::PreprocIfdef4
            | C::PreprocElse
            | C::PreprocElse2
            | C::PreprocElse3
            | C::PreprocElse4
            | C::PreprocElif
            | C::PreprocElif2
            | C::PreprocElif3
            | C::PreprocElif4
            | C::PreprocElifdef
            | C::PreprocElifdef2
            | C::PreprocElifdef3
            | C::PreprocElifdef4 => {
                self.current().loc.observe_lloc();
            }
            _ => {
                let start = node.start_position().row as u32;
                self.current().loc.observe_code_line(start);
            }
        }
    }

    fn classify_halstead(&mut self, node: &Node<'_>, kind: C) {
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
// Halstead classification (legacy `Getter::get_op_type for CCode`).
// --------------------------------------------------------------------

enum HalsteadType {
    Operator,
    Operand,
    Unknown,
}

fn halstead_op_type(kind: C) -> HalsteadType {
    match kind {
        // Keywords and control flow.
        C::If
        | C::Else
        | C::Switch
        | C::Case
        | C::Default
        | C::While
        | C::Do
        | C::For
        | C::Return
        | C::Break
        | C::Continue
        | C::Goto
        | C::Sizeof
        | C::Alignof
        | C::Alignof2
        | C::Alignof3
        | C::Alignof4
        | C::Alignof5
        | C::Offsetof
        | C::Typedef
        | C::Extern
        | C::Static
        | C::Auto
        | C::Register
        | C::Inline
        | C::Inline2
        | C::Inline3
        | C::Forceinline
        | C::ThreadLocal
        | C::Thread
        | C::Const
        | C::Constexpr
        | C::Volatile
        | C::Volatile2
        | C::Restrict
        | C::Restrict2
        | C::Atomic
        | C::Noreturn
        | C::Noreturn2
        | C::Nonnull
        | C::Alignas
        | C::Alignas2
        | C::Signed
        | C::Unsigned
        | C::Long
        | C::Short
        | C::Enum
        | C::Struct
        | C::Union
        // Punctuation.
        | C::LPAREN
        | C::LPAREN2
        | C::RPAREN
        | C::LBRACE
        | C::RBRACE
        | C::LBRACK
        | C::RBRACK
        | C::COMMA
        | C::SEMI
        | C::COLON
        | C::QMARK
        | C::DOT
        | C::DASHGT
        // Arithmetic / bitwise / logical / comparison operators.
        | C::PLUS
        | C::DASH
        | C::STAR
        | C::SLASH
        | C::PERCENT
        | C::AMP
        | C::PIPE
        | C::CARET
        | C::TILDE
        | C::BANG
        | C::LTLT
        | C::GTGT
        | C::AMPAMP
        | C::PIPEPIPE
        | C::EQ
        | C::EQEQ
        | C::BANGEQ
        | C::LT
        | C::LTEQ
        | C::GT
        | C::GTEQ
        | C::PLUSEQ
        | C::DASHEQ
        | C::STAREQ
        | C::SLASHEQ
        | C::PERCENTEQ
        | C::AMPEQ
        | C::PIPEEQ
        | C::CARETEQ
        | C::LTLTEQ
        | C::GTGTEQ
        | C::PLUSPLUS
        | C::DASHDASH
        // Preprocessor directives count as operators.
        | C::HASHinclude
        | C::HASHdefine
        | C::HASHif
        | C::HASHifdef
        | C::HASHifndef
        | C::HASHelse
        | C::HASHelif
        | C::HASHelifdef
        | C::HASHelifndef
        | C::HASHendif => HalsteadType::Operator,

        // Operands: identifiers, type identifiers, and literals.
        C::Identifier
        | C::FieldIdentifier
        | C::TypeIdentifier
        | C::StatementIdentifier
        | C::PrimitiveType
        | C::NumberLiteral
        | C::CharLiteral
        | C::StringLiteral
        | C::ConcatenatedString
        | C::True
        | C::False
        | C::NULL
        | C::Nullptr
        | C::SystemLibString => HalsteadType::Operand,

        _ => HalsteadType::Unknown,
    }
}

// --------------------------------------------------------------------
// Function-name and NArgs helpers — direct ports of legacy
// `getter.rs::get_func_space_name for CCode` and `compute_c_args`.
// --------------------------------------------------------------------

/// Walk `node.declarator` inward through `function_declarator` /
/// `pointer_declarator` / `parenthesized_declarator` chains until an
/// identifier is found. Mirrors legacy `getter.rs::get_func_space_name`.
fn function_name<'src>(node: &Node<'_>, source: &'src [u8]) -> Option<&'src str> {
    let mut cur = node.child_by_field_name("declarator");
    while let Some(current) = cur {
        match C::from(current.kind_id()) {
            C::Identifier | C::FieldIdentifier | C::TypeIdentifier => {
                let bytes = &source[current.start_byte()..current.end_byte()];
                return core::str::from_utf8(bytes).ok();
            }
            _ => {
                cur = current.child_by_field_name("declarator");
            }
        }
    }
    None
}

#[inline(always)]
fn is_c_function_declarator(kind: u16) -> bool {
    matches!(
        C::from(kind),
        C::FunctionDeclarator
            | C::FunctionDeclarator2
            | C::FunctionDeclarator3
            | C::FunctionDeclarator4
            | C::FunctionDeclarator5
    )
}

#[inline(always)]
fn is_c_parameter_list(kind: u16) -> bool {
    matches!(C::from(kind), C::ParameterList | C::ParameterList2)
}

/// Walk the `declarator` field inward until the innermost
/// `function_declarator` is found; that node's direct `parameter_list`
/// child holds the parameters. Mirrors legacy `compute_c_args`.
fn count_c_args(node: &Node<'_>, source: &[u8]) -> u32 {
    let mut cur = node.child_by_field_name("declarator");
    while let Some(current) = cur {
        if is_c_function_declarator(current.kind_id()) {
            let mut cursor = current.walk();
            let Some(param_list) = current
                .children(&mut cursor)
                .find(|c| is_c_parameter_list(c.kind_id()))
            else {
                return 0;
            };
            let mut list_cursor = param_list.walk();
            let params: Vec<_> = param_list
                .children(&mut list_cursor)
                .filter(|p| C::from(p.kind_id()) == C::ParameterDeclaration)
                .collect();
            // `(void)` is C's spelling for "no parameters" and must not
            // be counted. Detect it precisely by checking that the
            // sole parameter's text literally matches `void`.
            // `variadic_parameter` (`...`) is filtered out above.
            let is_void_only = params.len() == 1
                && source
                    .get(params[0].start_byte()..params[0].end_byte())
                    .is_some_and(|bytes| bytes == b"void");
            return if is_void_only { 0 } else { params.len() as u32 };
        }
        cur = current.child_by_field_name("declarator");
    }
    0
}

// --------------------------------------------------------------------
// Tree-sitter helpers
// --------------------------------------------------------------------

fn parent_kind(node: &Node<'_>) -> Option<C> {
    node.parent().map(|p| C::from(p.kind_id()))
}

/// `is_else_if`: an `if_statement` whose direct parent is the
/// `else_clause` wrapper. tree-sitter-c parses `else if (...)` as
/// `if_statement { else_clause { if_statement } }`, so the *inner*
/// if matches this predicate. Mirrors legacy
/// `checker.rs::is_else_if for CCode`.
fn is_else_if(node: &Node<'_>) -> bool {
    if C::from(node.kind_id()) != C::IfStatement {
        return false;
    }
    parent_kind(node) == Some(C::ElseClause)
}

fn has_child_kind(node: &Node<'_>, kind: C) -> bool {
    iter_children(node).any(|c| C::from(c.kind_id()) == kind)
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
