use std::collections::HashSet;

use crate::legacy::checker::Checker;
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::langs::{CCode, GoCode, KotlinCode, RubyCode};
use crate::legacy::languages::{C, Kotlin, Ruby};
use crate::legacy::node::Node;

/// The `SLoc` metric suite.
#[derive(Debug, Clone)]
pub(crate) struct Sloc {
    start: usize,
    end: usize,
    unit: bool,
    sloc_min: usize,
    sloc_max: usize,
}

impl Default for Sloc {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            unit: false,
            sloc_min: usize::MAX,
            sloc_max: 0,
        }
    }
}

impl Sloc {
    #[inline(always)]
    pub(crate) fn sloc(&self) -> f64 {
        // This metric counts the number of lines in a file
        // The if construct is needed to count the line of code that represents
        // the function signature in a function space
        let sloc = if self.unit {
            self.end - self.start
        } else {
            (self.end - self.start) + 1
        };
        sloc as f64
    }

    /// The `Sloc` metric minimum value.
    #[inline(always)]
    pub(crate) fn sloc_min(&self) -> f64 {
        self.sloc_min as f64
    }

    /// The `Sloc` metric maximum value.
    #[inline(always)]
    pub(crate) fn sloc_max(&self) -> f64 {
        self.sloc_max as f64
    }

    #[inline(always)]
    pub(crate) fn merge(&mut self, other: &Self) {
        self.sloc_min = self.sloc_min.min(other.sloc() as usize);
        self.sloc_max = self.sloc_max.max(other.sloc() as usize);
    }

    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        if self.sloc_min == usize::MAX {
            self.sloc_min = self.sloc_min.min(self.sloc() as usize);
            self.sloc_max = self.sloc_max.max(self.sloc() as usize);
        }
    }
}

/// The `PLoc` metric suite.
#[derive(Debug, Clone)]
pub(crate) struct Ploc {
    lines: HashSet<usize>,
    ploc_min: usize,
    ploc_max: usize,
}

impl Default for Ploc {
    fn default() -> Self {
        Self {
            lines: HashSet::default(),
            ploc_min: usize::MAX,
            ploc_max: 0,
        }
    }
}

impl Ploc {
    #[inline(always)]
    pub(crate) fn ploc(&self) -> f64 {
        // This metric counts the number of instruction lines in a code
        // https://en.wikipedia.org/wiki/Source_lines_of_code
        self.lines.len() as f64
    }

    /// The `Ploc` metric minimum value.
    #[inline(always)]
    pub(crate) fn ploc_min(&self) -> f64 {
        self.ploc_min as f64
    }

    /// The `Ploc` metric maximum value.
    #[inline(always)]
    pub(crate) fn ploc_max(&self) -> f64 {
        self.ploc_max as f64
    }

    #[inline(always)]
    pub(crate) fn merge(&mut self, other: &Self) {
        // Merge ploc lines
        for l in &other.lines {
            self.lines.insert(*l);
        }

        self.ploc_min = self.ploc_min.min(other.ploc() as usize);
        self.ploc_max = self.ploc_max.max(other.ploc() as usize);
    }

    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        if self.ploc_min == usize::MAX {
            self.ploc_min = self.ploc_min.min(self.ploc() as usize);
            self.ploc_max = self.ploc_max.max(self.ploc() as usize);
        }
    }
}

/// The `CLoc` metric suite.
#[derive(Debug, Clone)]
pub(crate) struct Cloc {
    only_comment_lines: usize,
    code_comment_lines: usize,
    comment_line_end: Option<usize>,
    cloc_min: usize,
    cloc_max: usize,
}

impl Default for Cloc {
    fn default() -> Self {
        Self {
            only_comment_lines: 0,
            code_comment_lines: 0,
            comment_line_end: Option::default(),
            cloc_min: usize::MAX,
            cloc_max: 0,
        }
    }
}

impl Cloc {
    #[inline(always)]
    pub(crate) fn cloc(&self) -> f64 {
        // Comments are counted regardless of their placement
        // https://en.wikipedia.org/wiki/Source_lines_of_code
        (self.only_comment_lines + self.code_comment_lines) as f64
    }

    /// The `Cloc` metric minimum value.
    #[inline(always)]
    pub(crate) fn cloc_min(&self) -> f64 {
        self.cloc_min as f64
    }

    /// The `Cloc` metric maximum value.
    #[inline(always)]
    pub(crate) fn cloc_max(&self) -> f64 {
        self.cloc_max as f64
    }

    #[inline(always)]
    pub(crate) fn merge(&mut self, other: &Self) {
        // Merge cloc lines
        self.only_comment_lines += other.only_comment_lines;
        self.code_comment_lines += other.code_comment_lines;

        self.cloc_min = self.cloc_min.min(other.cloc() as usize);
        self.cloc_max = self.cloc_max.max(other.cloc() as usize);
    }

    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        if self.cloc_min == usize::MAX {
            self.cloc_min = self.cloc_min.min(self.cloc() as usize);
            self.cloc_max = self.cloc_max.max(self.cloc() as usize);
        }
    }
}

/// The `LLoc` metric suite.
#[derive(Debug, Clone)]
pub(crate) struct Lloc {
    logical_lines: usize,
    lloc_min: usize,
    lloc_max: usize,
}

impl Default for Lloc {
    fn default() -> Self {
        Self {
            logical_lines: 0,
            lloc_min: usize::MAX,
            lloc_max: 0,
        }
    }
}

impl Lloc {
    #[inline(always)]
    pub(crate) fn lloc(&self) -> f64 {
        // This metric counts the number of statements in a code
        // https://en.wikipedia.org/wiki/Source_lines_of_code
        self.logical_lines as f64
    }

    /// The `Lloc` metric minimum value.
    #[inline(always)]
    pub(crate) fn lloc_min(&self) -> f64 {
        self.lloc_min as f64
    }

    /// The `Lloc` metric maximum value.
    #[inline(always)]
    pub(crate) fn lloc_max(&self) -> f64 {
        self.lloc_max as f64
    }

    #[inline(always)]
    pub(crate) fn merge(&mut self, other: &Self) {
        // Merge lloc lines
        self.logical_lines += other.logical_lines;
        self.lloc_min = self.lloc_min.min(other.lloc() as usize);
        self.lloc_max = self.lloc_max.max(other.lloc() as usize);
    }

    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        if self.lloc_min == usize::MAX {
            self.lloc_min = self.lloc_min.min(self.lloc() as usize);
            self.lloc_max = self.lloc_max.max(self.lloc() as usize);
        }
    }
}

/// The `Loc` metric suite.
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    sloc: Sloc,
    ploc: Ploc,
    cloc: Cloc,
    lloc: Lloc,
    space_count: usize,
    blank_min: usize,
    blank_max: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            sloc: Sloc::default(),
            ploc: Ploc::default(),
            cloc: Cloc::default(),
            lloc: Lloc::default(),
            space_count: 1,
            blank_min: usize::MAX,
            blank_max: 0,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("loc", 20)?;
        st.serialize_field("sloc", &self.sloc())?;
        st.serialize_field("ploc", &self.ploc())?;
        st.serialize_field("lloc", &self.lloc())?;
        st.serialize_field("cloc", &self.cloc())?;
        st.serialize_field("blank", &self.blank())?;
        st.serialize_field("sloc_average", &self.sloc_average())?;
        st.serialize_field("ploc_average", &self.ploc_average())?;
        st.serialize_field("lloc_average", &self.lloc_average())?;
        st.serialize_field("cloc_average", &self.cloc_average())?;
        st.serialize_field("blank_average", &self.blank_average())?;
        st.serialize_field("sloc_min", &self.sloc_min())?;
        st.serialize_field("sloc_max", &self.sloc_max())?;
        st.serialize_field("cloc_min", &self.cloc_min())?;
        st.serialize_field("cloc_max", &self.cloc_max())?;
        st.serialize_field("ploc_min", &self.ploc_min())?;
        st.serialize_field("ploc_max", &self.ploc_max())?;
        st.serialize_field("lloc_min", &self.lloc_min())?;
        st.serialize_field("lloc_max", &self.lloc_max())?;
        st.serialize_field("blank_min", &self.blank_min())?;
        st.serialize_field("blank_max", &self.blank_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sloc: {}, ploc: {}, lloc: {}, cloc: {}, blank: {}, sloc_average: {}, ploc_average: {}, lloc_average: {}, cloc_average: {}, blank_average: {}, sloc_min: {}, sloc_max: {}, cloc_min: {}, cloc_max: {}, ploc_min: {}, ploc_max: {}, lloc_min: {}, lloc_max: {}, blank_min: {}, blank_max: {}",
            self.sloc(),
            self.ploc(),
            self.lloc(),
            self.cloc(),
            self.blank(),
            self.sloc_average(),
            self.ploc_average(),
            self.lloc_average(),
            self.cloc_average(),
            self.blank_average(),
            self.sloc_min(),
            self.sloc_max(),
            self.cloc_min(),
            self.cloc_max(),
            self.ploc_min(),
            self.ploc_max(),
            self.lloc_min(),
            self.lloc_max(),
            self.blank_min(),
            self.blank_max(),
        )
    }
}

impl Stats {
    /// Merges a second `Loc` metric suite into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        self.sloc.merge(&other.sloc);
        self.ploc.merge(&other.ploc);
        self.cloc.merge(&other.cloc);
        self.lloc.merge(&other.lloc);

        // Count spaces
        self.space_count += other.space_count;

        // min and max

        self.blank_min = self.blank_min.min(other.blank() as usize);
        self.blank_max = self.blank_max.max(other.blank() as usize);
    }

    /// The `Sloc` metric.
    ///
    /// Counts the number of lines in a scope
    #[inline(always)]
    pub(crate) fn sloc(&self) -> f64 {
        self.sloc.sloc()
    }

    /// The `Ploc` metric.
    ///
    /// Counts the number of instruction lines in a scope
    #[inline(always)]
    pub(crate) fn ploc(&self) -> f64 {
        self.ploc.ploc()
    }

    /// The `Lloc` metric.
    ///
    /// Counts the number of statements in a scope
    #[inline(always)]
    pub(crate) fn lloc(&self) -> f64 {
        self.lloc.lloc()
    }

    /// The `Cloc` metric.
    ///
    /// Counts the number of comments in a scope
    #[inline(always)]
    pub(crate) fn cloc(&self) -> f64 {
        self.cloc.cloc()
    }

    /// The `Blank` metric.
    ///
    /// Counts the number of blank lines in a scope
    #[inline(always)]
    pub(crate) fn blank(&self) -> f64 {
        self.sloc() - self.ploc() - self.cloc.only_comment_lines as f64
    }

    /// The `Sloc` metric average value.
    ///
    /// This value is computed dividing the `Sloc` value for the number of spaces
    #[inline(always)]
    pub(crate) fn sloc_average(&self) -> f64 {
        self.sloc() / self.space_count as f64
    }

    /// The `Ploc` metric average value.
    ///
    /// This value is computed dividing the `Ploc` value for the number of spaces
    #[inline(always)]
    pub(crate) fn ploc_average(&self) -> f64 {
        self.ploc() / self.space_count as f64
    }

    /// The `Lloc` metric average value.
    ///
    /// This value is computed dividing the `Lloc` value for the number of spaces
    #[inline(always)]
    pub(crate) fn lloc_average(&self) -> f64 {
        self.lloc() / self.space_count as f64
    }

    /// The `Cloc` metric average value.
    ///
    /// This value is computed dividing the `Cloc` value for the number of spaces
    #[inline(always)]
    pub(crate) fn cloc_average(&self) -> f64 {
        self.cloc() / self.space_count as f64
    }

    /// The `Blank` metric average value.
    ///
    /// This value is computed dividing the `Blank` value for the number of spaces
    #[inline(always)]
    pub(crate) fn blank_average(&self) -> f64 {
        self.blank() / self.space_count as f64
    }

    /// The `Sloc` metric minimum value.
    #[inline(always)]
    pub(crate) fn sloc_min(&self) -> f64 {
        self.sloc.sloc_min()
    }

    /// The `Sloc` metric maximum value.
    #[inline(always)]
    pub(crate) fn sloc_max(&self) -> f64 {
        self.sloc.sloc_max()
    }

    /// The `Cloc` metric minimum value.
    #[inline(always)]
    pub(crate) fn cloc_min(&self) -> f64 {
        self.cloc.cloc_min()
    }

    /// The `Cloc` metric maximum value.
    #[inline(always)]
    pub(crate) fn cloc_max(&self) -> f64 {
        self.cloc.cloc_max()
    }

    /// The `Ploc` metric minimum value.
    #[inline(always)]
    pub(crate) fn ploc_min(&self) -> f64 {
        self.ploc.ploc_min()
    }

    /// The `Ploc` metric maximum value.
    #[inline(always)]
    pub(crate) fn ploc_max(&self) -> f64 {
        self.ploc.ploc_max()
    }

    /// The `Lloc` metric minimum value.
    #[inline(always)]
    pub(crate) fn lloc_min(&self) -> f64 {
        self.lloc.lloc_min()
    }

    /// The `Lloc` metric maximum value.
    #[inline(always)]
    pub(crate) fn lloc_max(&self) -> f64 {
        self.lloc.lloc_max()
    }

    /// The `Blank` metric minimum value.
    #[inline(always)]
    pub(crate) fn blank_min(&self) -> f64 {
        self.blank_min as f64
    }

    /// The `Blank` metric maximum value.
    #[inline(always)]
    pub(crate) fn blank_max(&self) -> f64 {
        self.blank_max as f64
    }

    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.sloc.compute_minmax();
        self.ploc.compute_minmax();
        self.cloc.compute_minmax();
        self.lloc.compute_minmax();

        if self.blank_min == usize::MAX {
            self.blank_min = self.blank_min.min(self.blank() as usize);
            self.blank_max = self.blank_max.max(self.blank() as usize);
        }
    }
}

pub(crate) trait Loc
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool);
}

#[inline(always)]
fn init(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool) -> (usize, usize) {
    let start = node.start_row();
    let end = node.end_row();

    if is_func_space {
        stats.sloc.start = start;
        stats.sloc.end = end;
        stats.sloc.unit = is_unit;
    }
    (start, end)
}

#[inline(always)]
// Discriminates among the comments that are *after* a code line and
// the ones that are on an independent line.
// This difference is necessary in order to avoid having
// a wrong count for the blank metric.
fn add_cloc_lines(stats: &mut Stats, start: usize, end: usize) {
    let comment_diff = end - start;
    let is_comment_after_code_line = stats.ploc.lines.contains(&start);
    if is_comment_after_code_line && comment_diff == 0 {
        // A comment is *entirely* next to a code line
        stats.cloc.code_comment_lines += 1;
    } else if is_comment_after_code_line && comment_diff > 0 {
        // A block comment that starts next to a code line and ends on
        // independent lines.
        stats.cloc.code_comment_lines += 1;
        stats.cloc.only_comment_lines += comment_diff;
    } else {
        // A comment on an independent line AND
        // a block comment on independent lines OR
        // a comment *before* a code line
        stats.cloc.only_comment_lines += (end - start) + 1;
        // Save line end of a comment to check whether
        // a comment *before* a code line is considered
        stats.cloc.comment_line_end = Some(end);
    }
}

#[inline(always)]
// Detects the comments that are on a code line but *before* the code part.
// This difference is necessary in order to avoid having
// a wrong count for the blank metric.
fn check_comment_ends_on_code_line(stats: &mut Stats, start_code_line: usize) {
    if let Some(end) = stats.cloc.comment_line_end
        && end == start_code_line
        && !stats.ploc.lines.contains(&start_code_line)
    {
        // Comment entirely *before* a code line
        stats.cloc.only_comment_lines -= 1;
        stats.cloc.code_comment_lines += 1;
    }
}

impl Loc for GoCode {
    fn compute(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool) {
        use crate::legacy::languages::Go::*;

        let (start, end) = init(node, stats, is_func_space, is_unit);
        match node.kind_id().into() {
            SourceFile => {}
            Comment => {
                add_cloc_lines(stats, start, end);
            }
            // LLOC: count statements
            ExpressionStatement
            | SendStatement
            | ReceiveStatement
            | IncStatement
            | DecStatement
            | AssignmentStatement
            | ShortVarDeclaration
            | ImportSpec
            | VarSpec
            | ConstSpec
            | TypeSpec
            | EmptyStatement
            | LabeledStatement
            | LabeledStatement2
            | GoStatement
            | DeferStatement
            | ReturnStatement
            | BreakStatement
            | ContinueStatement
            | GotoStatement
            | FallthroughStatement
            | IfStatement
            | ExpressionSwitchStatement
            | TypeSwitchStatement
            | SelectStatement
            | ForStatement => {
                stats.lloc.logical_lines += 1;
            }
            _ => {
                check_comment_ends_on_code_line(stats, start);
                stats.ploc.lines.insert(start);
            }
        }
    }
}

impl Loc for KotlinCode {
    fn compute(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool) {
        use Kotlin::*;

        let (start, end) = init(node, stats, is_func_space, is_unit);

        // LLOC kind-set is informed by SonarKotlin's `StatementsVisitor`,
        // which counts (a) non-declaration block statements and (b) single-
        // expression branches of `if` / `when` / loops as statements. The
        // tree-sitter grammar surfaces these as concrete named kinds; we
        // bump LLOC for those kinds and for declarations (`fun`, `class`,
        // `object`, secondary constructors, properties) as mehen's LLOC is
        // declaration-inclusive, matching the other languages in this
        // crate (e.g. Rust counts `let_declaration` and Ruby counts
        // `method` / `class`).
        // Reference:
        //   sonar-kotlin-metrics/.../StatementsVisitor.kt
        match node.kind_id().into() {
            // Containers and string internals that should not contribute
            // their own physical line.
            SourceFile | Statements | StringLiteral => {}
            LineComment | MultilineComment => {
                add_cloc_lines(stats, start, end);
            }
            FunctionDeclaration | ClassDeclaration | ObjectDeclaration | CompanionObject
            | SecondaryConstructor | PropertyDeclaration | Getter | Setter | Assignment
            | ForStatement | WhileStatement | DoWhileStatement | IfExpression | WhenExpression
            | TryExpression | JumpExpression => {
                stats.lloc.logical_lines += 1;
            }
            // Top-level or statement-position call expressions carry
            // program behavior that isn't already captured by a
            // surrounding declaration; count them once. Nested calls
            // inside an initializer still only contribute PLOC.
            CallExpression
                if node.parent().is_some_and(|parent| {
                    matches!(parent.kind_id().into(), Statements | ControlStructureBody)
                }) =>
            {
                stats.lloc.logical_lines += 1;
            }
            _ => {
                check_comment_ends_on_code_line(stats, start);
                stats.ploc.lines.insert(start);
            }
        }
    }
}

impl Loc for RubyCode {
    fn compute(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool) {
        use Ruby::*;

        let (start, end) = init(node, stats, is_func_space, is_unit);

        match node.kind_id().into() {
            // Containers and string internals that must not be counted as a
            // physical or logical line of their own.
            Program | Statements | BlockBody | BodyStatement | BodyStatement2 | StringContent
            | HeredocContent | HeredocBody | Interpolation | BareString | BareSymbol => {}

            Comment => {
                add_cloc_lines(stats, start, end);
            }

            // LLOC: count statement-like nodes exactly once.
            //
            // These cover Ruby's `_statement` supertype plus the primary
            // statement-ish _expression/_primary forms that carry program
            // behavior. Other expression nodes contribute to PLOC only.
            Method | SingletonMethod | Class | Module | SingletonClass | If | Unless | While
            | Until | For | Case | CaseMatch | Begin | IfModifier | UnlessModifier
            | WhileModifier | UntilModifier | RescueModifier | RescueModifier2
            | RescueModifier3 | Return | Return2 | Break | Break2 | Next | Next2 | Yield
            | Yield2 | Redo | Retry | Alias | Undef | Assignment | Assignment2
            | OperatorAssignment | OperatorAssignment2 | Call | Call2 | Call3 | Call4 => {
                stats.lloc.logical_lines += 1;
            }

            _ => {
                check_comment_ends_on_code_line(stats, start);
                stats.ploc.lines.insert(start);
            }
        }
    }
}

impl Loc for CCode {
    fn compute(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool) {
        use C::*;

        let (start, end) = init(node, stats, is_func_space, is_unit);

        match node.kind_id().into() {
            // Containers and string internals must not contribute their own
            // physical line on their own.
            TranslationUnit | StringLiteral | ConcatenatedString | CharLiteral
            | CompoundStatement | StringContent | EscapeSequence => {}
            Comment => {
                add_cloc_lines(stats, start, end);
            }
            // LLOC: count statement-shaped nodes and declarations exactly
            // once. `type_definition` (typedefs) are declarations like the
            // rest, and preprocessor conditional containers
            // (`#if` / `#ifdef` / `#else` / `#elif` and their variants)
            // each represent a structural directive that contributes one
            // logical line. The tree-sitter-c grammar emits positional
            // duplicate variants (`PreprocIfdef2/3/4`, `PreprocElse2/3/4`,
            // ...) that alias the same rule; each occurrence is one node,
            // so enumerating every variant is safe and complete.
            Declaration | TypeDefinition | ExpressionStatement | ExpressionStatement2
            | IfStatement | SwitchStatement | CaseStatement | WhileStatement | DoStatement
            | ForStatement | ReturnStatement | BreakStatement | ContinueStatement
            | GotoStatement | LabeledStatement | SehTryStatement | SehLeaveStatement
            | FunctionDefinition | FunctionDefinition2 | PreprocInclude | PreprocDef
            | PreprocFunctionDef | PreprocCall | PreprocIf | PreprocIf2 | PreprocIf3
            | PreprocIf4 | PreprocIfdef | PreprocIfdef2 | PreprocIfdef3 | PreprocIfdef4
            | PreprocElse | PreprocElse2 | PreprocElse3 | PreprocElse4 | PreprocElif
            | PreprocElif2 | PreprocElif3 | PreprocElif4 | PreprocElifdef | PreprocElifdef2
            | PreprocElifdef3 | PreprocElifdef4 => {
                stats.lloc.logical_lines += 1;
            }
            _ => {
                check_comment_ends_on_code_line(stats, start);
                stats.ploc.lines.insert(start);
            }
        }
    }
}

impl Loc for crate::legacy::langs::PhpCode {
    fn compute(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool) {
        use crate::legacy::languages::Php::*;

        let (start, end) = init(node, stats, is_func_space, is_unit);

        match node.kind_id().into() {
            // Containers and string internals must not contribute their own
            // physical line.
            Program | CompoundStatement | DeclarationList | StringContent | EncapsedString
            | String | Heredoc | HeredocBody | Nowdoc | NowdocBody => {}
            Comment => {
                add_cloc_lines(stats, start, end);
            }
            // LLOC: count statement-shaped nodes and declarations exactly once.
            ExpressionStatement
            | EmptyStatement
            | EchoStatement
            | UnsetStatement
            | DeclareStatement
            | NamespaceDefinition
            | NamespaceUseDeclaration
            | GlobalDeclaration
            | FunctionStaticDeclaration
            | TryStatement
            | ContinueStatement
            | BreakStatement
            | ReturnStatement
            | WhileStatement
            | DoStatement
            | ForStatement
            | ForeachStatement
            | IfStatement
            | SwitchStatement
            | CaseStatement
            | DefaultStatement
            | NamedLabelStatement
            | GotoStatement
            | FunctionDefinition
            | MethodDeclaration
            | ClassDeclaration
            | TraitDeclaration
            | InterfaceDeclaration
            | EnumDeclaration
            | ConstDeclaration
            | ConstDeclaration2
            | PropertyDeclaration
            | UseDeclaration => {
                stats.lloc.logical_lines += 1;
            }
            _ => {
                check_comment_ends_on_code_line(stats, start);
                stats.ploc.lines.insert(start);
            }
        }
    }
}

// Markdown uses the dedicated `src/markdown/loc.rs` LOC-family pipeline.
// The source-code `Loc` trait is intentionally a no-op here so the generic
// `metrics()` walker produces an empty LOC struct rather than misleading
// prose-as-code counts.
#[cfg(feature = "markdown")]
impl Loc for crate::legacy::langs::MarkdownCode {
    fn compute(_node: &Node, _stats: &mut Stats, _is_func_space: bool, _is_unit: bool) {}
}

#[cfg(test)]
mod tests {
    use crate::legacy::langs::{CParser, GoParser, KotlinParser, RubyParser};
    use crate::legacy::tools::check_metrics;

    #[test]
    fn go_sloc() {
        check_metrics::<GoParser>(
            "package main

            // A comment
            func main() {
                x := 1
            }
            ",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.loc,
                    @r###"
                    {
                      "sloc": 6.0,
                      "ploc": 4.0,
                      "lloc": 1.0,
                      "cloc": 1.0,
                      "blank": 1.0,
                      "sloc_average": 3.0,
                      "ploc_average": 2.0,
                      "lloc_average": 0.5,
                      "cloc_average": 0.5,
                      "blank_average": 0.5,
                      "sloc_min": 3.0,
                      "sloc_max": 3.0,
                      "cloc_min": 0.0,
                      "cloc_max": 0.0,
                      "ploc_min": 3.0,
                      "ploc_max": 3.0,
                      "lloc_min": 1.0,
                      "lloc_max": 1.0,
                      "blank_min": 0.0,
                      "blank_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_lloc() {
        check_metrics::<GoParser>(
            "package main

            func main() {
                x := 1
                y := 2
                if x > y {
                    return
                }
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.loc,
                    @r###"
                    {
                      "sloc": 9.0,
                      "ploc": 8.0,
                      "lloc": 4.0,
                      "cloc": 0.0,
                      "blank": 1.0,
                      "sloc_average": 4.5,
                      "ploc_average": 4.0,
                      "lloc_average": 2.0,
                      "cloc_average": 0.0,
                      "blank_average": 0.5,
                      "sloc_min": 7.0,
                      "sloc_max": 7.0,
                      "cloc_min": 0.0,
                      "cloc_max": 0.0,
                      "ploc_min": 7.0,
                      "ploc_max": 7.0,
                      "lloc_min": 4.0,
                      "lloc_max": 4.0,
                      "blank_min": 0.0,
                      "blank_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_lloc_counts_go_declaration_specs_and_receive_statements() {
        check_metrics::<GoParser>(
            "package main

            import (
                \"fmt\"
                _ \"net/http\"
            )

            var (
                a = 1
                b = 2
            )

            func main(ch chan int) {
            Loop:
                <-ch
                fmt.Println(a, b)
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.loc,
                    @r###"
                    {
                      "sloc": 17.0,
                      "ploc": 14.0,
                      "lloc": 7.0,
                      "cloc": 0.0,
                      "blank": 3.0,
                      "sloc_average": 8.5,
                      "ploc_average": 7.0,
                      "lloc_average": 3.5,
                      "cloc_average": 0.0,
                      "blank_average": 1.5,
                      "sloc_min": 5.0,
                      "sloc_max": 5.0,
                      "cloc_min": 0.0,
                      "cloc_max": 0.0,
                      "ploc_min": 5.0,
                      "ploc_max": 5.0,
                      "lloc_min": 3.0,
                      "lloc_max": 3.0,
                      "blank_min": 0.0,
                      "blank_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_simple_loc() {
        check_metrics::<RubyParser>(
            "# header comment
             def greet(name)
                 puts \"hi, #{name}\"
             end",
            "foo.rb",
            |metric| {
                // Spaces: 2 (unit + method)
                insta::assert_json_snapshot!(
                    metric.loc,
                    @r###"
                    {
                      "sloc": 4.0,
                      "ploc": 3.0,
                      "lloc": 2.0,
                      "cloc": 1.0,
                      "blank": 0.0,
                      "sloc_average": 2.0,
                      "ploc_average": 1.5,
                      "lloc_average": 1.0,
                      "cloc_average": 0.5,
                      "blank_average": 0.0,
                      "sloc_min": 3.0,
                      "sloc_max": 3.0,
                      "cloc_min": 0.0,
                      "cloc_max": 0.0,
                      "ploc_min": 3.0,
                      "ploc_max": 3.0,
                      "lloc_min": 2.0,
                      "lloc_max": 2.0,
                      "blank_min": 0.0,
                      "blank_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_simple_loc() {
        // Inline snapshots don't need hand-counted commentary — the goal
        // here is to lock down the shape so a grammar or Loc-rule change
        // won't silently shift Kotlin's SLOC/PLOC/LLOC.
        check_metrics::<KotlinParser>(
            "// header
             fun greet(name: String) {
                 println(\"hi, \" + name)
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(metric.loc);
            },
        );
    }

    #[test]
    fn kotlin_nested_calls_do_not_add_extra_lloc() {
        check_metrics::<KotlinParser>(
            "fun f() {
                 val x = foo(bar())
                 foo(bar())
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.loc,
                    @r###"
                    {
                      "sloc": 4.0,
                      "ploc": 4.0,
                      "lloc": 3.0,
                      "cloc": 0.0,
                      "blank": 0.0,
                      "sloc_average": 2.0,
                      "ploc_average": 2.0,
                      "lloc_average": 1.5,
                      "cloc_average": 0.0,
                      "blank_average": 0.0,
                      "sloc_min": 4.0,
                      "sloc_max": 4.0,
                      "cloc_min": 0.0,
                      "cloc_max": 0.0,
                      "ploc_min": 4.0,
                      "ploc_max": 4.0,
                      "lloc_min": 3.0,
                      "lloc_max": 3.0,
                      "blank_min": 0.0,
                      "blank_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_counts_companion_and_accessors_as_lloc() {
        check_metrics::<KotlinParser>(
            "class C {
                 companion object {
                     fun make() = C()
                 }

                 var x: Int = 0
                     get() = field
                     set(value) { field = value }
             }",
            "foo.kt",
            |metric| {
                assert_eq!(metric.loc.lloc(), 7.0);
            },
        );
    }

    #[test]
    fn c_typedef_counts_as_lloc() {
        // `typedef` is a declaration like `int x;` and must contribute one
        // logical line. Together with the `int x;` declaration this gives
        // an LLOC of 2.
        check_metrics::<CParser>(
            "typedef unsigned int u32;
int x;",
            "foo.c",
            |metric| {
                assert_eq!(metric.loc.lloc(), 2.0);
            },
        );
    }

    #[test]
    fn c_preproc_conditionals_count_as_lloc() {
        // `#ifdef FOO ... #else ... #endif` exposes two preprocessor
        // conditional containers (`preproc_ifdef` and a nested
        // `preproc_else`). Combined with the two inner `int x = …;`
        // declarations, LLOC must reach 4:
        //   +1 preproc_ifdef  (the `#ifdef FOO` branch)
        //   +1 declaration    (`int x = 1;`)
        //   +1 preproc_else   (the `#else` branch)
        //   +1 declaration    (`int y = 2;`)
        check_metrics::<CParser>(
            "#ifdef FOO
int x = 1;
#else
int y = 2;
#endif",
            "foo.c",
            |metric| {
                assert_eq!(metric.loc.lloc(), 4.0);
            },
        );
    }
}
