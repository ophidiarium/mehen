//! Ruby-specific tests for the Phase 9 ruby-prism walker — exercises
//! Ruby idioms the legacy tree-sitter walker handled incompletely or
//! had no fixture for. Per the rewrite plan §6.5 the prism migration
//! is justified by "first-class" handling of these forms.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_ruby::RubyAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RubyAnalyzer::new();
    let file = SourceFile::new("foo.rb".into(), Language::Ruby, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn ruby_safe_navigation_does_not_crash() {
    // `&.` safe-navigation method call. Prism flags via
    // `CallNode::is_safe_navigation()`; the walker treats it as a
    // regular ABC.B branch (just like a normal `.` call). The legacy
    // tree-sitter grammar exposed it as a separate `&.` punctuation
    // child of `call`, but the metric outcome is the same: one branch.
    let a = analyze(
        "def f(obj)
             obj&.bar&.baz
         end",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    // Two safe-nav method calls (`bar`, `baz`).
    assert_eq!(abc.branches, 2.0);
}

#[test]
fn ruby_pattern_matching_each_in_branch_is_a_decision() {
    // Pattern matching `case … in pat` — each `in` clause is a
    // cyclomatic decision, just like classic `when`. Legacy walker
    // handled this via the `InClause` node kind; prism exposes it as
    // `CaseMatchNode` containing `InNode`s.
    let a = analyze(
        "def f(x)
             case x
             in [1, *]
                 :a
             in {a:, **}
                 :b
             in Integer => n if n > 0
                 :c
             end
         end",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    // unit (+1), method (+1), 3× `in` (+3), guard `if` (+1), `>` (+0
    // because comparison is a CallNode, not a binary expression that
    // adds cyclomatic — only logical ops do).
    // Total = 6.
    assert_eq!(cy.sum, 6.0, "{}", serde_json::to_string(&cy).unwrap());
}

#[test]
fn ruby_endless_method_definition() {
    // Ruby 3.0+ endless method: `def square(x) = x * x`. Prism's
    // DefNode has `end_keyword_loc().is_none()` for endless methods.
    // We still count it as one method (NOM=1) and one space.
    let a = analyze("def square(x) = x * x");
    let nom = mehen_report::metrics_json::nom(&a.root.metrics);
    assert_eq!(
        nom.functions,
        1.0,
        "{}",
        serde_json::to_string(&nom).unwrap()
    );
}

#[test]
fn ruby_numbered_block_parameters_count_correctly() {
    // Ruby 2.7+ numbered block params (`_1`, `_2`, etc.) — prism
    // exposes these as `NumberedParametersNode` with a `maximum: u8`
    // field. Legacy tree-sitter had a hand-rolled `block_argument`
    // walk that didn't always recover the implicit arity.
    let a = analyze(
        "[1, 2, 3].each_with_index do
             puts _1 + _2
         end",
    );
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    // Block has implicit `_1, _2` → 2 closure args.
    assert_eq!(
        nargs.total_closures,
        2.0,
        "{}",
        serde_json::to_string(&nargs).unwrap()
    );
}

#[test]
fn ruby_singleton_class_body_contributes_to_class_metrics() {
    // `class << self; def foo; end; end` — singleton class scope,
    // a class-like space in prism. Methods inside count toward NPM.
    let a = analyze(
        "class C
             class << self
                 def cls_method
                     1
                 end
             end
         end",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    assert!(
        npm.total_methods >= 1.0,
        "{}",
        serde_json::to_string(&npm).unwrap()
    );
}

#[test]
fn ruby_modifier_if_does_not_increase_nesting() {
    // Sonar cognitive spec: `x if y` adds +1 without nesting. Two
    // sibling modifier-if statements should each contribute +1, NOT
    // collapse into a nested-if pattern.
    let a = analyze(
        "def f(a, b)
             return 1 if a    # +1
             return 0 if b    # +1
         end",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    assert_eq!(cog.sum, 2.0, "{}", serde_json::to_string(&cog).unwrap());
}

#[test]
fn ruby_op_assignment_writes_count_as_assignment_and_decision() {
    // `x &&= y` and `x ||= y` are short-circuiting writes — they
    // count as both ABC.A (assignment) AND ABC.C (condition / cyclomatic).
    // Legacy tree-sitter exposed these as `operator_assignment`; prism
    // splits into `LocalVariableAndWriteNode` / `LocalVariableOrWriteNode`.
    let a = analyze(
        "def f(x)
             x &&= 1
             x ||= 2
         end",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    assert_eq!(
        abc.assignments,
        2.0,
        "{}",
        serde_json::to_string(&abc).unwrap()
    );
    assert_eq!(
        abc.conditions,
        2.0,
        "{}",
        serde_json::to_string(&abc).unwrap()
    );
    // unit (+1), method (+1), &&= (+1), ||= (+1) = 4
    assert_eq!(cy.sum, 4.0, "{}", serde_json::to_string(&cy).unwrap());
}
