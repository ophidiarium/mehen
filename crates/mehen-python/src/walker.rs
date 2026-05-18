//! Ruff AST + token-stream walker that produces a populated `MetricSpace`.
//!
//! The walker follows the same per-space `State` accumulator pattern used
//! by `mehen-typescript` (see `crates/mehen-typescript/src/walker.rs`):
//! one `State` for the unit, one for every opened function / closure /
//! class space, finalize on close, fold child stats into parent. The
//! publishing helpers (`apply_state_to`, `finalize_state`,
//! `merge_child_into_parent`) live in `mehen-metrics::state` per the
//! rewrite plan §4.3.
//!
//! Python-specific design decisions are documented in
//! `docs/python-ruff-spec.md`. The short version:
//!
//! - **Type annotations and default-value expressions**: in Python, type
//!   annotations are runtime-accessible objects (Pydantic, dataclasses,
//!   `typing.get_type_hints`, etc.). Tokens inside an annotation
//!   subtree are treated like ordinary tokens for Halstead — they ARE
//!   operands and operators in the running program. This is a deliberate
//!   semantic difference from `mehen-typescript`, where TS-only type
//!   metadata is excluded because TS types are erased at runtime.
//!
//! - **Docstrings**: a string literal that is the *first statement of a
//!   module / class / function body* is a docstring per PEP 257 — a
//!   structural language feature, not arbitrary code. We do not emit
//!   Halstead operators/operands for docstring tokens, but the LOC
//!   accounting still counts those lines as `cloc` (comment-like) per
//!   the legacy convention.
//!
//! - **`match`/`case`**: every `case` clause is a cyclomatic decision
//!   point and a cognitive nesting bump. The Python `match` statement
//!   is a structural pattern match, so each case is a real branch.
//!
//! - **Exception groups (`try*`/`except*`)**: an `except*` handler still
//!   counts the same as a regular `except` — both add a decision and a
//!   nesting level; the underlying `is_star: bool` flag on `StmtTry` is
//!   noted for evidence but does not change the metric output.

use mehen_core::{LineIndex, MetricSpace, SourceSpan, SpaceKind};
use mehen_metrics::{
    ContainerKind, HalsteadOperand, HalsteadOperator, MetricTreeBuilder, State, apply_state_to,
    finalize_state, merge_child_into_parent,
};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{
    self as ast, BoolOp, ElifElseClause, ExceptHandler, Expr, MatchCase, ModModule, Parameters,
    Stmt, UnaryOp,
};
use ruff_python_parser::Parsed;
use ruff_text_size::{Ranged, TextRange};
use smol_str::SmolStr;

/// Drive the walker over a parsed Python module. Crate-internal entry
/// point — only `mehen_python::PythonAnalyzer::analyze` calls this; not
/// part of any cross-crate API.
pub(crate) fn walk_module(
    parsed: &Parsed<ModModule>,
    source: &str,
    line_index: &LineIndex,
) -> MetricSpace {
    let module = parsed.syntax();
    let unit_span = SourceSpan {
        start_byte: module.range.start().to_u32(),
        end_byte: module.range.end().to_u32(),
        start_line: line_index.line_at(module.range.start().to_u32()),
        end_line: line_index.line_at(module.range.end().to_u32()),
    };

    let mut visitor = Visitor::new(source, line_index, unit_span);
    visitor.record_module_docstring(&module.body);
    for stmt in &module.body {
        visitor.visit_stmt(stmt);
    }

    visitor.emit_halstead_from_tokens(parsed.tokens());

    visitor.finish()
}

struct Visitor<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
    tree: MetricTreeBuilder,
    /// Per-space accumulator stack — index 0 is the unit.
    stack: Vec<State>,
    /// Parallel to `stack`: the SpaceKind of each frame so we can tell
    /// "what's the enclosing class-like" without re-walking.
    kinds: Vec<SpaceKind>,
    /// Cognitive context inherited down the recursion. Mirrors the
    /// legacy `(nesting, depth, lambda)` triple from
    /// `mehen-engine/src/legacy/metrics/cognitive.rs::python` —
    /// Python increments `lambda` only inside an `ExprLambda` and the
    /// boolean-sequence reset rules apply to expression statements.
    cognitive: CognitiveContext,
    /// Byte ranges of nodes whose tokens should NOT contribute to the
    /// Halstead token sweep. Currently this is only docstring spans
    /// (per PEP 257). Type annotation spans are NOT added here because
    /// Python types are runtime-accessible — see crate docs.
    docstring_ranges: Vec<TextRange>,
}

#[derive(Clone, Copy, Debug, Default)]
struct CognitiveContext {
    nesting: u32,
    depth: u32,
    lambda: u32,
    /// Depth of nested `BoolOp` expressions. Used by the BoolOp handler
    /// to detect the *outermost* boolean operator inside a statement —
    /// only that one gets the legacy "lambda ancestor" bonus
    /// (mehen-engine cognitive.rs:281 `count_specific_ancestors` to
    /// detect outermost-up-to-Lambda-boundary).
    bool_op_depth: u32,
}

impl<'a> Visitor<'a> {
    fn new(source: &'a str, line_index: &'a LineIndex, unit_span: SourceSpan) -> Self {
        let mut state = State::new();
        state.loc.set_span(
            unit_span.start_line.saturating_sub(1),
            unit_span.end_line.saturating_sub(1),
            true,
        );
        Self {
            source,
            line_index,
            tree: MetricTreeBuilder::new(unit_span),
            stack: vec![state],
            kinds: vec![SpaceKind::Unit],
            cognitive: CognitiveContext::default(),
            docstring_ranges: Vec::new(),
        }
    }

    fn current(&mut self) -> &mut State {
        self.stack.last_mut().expect("walker stack empty")
    }

    fn parent_kind(&self) -> SpaceKind {
        self.kinds.last().cloned().unwrap_or(SpaceKind::Unit)
    }

    fn finish(mut self) -> MetricSpace {
        let mut unit_state = self.stack.pop().expect("walker stack underflow");
        finalize_state(&mut unit_state);
        apply_state_to(unit_state, self.tree.metrics_mut());
        self.tree.finish()
    }

    fn record_module_docstring(&mut self, body: &[Stmt]) {
        if let Some(span) = leading_docstring_range(body) {
            self.docstring_ranges.push(span);
        }
    }

    fn open_space(&mut self, kind: SpaceKind, range: TextRange, name: Option<String>) {
        let mut child = State::new();
        let start_row = self
            .line_index
            .line_at(range.start().to_u32())
            .saturating_sub(1);
        let end_row = self
            .line_index
            .line_at(range.end().to_u32())
            .saturating_sub(1);
        child.loc.set_span(start_row, end_row, false);

        match kind {
            SpaceKind::Function => {
                child.nom.record_function();
            }
            SpaceKind::Closure => {
                child.nom.record_closure();
            }
            SpaceKind::Class | SpaceKind::Impl => {
                child.npa.record_class_like();
                child.npm.record_class_like();
                child.wmc.record_class_like();
            }
            SpaceKind::Interface | SpaceKind::Trait => {
                child.npa.record_class_like();
                child.npm.record_class_like();
            }
            _ => {}
        }
        let span = text_range_to_source_span(range, self.line_index);
        self.tree.open(kind.clone(), span, name);
        self.stack.push(child);
        self.kinds.push(kind);
    }

    fn close_space(&mut self) {
        let closed_kind = self.kinds.pop().expect("kinds underflow");
        let mut state = self.stack.pop().expect("stack underflow");
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

    fn enter_function(
        &mut self,
        name: &str,
        parameters: &Parameters,
        decorator_list: &[ast::Decorator],
        body: &[Stmt],
        range: TextRange,
    ) {
        // Python decorators: each `@decorator` is in itself an extra
        // expression that runs at definition time. The legacy walker
        // records them as part of the enclosing space's metric stream
        // (decorators land in the *enclosing* class/unit). We follow
        // that by visiting decorators *before* opening the function
        // space.
        for decorator in decorator_list {
            self.visit_expr(&decorator.expression);
        }

        self.open_space(SpaceKind::Function, range, Some(name.to_string()));
        let argc = parameters.len() as u32;
        self.current().nargs.record_function_args(argc);

        // Cognitive — function entry resets nesting/lambda and bumps
        // depth when nested inside another function.
        let mut ctx = self.cognitive;
        let nested = self
            .kinds
            .iter()
            .rev()
            .skip(1)
            .any(|k| matches!(k, SpaceKind::Function));
        ctx.nesting = 0;
        ctx.lambda = 0;
        if nested {
            ctx.depth = ctx.depth.saturating_add(1);
        }
        let saved = self.cognitive;
        self.cognitive = ctx;

        // Walk the parameter defaults / annotations first so they
        // contribute Halstead/ABC/etc. against the function's own state
        // (Python evaluates these at definition time, but they belong
        // to the function's signature — keep them in the func space).
        self.visit_parameters(parameters);

        // Capture the leading docstring so it does not contribute to
        // Halstead via the token sweep.
        if let Some(span) = leading_docstring_range(body) {
            self.docstring_ranges.push(span);
        }

        for stmt in body {
            self.visit_stmt(stmt);
        }

        self.cognitive = saved;
        self.close_space();
    }

    fn enter_class(
        &mut self,
        name: &str,
        decorator_list: &[ast::Decorator],
        arguments: Option<&ast::Arguments>,
        body: &[Stmt],
        range: TextRange,
    ) {
        for decorator in decorator_list {
            self.visit_expr(&decorator.expression);
        }
        // Class arguments (base classes, metaclass=...) live in the
        // enclosing scope, not in the class body — they execute at
        // definition time.
        if let Some(args) = arguments {
            self.visit_arguments(args);
        }

        self.open_space(SpaceKind::Class, range, Some(name.to_string()));

        if let Some(span) = leading_docstring_range(body) {
            self.docstring_ranges.push(span);
        }

        for stmt in body {
            // Class-body assignments — `name: T = value` (StmtAnnAssign)
            // and `name = value` (StmtAssign with bare-identifier
            // target) — count as class attributes (NPA). Method-style
            // `def f(self):` inside a class body counts as a method
            // (NPM). We classify here because the AnnAssign / Assign
            // context (top-level of class body) matters.
            self.classify_class_body_member(stmt);
            self.visit_stmt(stmt);
        }

        self.close_space();
    }

    fn classify_class_body_member(&mut self, stmt: &Stmt) {
        let parent = self.parent_kind();
        if !matches!(
            parent,
            SpaceKind::Class | SpaceKind::Impl | SpaceKind::Interface | SpaceKind::Trait
        ) {
            return;
        }
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => {
                if let Expr::Name(name) = target.as_ref() {
                    let is_public = python_attribute_is_public(name.id.as_str());
                    self.current()
                        .npa
                        .record_attribute(ContainerKind::Class, is_public);
                }
            }
            Stmt::Assign(ast::StmtAssign { targets, .. }) => {
                for tgt in targets {
                    if let Expr::Name(name) = tgt {
                        let is_public = python_attribute_is_public(name.id.as_str());
                        self.current()
                            .npa
                            .record_attribute(ContainerKind::Class, is_public);
                    }
                }
            }
            Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => {
                let is_public = python_method_is_public(name.id.as_str());
                self.current()
                    .npm
                    .record_method(ContainerKind::Class, is_public);
            }
            _ => {}
        }
    }

    fn enter_lambda(&mut self, lam: &ast::ExprLambda) {
        self.open_space(SpaceKind::Closure, lam.range, None);
        let argc = lam
            .parameters
            .as_deref()
            .map(|p| p.len() as u32)
            .unwrap_or(0);
        self.current().nargs.record_closure_args(argc);

        let mut ctx = self.cognitive;
        ctx.lambda = ctx.lambda.saturating_add(1);
        let saved = self.cognitive;
        self.cognitive = ctx;

        if let Some(params) = lam.parameters.as_deref() {
            self.visit_parameters(params);
        }
        self.visit_expr(&lam.body);

        self.cognitive = saved;
        self.close_space();
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        // LOC accounting — every statement bumps lloc, and its starting
        // line is a code line.
        self.observe_loc_for_stmt(stmt);

        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef {
                name,
                parameters,
                decorator_list,
                body,
                range,
                ..
            }) => {
                self.enter_function(name.id.as_str(), parameters, decorator_list, body, *range);
            }
            Stmt::ClassDef(ast::StmtClassDef {
                name,
                decorator_list,
                arguments,
                body,
                range,
                ..
            }) => {
                self.enter_class(
                    name.id.as_str(),
                    decorator_list,
                    arguments.as_deref(),
                    body,
                    *range,
                );
            }
            Stmt::If(ast::StmtIf {
                test,
                body,
                elif_else_clauses,
                ..
            }) => {
                self.current().cyclomatic.record_decision();
                self.current().abc.record_condition();
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                // Match legacy `increase_nesting` (mehen-engine cognitive.rs:239):
                // a new control-flow scope resets the boolean sequence so two
                // sibling `if a and b: ...` blocks each contribute +1 for their
                // own `and`, instead of collapsing into a single same-op run.
                self.current().cognitive.boolean_seq.reset();
                self.cognitive.nesting += 1;
                self.visit_expr(test);
                for s in body {
                    self.visit_stmt(s);
                }
                // Elif/else clauses inherit the nesting bump from their
                // owning `if` (legacy walks them as children of `if_statement`,
                // so they see the parent's nesting via the nesting map).
                // Keep `cognitive.nesting` raised while walking them.
                for clause in elif_else_clauses {
                    self.visit_elif_else_clause(clause);
                }
                self.cognitive.nesting -= 1;
            }
            Stmt::For(ast::StmtFor {
                target,
                iter,
                body,
                orelse,
                ..
            }) => {
                self.current().cyclomatic.record_decision();
                self.current().abc.record_condition();
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                // Match legacy `increase_nesting` (cognitive.rs:239).
                self.current().cognitive.boolean_seq.reset();
                self.cognitive.nesting += 1;
                self.visit_expr(target);
                self.visit_expr(iter);
                for s in body {
                    self.visit_stmt(s);
                }
                self.cognitive.nesting -= 1;
                if !orelse.is_empty() {
                    // `for ... else` — the else-branch runs only if
                    // the loop completed without `break`. Legacy treats
                    // the else-clause as +1 cyclomatic (a real branch
                    // that depends on `break` not firing).
                    self.current().cyclomatic.record_decision();
                    self.current().cognitive.increment_by_one();
                    self.current().abc.record_condition();
                    for s in orelse {
                        self.visit_stmt(s);
                    }
                }
            }
            Stmt::While(ast::StmtWhile {
                test, body, orelse, ..
            }) => {
                self.current().cyclomatic.record_decision();
                self.current().abc.record_condition();
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                // Match legacy `increase_nesting` (cognitive.rs:239).
                self.current().cognitive.boolean_seq.reset();
                self.cognitive.nesting += 1;
                self.visit_expr(test);
                for s in body {
                    self.visit_stmt(s);
                }
                self.cognitive.nesting -= 1;
                if !orelse.is_empty() {
                    self.current().cyclomatic.record_decision();
                    self.current().cognitive.increment_by_one();
                    self.current().abc.record_condition();
                    for s in orelse {
                        self.visit_stmt(s);
                    }
                }
            }
            Stmt::Try(ast::StmtTry {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            }) => {
                // `try` itself is a +1 nesting bump (cognitive only —
                // legacy does not count the bare `try` for cyclomatic
                // because the decision is in the handler). The `try`
                // raises the nesting level for its body AND its
                // except / else / finally branches (siblings in the
                // Ruff AST, but children of the `try_statement` in
                // tree-sitter — both should see the same nesting).
                self.current().abc.record_condition();
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                // Match legacy `increase_nesting` (cognitive.rs:239).
                self.current().cognitive.boolean_seq.reset();
                self.cognitive.nesting += 1;
                for s in body {
                    self.visit_stmt(s);
                }
                for handler in handlers {
                    self.visit_except_handler(handler);
                }
                if !orelse.is_empty() {
                    self.current().cognitive.increment_by_one();
                    for s in orelse {
                        self.visit_stmt(s);
                    }
                }
                if !finalbody.is_empty() {
                    self.current().cognitive.increment_by_one();
                    for s in finalbody {
                        self.visit_stmt(s);
                    }
                }
                self.cognitive.nesting -= 1;
            }
            Stmt::Match(ast::StmtMatch { subject, cases, .. }) => {
                // `match` itself does not increment cyclomatic — each
                // `case` does. ABC records `match` as a condition once
                // (the match itself is a structural branch).
                self.current().abc.record_condition();
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                // Match legacy `increase_nesting` (cognitive.rs:239).
                self.current().cognitive.boolean_seq.reset();
                self.cognitive.nesting += 1;
                self.visit_expr(subject);
                for case in cases {
                    self.visit_match_case(case);
                }
                self.cognitive.nesting -= 1;
            }
            Stmt::With(ast::StmtWith { items, body, .. }) => {
                // `with` is not a cyclomatic decision (no branching),
                // but it does add cognitive nesting (a structural
                // scope) and one ABC condition equivalent.
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                // Match legacy `increase_nesting` (cognitive.rs:239).
                self.current().cognitive.boolean_seq.reset();
                self.cognitive.nesting += 1;
                for item in items {
                    self.visit_expr(&item.context_expr);
                    if let Some(opt_vars) = &item.optional_vars {
                        self.visit_expr(opt_vars);
                    }
                }
                for s in body {
                    self.visit_stmt(s);
                }
                self.cognitive.nesting -= 1;
            }
            Stmt::Return(ast::StmtReturn { value, .. }) => {
                self.current().nexit.record_exit();
                if let Some(v) = value {
                    self.visit_expr(v);
                }
            }
            Stmt::Raise(ast::StmtRaise { exc, cause, .. }) => {
                self.current().nexit.record_exit();
                if let Some(e) = exc {
                    self.visit_expr(e);
                }
                if let Some(c) = cause {
                    self.visit_expr(c);
                }
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                self.current().abc.record_assignment();
                for t in targets {
                    self.visit_expr(t);
                }
                self.visit_expr(value);
            }
            Stmt::AugAssign(ast::StmtAugAssign { target, value, .. }) => {
                self.current().abc.record_assignment();
                self.visit_expr(target);
                self.visit_expr(value);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign {
                target,
                annotation,
                value,
                ..
            }) => {
                if value.is_some() {
                    self.current().abc.record_assignment();
                }
                self.visit_expr(target);
                self.visit_expr(annotation);
                if let Some(v) = value {
                    self.visit_expr(v);
                }
            }
            Stmt::Expr(ast::StmtExpr { value, .. }) => {
                // ExpressionStatement resets the boolean sequence (for
                // cognitive complexity boolean-chain folding).
                self.current().cognitive.boolean_seq.reset();
                self.visit_expr(value);
            }
            // Statements that don't recurse into expressions or have
            // no metric implications beyond the LOC bump above.
            Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::Pass(_)
            | Stmt::Global(_)
            | Stmt::Nonlocal(_)
            | Stmt::Import(_)
            | Stmt::ImportFrom(_) => {}
            Stmt::Delete(ast::StmtDelete { targets, .. }) => {
                for t in targets {
                    self.visit_expr(t);
                }
            }
            Stmt::Assert(ast::StmtAssert { test, msg, .. }) => {
                self.visit_expr(test);
                if let Some(m) = msg {
                    self.visit_expr(m);
                }
            }
            Stmt::TypeAlias(ast::StmtTypeAlias { name, value, .. }) => {
                // `type X = Y` (PEP 695 type alias). The target is an
                // assignment — count it once.
                self.current().abc.record_assignment();
                self.visit_expr(name);
                self.visit_expr(value);
            }
            Stmt::IpyEscapeCommand(_) => {
                // Notebook-only IPython escape commands are not
                // reachable for `.py` source. Skip silently.
            }
        }
    }

    fn visit_elif_else_clause(&mut self, clause: &ElifElseClause) {
        // Elif: +1 cyclomatic (the chained condition is a real branch),
        // +1 cognitive (no nesting bump — its cost is paid by the outer
        // `if`), reset the boolean sequence.
        // Else: +1 cognitive only — no cyclomatic increment because the
        // else branch isn't a separate decision (the if already picked
        // a branch).
        if clause.test.is_some() {
            self.current().cyclomatic.record_decision();
            self.current().cognitive.increment_by_one();
            self.current().cognitive.boolean_seq.reset();
            self.current().abc.record_condition();
        } else {
            self.current().cognitive.increment_by_one();
            self.current().abc.record_condition();
        }
        if let Some(test) = &clause.test {
            self.visit_expr(test);
        }
        for s in &clause.body {
            self.visit_stmt(s);
        }
    }

    fn visit_except_handler(&mut self, handler: &ExceptHandler) {
        let ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { type_, body, .. }) =
            handler;
        self.current().cyclomatic.record_decision();
        self.current().abc.record_condition();
        let effective = self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
        self.current().cognitive.increase_nesting(effective);
        // Match legacy `increase_nesting` (cognitive.rs:239).
        self.current().cognitive.boolean_seq.reset();
        self.cognitive.nesting += 1;
        if let Some(t) = type_ {
            self.visit_expr(t);
        }
        for s in body {
            self.visit_stmt(s);
        }
        self.cognitive.nesting -= 1;
    }

    fn visit_match_case(&mut self, case: &MatchCase) {
        self.current().cyclomatic.record_decision();
        self.current().abc.record_condition();
        let effective = self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
        self.current().cognitive.increase_nesting(effective);
        // Match legacy `increase_nesting` (cognitive.rs:239).
        self.current().cognitive.boolean_seq.reset();
        self.cognitive.nesting += 1;
        if let Some(g) = &case.guard {
            self.visit_expr(g);
        }
        for s in &case.body {
            self.visit_stmt(s);
        }
        self.cognitive.nesting -= 1;
    }

    fn visit_parameters(&mut self, params: &Parameters) {
        // Parameter defaults and annotations are runtime-evaluated
        // expressions in Python — visit them so they participate in
        // ABC / cyclomatic / Halstead.
        for p in params.iter_non_variadic_params() {
            if let Some(annotation) = &p.parameter.annotation {
                self.visit_expr(annotation);
            }
            if let Some(default) = &p.default {
                self.visit_expr(default);
            }
        }
        if let Some(va) = &params.vararg
            && let Some(annotation) = &va.annotation
        {
            self.visit_expr(annotation);
        }
        if let Some(kw) = &params.kwarg
            && let Some(annotation) = &kw.annotation
        {
            self.visit_expr(annotation);
        }
    }

    fn visit_arguments(&mut self, args: &ast::Arguments) {
        for arg in &args.args {
            self.visit_expr(arg);
        }
        for kw in &args.keywords {
            self.visit_expr(&kw.value);
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::BoolOp(ast::ExprBoolOp { op, values, .. }) => {
                // Boolean `and` / `or` — each operand beyond the first
                // is one decision point per legacy.
                for _ in 1..values.len() {
                    self.current().cyclomatic.record_decision();
                    self.current().abc.record_condition();
                }
                // Lambda-ancestor bonus (legacy cognitive.rs:281): the
                // *outermost* BoolOp inside a statement adds one structural
                // unit per enclosing lambda. Inside `bar = lambda a:
                // lambda b: b or True or True`, the `or` sequence sits
                // inside two lambdas — legacy adds +2 to the per-space
                // structural before the boolean-sequence collapser.
                let lambda_bonus = if self.cognitive.bool_op_depth == 0 {
                    self.cognitive.lambda
                } else {
                    0
                };
                if lambda_bonus > 0 {
                    self.current().cognitive.record_increment(lambda_bonus);
                }
                let label = match op {
                    BoolOp::And => "and",
                    BoolOp::Or => "or",
                };
                self.current().cognitive.observe_boolean(label);
                self.cognitive.bool_op_depth = self.cognitive.bool_op_depth.saturating_add(1);
                for v in values {
                    self.visit_expr(v);
                }
                self.cognitive.bool_op_depth = self.cognitive.bool_op_depth.saturating_sub(1);
            }
            Expr::Named(ast::ExprNamed { target, value, .. }) => {
                self.current().abc.record_assignment();
                self.visit_expr(target);
                self.visit_expr(value);
            }
            Expr::BinOp(ast::ExprBinOp { left, right, .. }) => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::UnaryOp(ast::ExprUnaryOp { op, operand, .. }) => {
                if matches!(op, UnaryOp::Not) {
                    self.current().cognitive.boolean_seq.not_operator("not");
                }
                self.visit_expr(operand);
            }
            Expr::Lambda(lam) => {
                self.enter_lambda(lam);
            }
            Expr::If(ast::ExprIf {
                test, body, orelse, ..
            }) => {
                // Conditional expression `a if b else c` — one decision.
                self.current().cyclomatic.record_decision();
                self.current().abc.record_condition();
                let effective =
                    self.cognitive.nesting + self.cognitive.depth + self.cognitive.lambda;
                self.current().cognitive.increase_nesting(effective);
                // Match legacy `increase_nesting` (cognitive.rs:239).
                self.current().cognitive.boolean_seq.reset();
                self.cognitive.nesting += 1;
                self.visit_expr(test);
                self.visit_expr(body);
                self.visit_expr(orelse);
                self.cognitive.nesting -= 1;
            }
            Expr::Compare(ast::ExprCompare {
                left,
                ops: _,
                comparators,
                ..
            }) => {
                // Comparison ops (`==`, `<`, ...) — each pair counts as
                // one ABC condition.
                for _ in comparators.iter() {
                    self.current().abc.record_condition();
                }
                self.visit_expr(left);
                for c in comparators {
                    self.visit_expr(c);
                }
            }
            Expr::Call(ast::ExprCall {
                func, arguments, ..
            }) => {
                self.current().abc.record_branch();
                self.visit_expr(func);
                for a in &arguments.args {
                    self.visit_expr(a);
                }
                for kw in &arguments.keywords {
                    self.visit_expr(&kw.value);
                }
            }
            Expr::Attribute(attr) => {
                // Halstead-wise, `a.b` is two operand tokens
                // (`a` and `b`) plus one operator (`.`) — exactly what
                // the lexer emits. We do NOT emit an extra "attribute"
                // operand for the joined chain text; doing so would
                // triple-count the same syntactic structure (legacy
                // Python tree-sitter walker also did not emit such an
                // entry, see `crates/mehen-engine/src/legacy/getter.rs`
                // — `Attribute` is not in the operand match arms).
                self.visit_expr(&attr.value);
            }
            Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
                self.visit_expr(value);
                self.visit_expr(slice);
            }
            Expr::Starred(ast::ExprStarred { value, .. }) => {
                self.visit_expr(value);
            }
            Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                for e in elts {
                    self.visit_expr(e);
                }
            }
            Expr::Set(ast::ExprSet { elts, .. }) => {
                for e in elts {
                    self.visit_expr(e);
                }
            }
            Expr::Slice(ast::ExprSlice {
                lower, upper, step, ..
            }) => {
                if let Some(l) = lower {
                    self.visit_expr(l);
                }
                if let Some(u) = upper {
                    self.visit_expr(u);
                }
                if let Some(s) = step {
                    self.visit_expr(s);
                }
            }
            Expr::Dict(ast::ExprDict { items, .. }) => {
                for item in items {
                    if let Some(k) = &item.key {
                        self.visit_expr(k);
                    }
                    self.visit_expr(&item.value);
                }
            }
            Expr::ListComp(ast::ExprListComp {
                elt, generators, ..
            })
            | Expr::SetComp(ast::ExprSetComp {
                elt, generators, ..
            })
            | Expr::Generator(ast::ExprGenerator {
                elt, generators, ..
            }) => {
                self.visit_expr(elt);
                for comp in generators {
                    self.visit_comprehension(comp);
                }
            }
            Expr::DictComp(ast::ExprDictComp {
                key,
                value,
                generators,
                ..
            }) => {
                self.visit_expr(key);
                self.visit_expr(value);
                for comp in generators {
                    self.visit_comprehension(comp);
                }
            }
            Expr::Await(ast::ExprAwait { value, .. })
            | Expr::Yield(ast::ExprYield {
                value: Some(value), ..
            })
            | Expr::YieldFrom(ast::ExprYieldFrom { value, .. }) => {
                self.visit_expr(value);
            }
            Expr::Yield(ast::ExprYield { value: None, .. }) => {}
            Expr::FString(ast::ExprFString { value, .. }) => {
                for part in value.iter() {
                    if let ast::FStringPart::FString(fs) = part {
                        for elem in fs.elements.iter() {
                            if let ast::InterpolatedStringElement::Interpolation(interp) = elem {
                                self.visit_expr(&interp.expression);
                            }
                        }
                    }
                }
            }
            Expr::TString(ast::ExprTString { value, .. }) => {
                for part in value.iter() {
                    for elem in part.elements.iter() {
                        if let ast::InterpolatedStringElement::Interpolation(interp) = elem {
                            self.visit_expr(&interp.expression);
                        }
                    }
                }
            }
            // Atomic expressions — no metric impact via the AST walk;
            // their tokens are picked up by the Halstead token sweep.
            Expr::Name(_)
            | Expr::StringLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
            | Expr::EllipsisLiteral(_)
            | Expr::IpyEscapeCommand(_) => {}
        }
    }

    fn visit_comprehension(&mut self, comp: &ast::Comprehension) {
        // A comprehension's first generator is +1 cyclomatic (the
        // implicit `for`); each `if` filter is also +1.
        self.current().cyclomatic.record_decision();
        self.current().abc.record_condition();
        for _ in &comp.ifs {
            self.current().cyclomatic.record_decision();
            self.current().abc.record_condition();
        }
        self.visit_expr(&comp.target);
        self.visit_expr(&comp.iter);
        for f in &comp.ifs {
            self.visit_expr(f);
        }
    }

    fn observe_loc_for_stmt(&mut self, stmt: &Stmt) {
        let range = stmt.range();
        let start_row = self
            .line_index
            .line_at(range.start().to_u32())
            .saturating_sub(1);
        let cur = self.current();
        // LLOC: only "actionable" statements. Container statements
        // (`def`, `class`) are not LLOC per the legacy
        // `legacy/metrics/loc.rs::PythonCode::compute` enumeration —
        // their bodies contain the lloc-bumping nodes. Match expr and
        // bare keywords (`pass`/`break`/etc.) are still bumped via the
        // generic `Stmt::*` arm.
        let is_lloc = !matches!(
            stmt,
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) | Stmt::TypeAlias(_)
        );
        if is_lloc {
            cur.loc.observe_lloc();
        }
        cur.loc.observe_code_line(start_row);
    }

    /// Token-stream Halstead emission — runs after the AST walk.
    ///
    /// Each Ruff token is mapped to one of `Operator(kind)`,
    /// `Operand(kind)`, or `Skip`. Tokens whose span falls inside a
    /// recorded docstring are skipped entirely. Type annotations and
    /// default values are NOT skipped — Python types are runtime
    /// objects, not erased metadata.
    fn emit_halstead_from_tokens(&mut self, tokens: &ruff_python_ast::token::Tokens) {
        // Sort docstring ranges so a binary scan is cheap.
        self.docstring_ranges.sort_by_key(|r| r.start());

        for tok in tokens.iter() {
            let span = tok.range();

            // LOC: comment tokens contribute to `cloc`. The legacy
            // walker `legacy/metrics/loc.rs::PythonCode::compute`
            // matches the `Comment` node and calls `add_cloc_lines`;
            // the equivalent here is `observe_comment` on the unit
            // state (Python comments are always single-line).
            if matches!(tok.kind(), TokenKind::Comment) {
                let start_row = self
                    .line_index
                    .line_at(span.start().to_u32())
                    .saturating_sub(1);
                let end_row = self
                    .line_index
                    .line_at(span.end().to_u32())
                    .saturating_sub(1);
                self.stack[0].loc.observe_comment(start_row, end_row);
            }

            // Module-level docstrings are PEP 257 documentation, so
            // their tokens are excluded from Halstead. The legacy
            // walker also folded triple-quoted module/class/function
            // docstrings into `cloc` via the `String` arm — apply the
            // same here so cloc covers both `# …` line comments and
            // top-of-body docstrings.
            if self.is_inside_docstring(span)
                && matches!(
                    tok.kind(),
                    TokenKind::String | TokenKind::FStringStart | TokenKind::FStringEnd
                )
            {
                let start_row = self
                    .line_index
                    .line_at(span.start().to_u32())
                    .saturating_sub(1);
                let end_row = self
                    .line_index
                    .line_at(span.end().to_u32())
                    .saturating_sub(1);
                self.stack[0].loc.observe_comment(start_row, end_row);
            }

            if self.is_inside_docstring(span) {
                continue;
            }
            match classify_token(tok.kind()) {
                TokenClass::Operator(kind) => {
                    self.stack[0].halstead.observe_operator(HalsteadOperator {
                        kind: SmolStr::new(kind),
                        text: None,
                    });
                }
                TokenClass::Operand(kind) => {
                    let text = self
                        .source
                        .get(span.start().to_u32() as usize..span.end().to_u32() as usize)
                        .unwrap_or("");
                    self.stack[0].halstead.observe_operand(HalsteadOperand {
                        kind: SmolStr::new(kind),
                        text: Some(SmolStr::new(text)),
                    });
                }
                TokenClass::Skip => {}
            }
        }
    }

    fn is_inside_docstring(&self, span: TextRange) -> bool {
        self.docstring_ranges
            .iter()
            .any(|r| span.start() >= r.start() && span.end() <= r.end())
    }
}

fn text_range_to_source_span(range: TextRange, line_index: &LineIndex) -> SourceSpan {
    SourceSpan {
        start_byte: range.start().to_u32(),
        end_byte: range.end().to_u32(),
        start_line: line_index.line_at(range.start().to_u32()),
        end_line: line_index.line_at(range.end().to_u32()),
    }
}

fn leading_docstring_range(body: &[Stmt]) -> Option<TextRange> {
    let first = body.first()?;
    if let Stmt::Expr(ast::StmtExpr { value, .. }) = first
        && let Expr::StringLiteral(s) = value.as_ref()
    {
        return Some(s.range);
    }
    None
}

fn python_method_is_public(name: &str) -> bool {
    if name.starts_with("__") && name.ends_with("__") {
        return true;
    }
    !name.starts_with('_')
}

fn python_attribute_is_public(name: &str) -> bool {
    if name.starts_with("__") && name.ends_with("__") {
        return true;
    }
    !name.starts_with('_')
}

enum TokenClass {
    Operator(&'static str),
    Operand(&'static str),
    Skip,
}

fn classify_token(kind: TokenKind) -> TokenClass {
    use TokenClass::*;
    use TokenKind::*;
    match kind {
        // Operators — punctuation that reads as `do something`.
        Lpar => Operator("("),
        Lsqb => Operator("["),
        Lbrace => Operator("{"),
        Comma => Operator(","),
        Colon => Operator(":"),
        Semi => Operator(";"),
        Dot => Operator("."),
        At => Operator("@"),
        Plus => Operator("+"),
        Minus => Operator("-"),
        Star => Operator("*"),
        Slash => Operator("/"),
        Percent => Operator("%"),
        Vbar => Operator("|"),
        Amper => Operator("&"),
        CircumFlex => Operator("^"),
        Tilde => Operator("~"),
        DoubleStar => Operator("**"),
        DoubleSlash => Operator("//"),
        LeftShift => Operator("<<"),
        RightShift => Operator(">>"),
        Less => Operator("<"),
        Greater => Operator(">"),
        Equal => Operator("="),
        EqEqual => Operator("=="),
        NotEqual => Operator("!="),
        LessEqual => Operator("<="),
        GreaterEqual => Operator(">="),
        PlusEqual => Operator("+="),
        MinusEqual => Operator("-="),
        StarEqual => Operator("*="),
        SlashEqual => Operator("/="),
        PercentEqual => Operator("%="),
        AmperEqual => Operator("&="),
        VbarEqual => Operator("|="),
        CircumflexEqual => Operator("^="),
        DoubleStarEqual => Operator("**="),
        DoubleSlashEqual => Operator("//="),
        LeftShiftEqual => Operator("<<="),
        RightShiftEqual => Operator(">>="),
        ColonEqual => Operator(":="),
        AtEqual => Operator("@="),
        Rarrow => Operator("->"),
        // Keywords — these are Halstead operators.
        And => Operator("and"),
        Or => Operator("or"),
        Not => Operator("not"),
        If => Operator("if"),
        Elif => Operator("elif"),
        Else => Operator("else"),
        For => Operator("for"),
        While => Operator("while"),
        Try => Operator("try"),
        Except => Operator("except"),
        Finally => Operator("finally"),
        With => Operator("with"),
        Return => Operator("return"),
        Raise => Operator("raise"),
        Yield => Operator("yield"),
        Assert => Operator("assert"),
        Import => Operator("import"),
        From => Operator("from"),
        As => Operator("as"),
        Pass => Operator("pass"),
        Break => Operator("break"),
        Continue => Operator("continue"),
        Def => Operator("def"),
        Class => Operator("class"),
        Lambda => Operator("lambda"),
        In => Operator("in"),
        Is => Operator("is"),
        Async => Operator("async"),
        Await => Operator("await"),
        Global => Operator("global"),
        Nonlocal => Operator("nonlocal"),
        Del => Operator("del"),
        // Soft keywords (`match`, `case`, `type`, `_`-as-pattern) read
        // as operators when they head a statement.
        Match => Operator("match"),
        Case => Operator("case"),
        Type => Operator("type"),
        // Operands — leaves that name or contain a value.
        Name => Operand("Identifier"),
        Int | Float | Complex => Operand("Number"),
        String | FStringStart | FStringMiddle | FStringEnd | TStringStart | TStringMiddle
        | TStringEnd => Operand("String"),
        True => Operand("True"),
        False => Operand("False"),
        None => Operand("None"),
        Ellipsis => Operand("Ellipsis"),
        // Closing punctuation, newlines, indents, and comments are not
        // counted (closing `)` etc. would double-count alongside their
        // opening counterpart — Halstead's classical formula counts
        // brackets *as a pair*, with the open bracket as the operator
        // and the close as a no-op).
        Rpar | Rsqb | Rbrace => Skip,
        Newline | NonLogicalNewline | Indent | Dedent | EndOfFile | Comment | Question
        | Exclamation | Lazy | Unknown | IpyEscapeCommand => Skip,
    }
}
