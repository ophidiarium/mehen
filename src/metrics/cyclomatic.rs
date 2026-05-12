use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::{
    CCode, GoCode, KotlinCode, PowershellCode, PythonCode, RubyCode, RustCode, TsxCode,
    TypescriptCode,
};
use crate::languages::{C, Kotlin, Powershell, Python, Ruby, Rust, Tsx, Typescript};
use crate::node::Node;
use crate::rust_metric_helpers::{is_inside_rust_macro_tokens, is_rust_logical_operator};

/// The `Cyclomatic` metric.
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    cyclomatic_sum: f64,
    cyclomatic: f64,
    n: usize,
    cyclomatic_max: f64,
    cyclomatic_min: f64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            cyclomatic_sum: 0.,
            cyclomatic: 1.,
            n: 1,
            cyclomatic_max: 0.,
            cyclomatic_min: f64::MAX,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("cyclomatic", 4)?;
        st.serialize_field("sum", &self.cyclomatic_sum())?;
        st.serialize_field("average", &self.cyclomatic_average())?;
        st.serialize_field("min", &self.cyclomatic_min())?;
        st.serialize_field("max", &self.cyclomatic_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {}, min: {}, max: {}",
            self.cyclomatic_sum(),
            self.cyclomatic_average(),
            self.cyclomatic_min(),
            self.cyclomatic_max()
        )
    }
}

impl Stats {
    /// Merges a second `Cyclomatic` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        //Calculate minimum and maximum values
        self.cyclomatic_max = self.cyclomatic_max.max(other.cyclomatic_max);
        self.cyclomatic_min = self.cyclomatic_min.min(other.cyclomatic_min);

        self.cyclomatic_sum += other.cyclomatic_sum;
        self.n += other.n;
    }

    /// Returns the `Cyclomatic` metric value
    pub(crate) fn cyclomatic(&self) -> f64 {
        self.cyclomatic
    }
    /// Returns the sum
    pub(crate) fn cyclomatic_sum(&self) -> f64 {
        self.cyclomatic_sum
    }

    /// Returns the `Cyclomatic` metric average value
    ///
    /// This value is computed dividing the `Cyclomatic` value for the
    /// number of spaces.
    pub(crate) fn cyclomatic_average(&self) -> f64 {
        self.cyclomatic_sum() / self.n as f64
    }
    /// Returns the `Cyclomatic` maximum value
    pub(crate) fn cyclomatic_max(&self) -> f64 {
        self.cyclomatic_max
    }
    /// Returns the `Cyclomatic` minimum value
    pub(crate) fn cyclomatic_min(&self) -> f64 {
        self.cyclomatic_min
    }
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.cyclomatic_sum += self.cyclomatic;
    }
    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.cyclomatic_max = self.cyclomatic_max.max(self.cyclomatic);
        self.cyclomatic_min = self.cyclomatic_min.min(self.cyclomatic);
        self.compute_sum();
    }
}

pub(crate) trait Cyclomatic
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats);
}

impl Cyclomatic for PythonCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Python::*;

        match node.kind_id().into() {
            If | Elif | For | While | Except | And | Or => {
                stats.cyclomatic += 1.;
            }
            Else if node.has_ancestors(
                |node| matches!(node.kind_id().into(), ForStatement | WhileStatement),
                |node| node.kind_id() == ElseClause,
            ) =>
            {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for TypescriptCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Typescript::*;

        match node.kind_id().into() {
            If | For | While | Case | Catch | TernaryExpression | AMPAMP | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for TsxCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Tsx::*;

        match node.kind_id().into() {
            If | For | While | Case | Catch | TernaryExpression | AMPAMP | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for RustCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Rust::*;

        if is_inside_rust_macro_tokens(node) {
            return;
        }

        match node.kind_id().into() {
            IfExpression | ForExpression | WhileExpression | LoopExpression | MatchArm
            | MatchArm2 | TryExpression => {
                stats.cyclomatic += 1.;
            }
            _ if is_rust_logical_operator(node) => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for GoCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use crate::languages::Go::*;

        match node.kind_id().into() {
            If | For | ExpressionCase | TypeCase | CommunicationCase | AMPAMP | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            // `default_case` is shared between switch and select in the Go
            // grammar. In switch it is fallthrough (no branch), but in
            // `select` the default branch is an additional executable path
            // and should count as a decision point.
            DefaultCase
                if node
                    .parent()
                    .is_some_and(|p| p.kind_id() == SelectStatement as u16) =>
            {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for RubyCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Ruby::*;

        match node.kind_id().into() {
            // Decision points: if / unless / while / until / for,
            // their trailing-modifier forms, pattern matching arms, ternary,
            // rescue branches, and short-circuit boolean operators.
            If | Elsif | Unless | While | Until | For | IfModifier | UnlessModifier
            | WhileModifier | UntilModifier | When | InClause | Rescue | RescueModifier
            | RescueModifier2 | RescueModifier3 | Conditional | AMPAMP | PIPEPIPE | And | Or => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for KotlinCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Kotlin::*;

        // Decision-point set is aligned with SonarKotlin's
        // `CyclomaticComplexityVisitor`: every `KtIfExpression` (including
        // else-if branches, because each parses as a nested `if_expression`),
        // every loop (`for`/`while`/`do-while`), every `when_entry`, and
        // each short-circuit `&&`/`||`. `catch` is intentionally excluded —
        // SonarKotlin does not count it towards cyclomatic complexity.
        // Reference: sonar-kotlin-metrics/.../CyclomaticComplexityVisitor.kt
        match node.kind_id().into() {
            IfExpression | ForStatement | WhileStatement | DoWhileStatement | WhenEntry
            | AMPAMP | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

// No languages require empty Cyclomatic implementations
// implement_metric_trait!(Cyclomatic);

impl Cyclomatic for PowershellCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Powershell::*;

        // Decision-point set aligned with Sonar's language-agnostic
        // cyclomatic definition: every `if`, `elseif`, loop, `switch`
        // clause, `catch`, `trap`, real ternary / null-coalesce, and each
        // short-circuit (`&&` / `||`) or logical (`-and` / `-or`) operator
        // adds one. Matches the standard Sonar rule and the community
        // `Get-CyclomaticComplexity` helper that counts `If`, `ElseIf`,
        // `Catch`, `While`, `For`, `Switch` tokens, extended with
        // `foreach` / `do` loops and v7's `?` / `??`.
        //
        // `-xor` is intentionally NOT counted. Sonar's cyclomatic rule
        // counts only *short-circuit* boolean operators across every
        // language it analyzes (JS/TS/PHP/C#/Java/Dart list `&&`/`||`
        // explicitly; VB.NET lists `AndAlso`/`OrElse`, not `And`/`Or`).
        // PowerShell's `-xor` always evaluates both operands — by
        // definition it cannot introduce a new control-flow path — so
        // it doesn't meet the cyclomatic criterion. It IS counted in ABC
        // conditions and cognitive boolean-sequence scoring, where the
        // relevant axis is "boolean operator occurrence" rather than
        // "new linearly independent path".
        //
        // tree-sitter-pwsh v0.37+ only emits the operator-level expression
        // kinds (`ternary_expression`, `null_coalesce_expression`, ...)
        // when the operator is actually present, so matching on the kind
        // is sufficient — no operator-token guard is needed. See
        // wharflab/tree-sitter-powershell#56. The grammar also emits a
        // parallel family of `*_argument_expression` kinds for the same
        // operators when they appear inside a method-invocation
        // `argument_list` (e.g. `[Foo]::Bar($cond ? 1 : 2)`), so we match
        // both families.
        match node.kind_id().into() {
            IfStatement
            | ElseifClause
            | ForStatement
            | ForeachStatement
            | WhileStatement
            | DoStatement
            | SwitchClause
            | CatchClause
            | TrapStatement
            | TernaryExpression
            | TernaryArgumentExpression
            | NullCoalesceExpression
            | NullCoalesceArgumentExpression
            | AMPAMP
            | PIPEPIPE
            | DASHand
            | DASHor => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for CCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use C::*;

        match node.kind_id().into() {
            // Decision-point set aligned with Sonar's cyclomatic rule for C:
            // `if`, every `case`, loops, ternary (`conditional_expression`),
            // and each short-circuit boolean operator (`&&` / `||`).
            // `switch` itself is not a decision (cases are); `default` is
            // fallthrough.
            IfStatement
            | CaseStatement
            | ForStatement
            | WhileStatement
            | DoStatement
            | ConditionalExpression
            | AMPAMP
            | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

// Markdown is a documentation language; cyclomatic is a code metric. The
// dedicated Markdown pipeline computes its own MRPC analogue (Phase B).
#[cfg(feature = "markdown")]
impl Cyclomatic for crate::langs::MarkdownCode {
    fn compute(_node: &Node, _stats: &mut Stats) {}
}

#[cfg(test)]
mod tests {
    use crate::langs::{
        GoParser, KotlinParser, PowershellParser, PythonParser, RubyParser, RustParser,
        TypescriptParser,
    };
    use crate::tools::check_metrics;

    #[test]
    fn typescript_for_variants_count_once() {
        // `for`, `for…in`, `for…of` each should contribute +1 via the
        // `For` anonymous keyword token.
        check_metrics::<TypescriptParser>(
            "function f(arr) { // +2 (+1 unit space)
                 for (let i = 0; i < 3; i++) {}  // +1
                 for (const k in arr) {}          // +1
                 for (const v of arr) {}          // +1
             }",
            "foo.ts",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn typescript_do_while() {
        check_metrics::<TypescriptParser>(
            "function f() { // +2 (+1 unit space)
                 do {
                     x++;
                 } while (x < 10); // +1 loop
             }",
            "foo.ts",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 1.5,
                      "min": 1.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_simple_function() {
        check_metrics::<PythonParser>(
            "def f(a, b): # +2 (+1 unit space)
                if a and b:  # +2 (+1 and)
                   return 1
                if c and d: # +2 (+1 and)
                   return 1",
            "foo.py",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 3.0,
                      "min": 1.0,
                      "max": 5.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_1_level_nesting() {
        check_metrics::<PythonParser>(
            "def f(a, b): # +2 (+1 unit space)
                if a:  # +1
                    for i in range(b):  # +1
                        return 1",
            "foo.py",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 2.0,
                      "min": 1.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_1_level_nesting() {
        check_metrics::<RustParser>(
            "fn f() { // +2 (+1 unit space)
                 if true { // +1
                     match true {
                         true => println!(\"test\"), // +1
                         false => println!(\"test\"), // +1
                     }
                 }
             }",
            "foo.rs",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_macro_tokens_are_opaque_for_cyclomatic() {
        check_metrics::<RustParser>(
            "fn f() {
                 maybe!(a && b, if c { d() });
             }",
            "foo.rs",
            |metric| {
                // Unit + function baselines only; macro token-tree control
                // tokens are not parsed Rust control flow.
                assert_eq!(metric.cyclomatic.cyclomatic_sum(), 2.0);
            },
        );
    }

    #[test]
    fn go_simple_function() {
        check_metrics::<GoParser>(
            "package main

            func calculate(a, b int) int { // +2 (+1 unit space)
                if a > b { // +1
                    return a
                }
                return b
            }",
            "foo.go",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 1.5,
                      "min": 1.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_switch_statement() {
        check_metrics::<GoParser>(
            "package main

            func grade(score int) string { // +2 (+1 unit space)
                switch { // switch itself doesn't add, cases do
                case score >= 90: // +1
                    return \"A\"
                case score >= 80: // +1
                    return \"B\"
                case score >= 70: // +1
                    return \"C\"
                default: // default is fallthrough, not a decision point
                    return \"F\"
                }
            }",
            "foo.go",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_select_default_counts() {
        // `default` in a `switch` is fallthrough and should NOT count,
        // but `default` in a `select` is an additional executable
        // communication branch and SHOULD count.
        check_metrics::<GoParser>(
            "package main

            func f(ch chan int) { // +2 (+1 unit space)
                select { // +1 CommunicationCase
                case v := <-ch:
                    _ = v
                default: // +1 default branch of select
                }
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 2.0,
                      "min": 1.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_logical_operators() {
        check_metrics::<GoParser>(
            "package main

            func check(a, b, c bool) bool { // +2 (+1 unit space)
                if a && b || c { // +3 (+1 if, +1 &&, +1 ||)
                    return true
                }
                return false
            }",
            "foo.go",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_simple_method() {
        check_metrics::<RubyParser>(
            "def f(a, b) # +2 (+1 unit space)
                 if a && b # +2 (+1 if, +1 &&)
                     return 1
                 end
                 if c or d # +2 (+1 if, +1 or)
                     return 1
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 3.0,
                      "min": 1.0,
                      "max": 5.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_modifier_forms() {
        // Each trailing-modifier form contributes +1 like its block form.
        check_metrics::<RubyParser>(
            "def f(a)      # +1 unit space +1 method
                 return a if a > 0   # +1 if_modifier
                 return -a unless a == 0 # +1 unless_modifier
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 2.0,
                      "min": 1.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_simple_function() {
        check_metrics::<KotlinParser>(
            "fun f(a: Int, b: Int): Int { // +2 (+1 unit space, +1 fun)
                 if (a > b) { // +1
                     return a
                 }
                 return b
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 1.5,
                      "min": 1.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_when_branches_count() {
        // `when` itself doesn't add; each branch (`when_entry`) does.
        check_metrics::<KotlinParser>(
            "fun grade(score: Int): String { // +2 (+1 unit, +1 fun)
                 return when { // +0
                     score >= 90 -> \"A\" // +1
                     score >= 80 -> \"B\" // +1
                     score >= 70 -> \"C\" // +1
                     else -> \"F\"       // +1 (else is its own when_entry)
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 3.0,
                      "min": 1.0,
                      "max": 5.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_try_catch_counts_catch_not_try() {
        // Aligns with SonarKotlin's `CyclomaticComplexityVisitor`: `try`
        // itself is NOT a decision point, but each `catch` is NOT either —
        // SonarKotlin counts `catch` only in cognitive complexity, not
        // cyclomatic. Reference:
        //   sonar-kotlin-metrics/.../CyclomaticComplexityVisitor.kt
        check_metrics::<KotlinParser>(
            "fun f() { // +2 (+1 unit, +1 fun)
                 try {
                     risky()
                 } catch (e: Exception) {
                     // catch does not add cyclomatic complexity per SonarKotlin
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 1.0,
                      "min": 1.0,
                      "max": 1.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_logical_operators() {
        check_metrics::<KotlinParser>(
            "fun check(a: Boolean, b: Boolean, c: Boolean): Boolean { // +2
                 if (a && b || c) { // +3 (+1 if, +1 &&, +1 ||)
                     return true
                 }
                 return false
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_case_when() {
        check_metrics::<RubyParser>(
            "def f(x)      # +1 unit +1 method
                 case x    # case itself doesn't add; each `when` does
                 when 1 then 'a' # +1
                 when 2 then 'b' # +1
                 when 3 then 'c' # +1
                 else 'z'
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_simple_function() {
        check_metrics::<PowershellParser>(
            "function Greet($name) { # +2 (+1 unit, +1 function)
                 if ($name) {         # +1
                     Write-Host \"hi, $name\"
                 }
             }",
            "foo.ps1",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 1.5,
                      "min": 1.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_counts_each_switch_clause() {
        // The `switch` statement itself does NOT add a decision; each
        // `switch_clause` does. Aligns with Sonar's general cyclomatic rule.
        //
        // In the tree-sitter-pwsh grammar, every `{ ... }` script block is
        // a `script_block_expression`, which mehen treats as its own
        // closure-like function space (the same convention Kotlin uses for
        // `lambda_literal`). Each clause body and predicate therefore opens
        // a fresh space with a base cyclomatic of 1, and the sum is the
        // aggregate across all of them.
        check_metrics::<PowershellParser>(
            "function Grade($score) {
                 switch ($score) {
                     1 { 'A' }
                     2 { 'B' }
                     3 { 'C' }
                     default { 'F' }
                 }
             }",
            "foo.ps1",
            |metric| {
                // Decisions: 4 switch clauses. Base 1s at unit, function
                // (no extra spaces because literal clause conditions do
                // not introduce an additional `script_block_expression`;
                // the result body `{ 'A' }` is parsed as the clause's
                // `statement_block`, not a closure space).
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 3.0,
                      "min": 1.0,
                      "max": 5.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_short_circuit_and_word_form_boolean_operators() {
        // PowerShell has two boolean operator pairs:
        //   - short-circuit `&&` / `||` (inside `pipeline_chain`)
        //   - logical `-and` / `-or` / `-xor` (inside `logical_expression`)
        // Each occurrence contributes +1.
        check_metrics::<PowershellParser>(
            "function Check($a, $b, $c) { # +2 (+1 unit, +1 function)
                 if ($a -and $b -or $c) { # +3 (+1 if, +1 -and, +1 -or)
                     return $true
                 }
                 return $false
             }",
            "foo.ps1",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_ternary_and_null_coalesce_wrappers_do_not_false_trigger() {
        // Regression: the tree-sitter-pwsh grammar emits `ternary_expression`,
        // `null_coalesce_expression`, and `logical_expression` as wrapper
        // kinds in the precedence cascade even for plain expressions like
        // `$a + $b`. Those wrappers must NOT contribute to cyclomatic;
        // only the real `?` / `??` / `-and` / `-or` operator tokens do.
        check_metrics::<PowershellParser>(
            "function Plain { # +2 (+1 unit, +1 function)
                 $x = $a + $b  # no decision point
                 return $x     # no decision point
             }",
            "foo.ps1",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 1.0,
                      "min": 1.0,
                      "max": 1.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_real_ternary_and_null_coalesce_count() {
        // Real `?` / `??` expressions add one decision each.
        check_metrics::<PowershellParser>(
            "$a = $cond ? 1 : 2   # +1 ternary
             $b = $x ?? 0         # +1 null-coalesce",
            "foo.ps1",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 3.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_argument_form_ternary_and_null_coalesce_count() {
        // Regression: tree-sitter-pwsh emits a parallel family of
        // `*_argument_expression` kinds for expressions that live inside a
        // method-invocation `argument_list` (e.g. `[Foo]::Bar($cond ? 1 : 2)`).
        // Those argument-form decision operators must count the same as
        // their regular-form twins.
        check_metrics::<PowershellParser>(
            "function F($a, $b, $x, $cond) { # +2 (+1 unit, +1 function)
                 [Foo]::Bar($a -eq $b)        # comparison: no decision
                 [Foo]::Baz($cond ? 1 : 2)    # +1 ternary
                 [Foo]::Qux($x ?? 3)          # +1 null-coalesce
             }",
            "foo.ps1",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 2.0,
                      "min": 1.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_xor_is_not_a_cyclomatic_decision_point() {
        // Regression: `-xor` is intentionally excluded from the cyclomatic
        // decision-point set. Sonar's cyclomatic rule counts only
        // *short-circuit* boolean operators across every language it
        // analyzes; `-xor` always evaluates both operands so it cannot
        // introduce a new control-flow path. `-and` / `-or` are counted
        // because they short-circuit.
        //
        // The test locks in the expected sum for:
        //   base +1 (unit) + function +1 + if +1 + -and +1  = 4
        //   but NOT +1 for `-xor` (would be 5 if it were miscounted).
        check_metrics::<PowershellParser>(
            "function f($a, $b, $c) {
                 if ($a -xor $b) { }        # +1 if, NOT +1 -xor
                 if ($a -and $b) { }        # +1 if, +1 -and
             }",
            "foo.ps1",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }
}
