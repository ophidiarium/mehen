# Science-Backed Heuristic Metrics for Standalone SQL Files in mehen

**Project:** mehen source code metrics analytics  
**Target module:** proposed `mehen-sql` language analyzer  
**Target inputs:** standalone `.sql` files in software repositories  
**Primary use case:** CI/diff analytics, repository health reporting, and top-offender identification for SQL-heavy codebases  
**Document status:** research foundation and metric design proposal  
**Last updated:** 2026-05-17

---

## 1. Executive summary

SQL should not be squeezed into the existing function/class-centric metric model. A standalone `.sql` file can be an ad hoc query, an analytics model, a migration script, a stored-program body, a DDL package, a transaction script, or a mix of those. The dominant complexity mechanism is usually **relational/dataflow structure** rather than imperative control flow. For PL/SQL and T-SQL procedural blocks, classic cyclomatic/cognitive complexity can still be meaningful, but for ordinary declarative SQL the more useful foundation is a dedicated metric family based on statements, query blocks, CTE graphs, join graphs, predicate/expression structure, output-schema clarity, object-touch risk, dialect portability, parser confidence, and optional lineage.

The most important prior-art observations are:

1. **SonarQube has SQL-family support, but its complexity metric is principally procedural.** Sonar defines cyclomatic complexity as `1 + conditional branches`, reports cognitive complexity, and documents PL/SQL-specific cyclomatic increments for anonymous blocks, procedures, triggers, loops, `WHEN`, `IF`/`ELSIF`, `RAISE`, `AND`/`OR`, and related constructs. It also documents T-SQL analysis and the `.sql` extension ambiguity: by default `.sql` is analyzed as PL/SQL, while `.tsql` is T-SQL. This is useful prior art for procedural SQL, but not sufficient for standalone declarative query complexity. Sources: [Sonar metric definitions](https://docs.sonarsource.com/sonarqube-server/10.7/user-guide/code-metrics/metrics-definition), [Sonar T-SQL](https://docs.sonarsource.com/sonarqube-server/analyzing-source-code/languages/t-sql), [Sonar PL/SQL](https://docs.sonarsource.com/sonarqube-server/analyzing-source-code/languages/pl-sql).
2. **SQLFluff and sqruff provide excellent parsing/linting precedents, but not a general built-in source metric suite.** SQLFluff exposes a dialect-aware parse tree and rule traversal architecture; its rules cover many structural smells that can become metric contributors. Sqruff is a Rust SQL linter/formatter with SQLFluff-inspired rules and experimental column-level lineage support. Sources: [SQLFluff docs](https://docs.sqlfluff.com/en/stable/), [SQLFluff architecture](https://docs.sqlfluff.com/en/stable/guides/contributing/architecture.html), [sqruff docs](https://playground.quary.dev/docs/), [sqruff rules](https://playground.quary.dev/docs/reference/rules/).
3. **A newer SQLFluff plugin is highly relevant practical prior art.** `sqlfluff-complexity` defines CPX rules for CTE count, join count, nested subquery depth, CASE expressions, boolean predicates, window functions, CTE dependency depth, nested CASE depth, set operations, inline derived tables, and an aggregate weighted complexity score. It is not a full repository metrics model, but its feature set is an excellent baseline for mehen’s SQL structural metrics. Sources: [`sqlfluff-complexity` README](https://github.com/yu-iskw/sqlfluff-complexity), [`docs/rules.md`](https://raw.githubusercontent.com/yu-iskw/sqlfluff-complexity/main/docs/rules.md), [`docs/configuration.md`](https://raw.githubusercontent.com/yu-iskw/sqlfluff-complexity/main/docs/configuration.md).
4. **Scientific literature supports SQL-specific query metrics.** Vashistha and Jain’s SQLShare workload work explicitly defines query complexity from the user-authoring perspective, not only optimizer cost; it evaluates number of tables, columns, query length, operators, expression operators, runtime, and adapts Halstead measures to SQL. Piattini and Martínez proposed SQL maintainability measures and validated them empirically. Text-to-SQL research such as Spider classifies SQL hardness using numbers of components, selections, conditions, keywords, set operations, nested subqueries, aggregators, and related features. Sources: [SQLShare paper](https://uwescience.github.io/sqlshare/pdfs/Jain-Vashistha.pdf), [Piattini & Martínez](https://link.springer.com/chapter/10.1007/3-540-44469-6_7), [Spider paper](https://ar5iv.labs.arxiv.org/html/1809.08887), [Spider benchmark site](https://yale-lily.github.io/spider).

Recommended mehen direction:

- Add a **dedicated SQL metric category** rather than mapping everything to `functions`, `classes`, or generic cyclomatic complexity.
- Report SQL metrics at several spaces: `sql.file`, `sql.statement`, `sql.query_block`, `sql.cte`, `sql.object`, and, when procedural dialect support is available, `sql.routine` / `sql.block`.
- Implement a deterministic AST/fact extractor that emits statement facts, query-block facts, CTE dependency graph facts, join graph facts, expression/predicate facts, identifier scope facts, write-risk facts, and parser confidence facts.
- Use composite scores only after exposing raw metrics. The composite scores should be explainable, contributor-backed, and initially treated as review prioritization signals rather than absolute quality judgments.

---

## 2. Context from mehen architecture

The mehen rewrite plan already establishes the right extension model for SQL: metric identifiers/selectors/formulas live in a shared metrics layer, while language crates own language interpretation and emit language-specific contribution evidence. SQL should follow the same principle as Markdown: shared output contracts, language-owned semantics.

For SQL this matters because a syntactic element can have multiple metric meanings depending on dialect and file role. For example, `CASE` in a SELECT list affects expression/cognitive burden; `CASE` in PL/SQL procedural control flow affects cyclomatic complexity; `CREATE TABLE AS SELECT` is both DDL and a query-producing statement; `MERGE` is a write-risk signal; and a `WITH` clause can either improve readability by naming subqueries or hurt readability if it creates a deep dependency chain.

Proposed crate-level architecture:

```text
mehen-sql
  ├── parser_adapter        # sqruff / SQLFluff Rust parser integration boundary
  ├── ast_facts             # dialect-normalized facts; no metric formulas
  ├── scopes                # CTE, table alias, column-reference, outer-reference scopes
  ├── graphs                # CTE dependency graph, join graph, optional lineage graph
  ├── metrics_raw           # raw counts, ratios, depths
  ├── metrics_composite     # weighted explainable scores
  └── reporters             # MetricContribution line ranges and reason codes
```

The metric design below assumes source spans are available from the chosen parser. If a parser node lacks reliable source spans, mehen should still compute file-level metrics, but it should lower confidence for top-offender line attribution.

---

## 3. Prior art review

### 3.1 SonarQube / SonarSource

Sonar’s general metric model includes:

- `complexity`: cyclomatic complexity, described as a quantitative metric for paths through code.
- `cognitive_complexity`: a qualification of how hard code control flow is to understand.
- size metrics such as lines, non-comment lines of code, functions, statements, and comment lines.
- maintainability metrics tied to issues and technical debt.

Sonar documents PL/SQL cyclomatic complexity at function/procedure level and increments it for procedural constructs: the main anonymous block, `CREATE PROCEDURE`, `CREATE TRIGGER`, loops, `WHEN` in CASE, cursor loops, `CONTINUE`/`EXIT WHEN`, exception handlers, `IF`, `ELSIF`, `RAISE`, `AND`, `OR`, and related constructs. That is a useful model for `sql.procedural.*`, but it is not a complete model for ordinary SELECT-heavy files because a SELECT with ten joins and five CTEs may have no imperative branches while still being difficult to review.

Sonar’s T-SQL page also highlights a practical mehen design issue: file suffix alone is not enough. Sonar defaults `.tsql` to T-SQL and `.sql` to PL/SQL, and lets projects override suffixes. mehen should avoid this trap by requiring or inferring a dialect and by exposing dialect confidence.

Sonar’s PL/SQL analyzer can optionally query Oracle data dictionary views through JDBC. That means some official SQL analysis is schema-aware. mehen’s baseline should remain standalone/static, but optional schema/context enrichments should be possible and clearly tagged.

Recommended takeaways:

- Use Sonar’s PL/SQL cyclomatic model as a reference for procedural SQL constructs.
- Do not call declarative query complexity “cyclomatic complexity” unless a control-flow graph exists.
- Treat `.sql` dialect inference as a first-class confidence problem.
- Keep schema-aware signals separate from standalone metrics.

### 3.2 SQLFluff

SQLFluff is an extensible, modular SQL linter. Its architecture is especially relevant because it parses SQL into a tree of segments and traverses that tree to run rules. The architecture stages are templater, lexer, parser, and linter. The parser uses dialect grammars, creates a `FileSegment` containing `StatementSegment`s, and can emit `UnparsableSegment`s when no grammar matches. Rule classes traverse the parse tree and return lint results for matching patterns.

SQLFluff itself is primarily a linter/formatter, but its rules encode practical maintainability judgments. Examples that should influence mehen metric contributors:

- `structure.nested_case`: nested CASE in ELSE can often be flattened.
- `structure.subquery`: subqueries in `FROM`/`JOIN` can often be moved into CTEs.
- `structure.column_order`: order SELECT targets by complexity.
- `structure.unused_cte`: CTE defined but unused.
- `structure.unused_join`: joined table not referenced elsewhere.
- `ambiguous.column_count`: SELECT `*` can hide output shape.
- `ambiguous.join_condition`: implicit cross join.
- `references.qualification`: qualify references when multiple tables are present.

Recommended takeaways:

- Reuse the parser and structural rule vocabulary as inspiration.
- Do not turn lints directly into metrics; convert them into measured contributors such as `sql.select.star_count`, `sql.cte.unused_count`, `sql.join.implicit_cross_count`, and `sql.case.max_depth`.
- Preserve metric neutrality: a high count is descriptive first; thresholding belongs to profiles.

### 3.3 sqruff

Sqruff is a Rust SQL linter and formatter, inspired by SQLFluff and Ruff. Its docs emphasize fast linting/fixing, valid SQL for specific dialects, and experimental SQL column lineage support. Its rule index closely mirrors the SQLFluff-inspired families: aliasing, ambiguity, conventions, layout, references, and structure rules.

Sqruff is especially attractive for mehen because it is Rust-native and already models dialect-specific SQL. The column-level lineage feature is also directly relevant to optional dataflow metrics such as lineage graph width/depth and ambiguous source ratio.

Recommended takeaways:

- Prefer a narrow parser adapter that converts sqruff/SQLFluff nodes into mehen `SqlFact`s rather than coupling metrics to parser-internal node names.
- If column lineage is exposed through a stable API, add optional `sql.lineage.*` metrics. If not, start with CTE/table-level lineage and leave column-level lineage as a vNext enhancement.

### 3.4 SQLFluff complexity plugin

`sqlfluff-complexity` is the most directly relevant current practical prior art. Its CPX rules include:

| Rule | Metric idea | Default threshold |
|---|---:|---:|
| `CPX_C101` | CTE count | 8 |
| `CPX_C102` | Join count | 8 |
| `CPX_C103` | Nested subquery depth | 3 |
| `CPX_C104` | CASE expressions | 10 |
| `CPX_C105` | Boolean AND/OR operators | 20 |
| `CPX_C106` | Window functions | 10 |
| `CPX_C107` | Longest CTE dependency chain | 5 |
| `CPX_C108` | Nested CASE depth | 10 |
| `CPX_C109` | Set operations | 12 |
| `CPX_C110` | Inline derived tables | 4 |
| `CPX_C201` | Aggregate weighted complexity score | 60 |

The plugin also documents an aggregate score as a weighted sum of CTEs, joins, subquery depth, CASE expressions, boolean operators, window functions, CTE dependency depth, set operation count, expression depth / CASE depth, and derived tables.

Recommended takeaways:

- Use CPX metric families as a conservative first compatibility profile, perhaps named `sql.profile.analytics_default`.
- Retain CPX thresholds as soft starting points only. Repositories differ: an analytics warehouse model and a migration script have different complexity budgets.
- mehen should go beyond CPX by adding object-touch risk, DDL/DML risk, output-shape clarity, identifier qualification, dialect portability, parser confidence, and optional lineage.

### 3.5 Scientific and empirical literature

#### 3.5.1 Vashistha & Jain: SQLShare query complexity

Vashistha and Jain’s “Measuring Query Complexity in SQLShare Workload” is a strong foundation because it explicitly frames query complexity as **cognitive load on users authoring SQL**, not just database server cost. They analyzed a high-variety SQLShare workload and considered metrics such as number of tables, number of columns, query length, numbers of operators, expression operators, and runtime. They also adapted Halstead measures to SQL by treating referenced columns as operands and operators/expressions as Halstead operators.

Their paper’s key implications for mehen:

- Query complexity is not equivalent to optimizer runtime.
- Operators and expressions were dominant factors in their analysis.
- Halstead-style SQL metrics are plausible, but the operator/operand taxonomy must be SQL-specific and explicit.
- Linear regression on a small hand-labeled set had limitations, so mehen should expose raw metrics and avoid overclaiming a universal formula.

#### 3.5.2 Piattini & Martínez: SQL maintainability

Piattini and Martínez argued that most software metrics had historically focused on 3GL code while disregarding databases and SQL, and they described three simple measures for SQL code maintainability validated with a student experiment and a real organizational case. The available abstract does not expose the full formulas, but the paper is important prior art because it treats SQL code maintainability as its own measurement subject rather than as an afterthought of host-language metrics.

#### 3.5.3 Siau, Chan & Wei: query complexity and novice users

Siau, Chan, and Wei studied effects of query complexity and learning on novice user query performance. Their results indicate that complex queries affect accuracy, confidence, and time, and that interface abstraction can change how users handle complex queries. This supports the idea that structural SQL burden is partly a human-comprehension problem, not only a database-performance problem.

#### 3.5.4 Taipalus: database complexity and query formulation

Taipalus studied 744 students querying three databases of varying logical complexity and found that increased database complexity lowered success rates and increased unnecessary complications. For mehen, this is a reminder that standalone SQL file metrics are incomplete without schema complexity. Baseline mehen should still be standalone, but any future schema-aware mode should include schema/object graph metrics.

#### 3.5.5 Spider / text-to-SQL hardness criteria

The Spider benchmark divides SQL queries into easy, medium, hard, and extra hard categories based on numbers of SQL components, selections, and conditions. Queries with more SQL keywords such as `GROUP BY`, `ORDER BY`, `INTERSECT`, nested subqueries, selected columns, and aggregators are considered harder. Although Spider is an ML benchmark, its hardness criteria align well with static query complexity features that mehen can calculate.

#### 3.5.6 Subali & Rochimah: SQL command complexity

Subali and Rochimah proposed a model for measuring software complexity that accounts for SQL query attributes in database systems. Their model is described as a five-stage process: reading program modules, forming SQL query models, assigning SQL query weights, calculating SQL complexity, and producing module complexity results. This supports a weighted-attribute approach, but mehen should make each attribute and weight transparent.

#### 3.5.7 Miedema, Fletcher & Aivaloglou: SQL learners and complexity management

The “So many brackets!” ICPC 2022 work analyzes how SQL learners manage or mismanage complexity during query formulation. For mehen, the main relevance is not to model novice errors directly, but to recognize that nested structure, brackets/subqueries, and query formulation complexity are part of program comprehension for SQL.

---

## 4. Design principles for mehen SQL metrics

### 4.1 AST-first, not regex-first

All metrics should be derived from parser facts. Regex can be used only for pre-parse hints, dialect detection hints, or comment/line classification when the parser does not expose trivia. Metric contributors must reference AST-derived constructs whenever possible.

### 4.2 Standalone by default; schema-aware only as an enhancement

Baseline metrics must work without a live database, schema registry, dbt manifest, or query plans. Schema-aware analysis can improve reference resolution, key inference, object blast-radius estimation, and lineage, but it must be opt-in and tagged, for example:

```text
sql.analysis.mode = standalone
sql.analysis.schema_context = none
sql.analysis.confidence.reference_resolution = 0.62
```

### 4.3 Dialect-aware, not `.sql`-extension-aware

A `.sql` suffix is ambiguous. mehen should accept an explicit dialect and optionally auto-detect a dialect with confidence. Dialect auto-detection should remain conservative: better to report `sql.dialect.confidence.low` than to misclassify T-SQL as PL/SQL or Snowflake as ANSI.

### 4.4 Statement-first, not function-first

For declarative SQL, the primary analysis spaces should be:

```text
sql.file
sql.batch
sql.statement
sql.query_block
sql.cte
sql.object
```

Only procedural dialect constructs should create `sql.routine` or `sql.block` spaces.

### 4.5 Separate descriptive metrics from prescriptive rules

Metrics answer “what is present?” Rules answer “is this acceptable?” For example:

- Metric: `sql.select.star_count = 3`
- Contributor: `select_star_in_outer_query` at line 42
- Rule/profile decision: fail only if `select_star_count > 0` in strict production profile.

### 4.6 Complexity is review burden, not query performance

Unless mehen consumes query plans, it should not claim to estimate runtime cost. Static metrics such as joins, subqueries, predicates, and functions can correlate with review burden and sometimes performance risk, but they are not optimizer cost.

### 4.7 Every composite score must be explainable

For top-offender reporting, a composite score should include the exact contributing factors, weights, and line ranges:

```text
sql.cognitive_complexity = 67
contributors:
  +12  cte_dependency_depth=6       lines 1-88
  +10  correlated_subquery_count=2  lines 42-61
  +9   boolean_operator_count=18    lines 73-77
  +8   join_count=8                 lines 18-39
```

### 4.8 Profile thresholds should be calibrated

Default thresholds should be starting points. mehen should support repository percentiles, historical deltas, and profile-specific gates. Absolute thresholds for migration scripts, analytics models, and stored procedures should differ.

---

## 5. Proposed SQL fact model

The metric layer should not depend on parser-internal node classes. It should depend on normalized facts.

### 5.1 File facts

```rust
struct SqlFileFacts {
    dialect_requested: Option<SqlDialect>,
    dialect_inferred: Option<SqlDialect>,
    dialect_confidence: f32,
    source_lines: LineMap,
    statements: Vec<SqlStatementFacts>,
    parser_diagnostics: Vec<SqlParserDiagnostic>,
    comments: Vec<SqlComment>,
    templating_tokens: Vec<SqlTemplatingToken>,
}
```

### 5.2 Statement facts

```rust
struct SqlStatementFacts {
    id: StatementId,
    kind: SqlStatementKind,
    span: SourceSpan,
    query: Option<SqlQueryFacts>,
    dml: Option<SqlDmlFacts>,
    ddl: Option<SqlDdlFacts>,
    dcl: Option<SqlDclFacts>,
    tcl: Option<SqlTransactionFacts>,
    procedural: Option<SqlProceduralFacts>,
}
```

Suggested `SqlStatementKind` values:

```text
select
with_select
insert_values
insert_select
update
delete
merge
create_view
create_table
create_table_as
create_materialized_view
alter_table
drop
truncate
grant
revoke
begin_transaction
commit
rollback
explain
procedure_or_function
anonymous_block
unknown
```

### 5.3 Query-block facts

A query block is a SELECT-like relational unit: a `SELECT` core, a CTE body, a subquery, a branch of a set operation, or a SELECT inside `INSERT ... SELECT` / `CREATE TABLE AS SELECT`.

```rust
struct SqlQueryBlockFacts {
    id: QueryBlockId,
    span: SourceSpan,
    nesting_depth: u32,
    select_items: Vec<SelectItemFacts>,
    from_items: Vec<RelationRefFacts>,
    joins: Vec<JoinFacts>,
    predicates: Vec<PredicateFacts>,
    group_by: Option<GroupByFacts>,
    having: Option<PredicateFacts>,
    windows: Vec<WindowFacts>,
    order_by: Option<OrderByFacts>,
    limit_offset: Option<LimitOffsetFacts>,
    subqueries: Vec<SubqueryFacts>,
    set_ops: Vec<SetOperationFacts>,
}
```

### 5.4 Scope and identifier facts

```rust
struct SqlScopeFacts {
    cte_defs: Vec<CteDefFacts>,
    relation_aliases: Vec<RelationAliasFacts>,
    column_refs: Vec<ColumnRefFacts>,
    unresolved_refs: Vec<IdentifierRefFacts>,
    outer_refs: Vec<ColumnRefFacts>,
    wildcard_refs: Vec<WildcardRefFacts>,
}
```

The most valuable standalone resolution is not perfect schema resolution; it is scope resolution:

- Which names are CTEs?
- Which names are relation aliases?
- Which subqueries reference outer aliases?
- Which SELECT items are derived expressions lacking aliases?
- Which references are unqualified in multi-relation scopes?

### 5.5 Graph facts

```rust
struct SqlGraphFacts {
    cte_graph: DirectedGraph<CteId>,
    relation_join_graphs: Vec<JoinGraph>,
    object_read_write_graph: ObjectTouchGraph,
    column_lineage_graph: Option<ColumnLineageGraph>,
}
```

Graph metrics should be computed after query facts and scope facts are available.

---

## 6. Metric namespaces and raw metric catalogue

The following keys are intentionally explicit. They can later be shortened or grouped by selectors, but the first implementation should optimize clarity and grepability.

### 6.1 Line and size metrics

| Metric key | Type | Definition | Notes |
|---|---:|---|---|
| `sql.loc.physical` | int | Physical lines in file | Includes comments/blanks. |
| `sql.loc.code` | int | Lines containing SQL code tokens | Excludes pure comment/blank lines. |
| `sql.loc.comment` | int | Lines containing SQL comments | `--`, `/* ... */`, dialect comments. |
| `sql.loc.blank` | int | Blank/whitespace-only lines | Raw line map. |
| `sql.loc.logical` | int | Logical SQL statements | Usually AST statement count, not semicolon count. |
| `sql.loc.comment_density` | float | `comment / max(1, code + comment)` | Useful but not a quality score. |
| `sql.loc.max_statement_lines` | int | Max code-span length of any statement | Top-offender-friendly. |
| `sql.loc.avg_statement_lines` | float | Mean statement span length | Report with median if possible. |

SQL-specific caveat: semicolons are not reliable statement separators in all dialects and contexts; T-SQL batches may use `GO`, PL/SQL blocks may contain semicolons inside procedural bodies, and some tools omit terminators. Prefer parser statements.

### 6.2 Statement composition metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.statement.count` | int | Number of top-level statements. |
| `sql.statement.batch_count` | int | Number of parser-recognized batches or batch separators. |
| `sql.statement.kind_count.<kind>` | int | Count by normalized statement kind. |
| `sql.statement.kind_distinct` | int | Number of statement kinds present. |
| `sql.statement.kind_entropy` | float | Normalized Shannon entropy over statement kinds. |
| `sql.statement.max_complexity` | float | Max per-statement composite score. |
| `sql.statement.unparsed_count` | int | Top-level statements with parser failure/unknown kind. |

`kind_entropy` is useful for mixed migration scripts. A file containing only `CREATE TABLE` statements has low entropy; a file mixing DDL, DML, transactions, grants, functions, and queries has higher operational complexity.

Formula:

```text
H = -Σ p(kind) * log2(p(kind))
sql.statement.kind_entropy = H / log2(max(2, distinct_kind_count))
```

### 6.3 Query-block metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.query_block.count` | int | Count of SELECT-like query blocks. |
| `sql.query_block.max_depth` | int | Maximum query-block nesting depth. |
| `sql.query_block.avg_select_items` | float | Mean SELECT item count per query block. |
| `sql.query_block.max_select_items` | int | Maximum SELECT item count. |
| `sql.query_block.max_clause_count` | int | Max number of major clauses in a query block. |
| `sql.query_block.with_clause_count` | int | Number of WITH clauses. |

Major clauses include SELECT, FROM, WHERE, GROUP BY, HAVING, WINDOW/QUALIFY where applicable, ORDER BY, LIMIT/OFFSET/FETCH, set operation, and dialect-specific clauses such as PIVOT/UNPIVOT, CONNECT BY, MODEL, SAMPLE, QUALIFY.

### 6.4 CTE metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.cte.count` | int | Number of CTE definitions. |
| `sql.cte.recursive_count` | int | Recursive CTEs. |
| `sql.cte.dependency_edges` | int | Edges from one CTE to another referenced CTE. |
| `sql.cte.max_dependency_depth` | int | Longest CTE dependency chain. |
| `sql.cte.max_fan_in` | int | Max number of upstream CTEs referenced by a CTE. |
| `sql.cte.max_fan_out` | int | Max number of downstream CTEs using a CTE. |
| `sql.cte.unused_count` | int | CTEs defined but not used by final query or downstream CTEs. |
| `sql.cte.trivial_count` | int | CTEs that only rename/select from one source with no filtering/aggregation/join. |
| `sql.cte.shadowed_name_count` | int | CTE names shadowing relation aliases or repeated names in nested scopes. |
| `sql.cte.avg_body_complexity` | float | Mean structural score of CTE query bodies. |

CTEs should not be treated as automatically good or bad. A CTE can improve reviewability by naming a concept; too many CTEs or a deep dependency chain can make dataflow hard to trace.

### 6.5 Join and relation graph metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.relation.ref_count` | int | Relation references, including base tables, views, CTE refs, derived tables. |
| `sql.relation.base_ref_count` | int | Base table/view refs, excluding CTEs and derived tables where known. |
| `sql.relation.distinct_object_count` | int | Distinct named objects touched for reads. |
| `sql.join.count` | int | Explicit join clauses. |
| `sql.join.kind_count.<kind>` | int | Join count by kind: inner, left, right, full, cross, natural, lateral, apply, implicit. |
| `sql.join.outer_count` | int | LEFT/RIGHT/FULL joins. |
| `sql.join.cross_count` | int | CROSS joins and implicit cross joins. |
| `sql.join.natural_count` | int | NATURAL joins. |
| `sql.join.non_equi_count` | int | Join predicates without equality between relation columns. |
| `sql.join.complex_condition_count` | int | Join conditions with boolean depth above threshold. |
| `sql.join.missing_condition_count` | int | Joins lacking ON/USING where required or implicit comma joins. |
| `sql.join.graph_node_count` | int | Nodes in relation join graph. |
| `sql.join.graph_edge_count` | int | Edges in relation join graph. |
| `sql.join.graph_component_count` | int | Connected components. |
| `sql.join.graph_surplus_edges` | int | `max(0, E - N + C)` over relation graph. |
| `sql.join.self_join_count` | int | Same base object referenced multiple times in one scope. |

Join graph construction should use relation aliases as nodes and join predicates as edges. Without schema, foreign-key semantics are unknown; the graph is about syntactic/review topology, not relational correctness.

### 6.6 Subquery and scope metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.subquery.count` | int | All nested SELECT-like subqueries. |
| `sql.subquery.max_depth` | int | Maximum nested subquery depth. |
| `sql.subquery.correlated_count` | int | Subqueries with references to outer query scopes. |
| `sql.subquery.scalar_count` | int | Scalar subqueries in SELECT/predicate expressions. |
| `sql.subquery.exists_count` | int | `EXISTS` / `NOT EXISTS` subqueries. |
| `sql.subquery.in_count` | int | `IN (SELECT ...)` subqueries. |
| `sql.derived_table.count` | int | Inline subqueries in FROM/JOIN. |
| `sql.derived_table.max_depth` | int | Nested derived-table depth. |

Correlated subqueries deserve separate weight: they combine nested query structure with outer scope coupling. They may be the strongest single standalone signal of high comprehension burden after deep CTE chains and large join graphs.

### 6.7 Predicate and boolean logic metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.predicate.count` | int | WHERE, HAVING, ON, QUALIFY, CHECK, FILTER predicates. |
| `sql.predicate.boolean_operator_count` | int | Count of `AND`/`OR` boolean operators. |
| `sql.predicate.max_boolean_depth` | int | Max nesting depth of boolean expression tree. |
| `sql.predicate.max_or_chain_length` | int | Max OR chain length. |
| `sql.predicate.not_count` | int | `NOT` operators. |
| `sql.predicate.comparison_count` | int | Equality, inequality, range, LIKE, IN, BETWEEN, etc. |
| `sql.predicate.in_list_max_length` | int | Longest literal/value IN list. |
| `sql.predicate.null_semantics_risk_count` | int | `NOT IN`, `= NULL`, `<> NULL`, or dialect-risky NULL logic. |
| `sql.predicate.sargability_risk_count` | int | Function/cast/arithmetic on column side, leading wildcard LIKE, regex, etc. |
| `sql.predicate.mixed_and_or_without_grouping_count` | int | Boolean chains where precedence may be non-obvious. |

`sql.predicate.sargability_risk_count` is not a performance prediction; it is a static risk indicator. A function on a column in a predicate may be appropriate, but it is worth surfacing for review.

### 6.8 CASE and conditional expression metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.case.count` | int | CASE expressions/statements. |
| `sql.case.max_depth` | int | Max nested CASE depth. |
| `sql.case.when_count` | int | Total WHEN arms. |
| `sql.case.max_when_count` | int | Maximum WHEN arms in a single CASE. |
| `sql.case.missing_else_count` | int | CASE expressions without ELSE. |
| `sql.case.nested_in_else_count` | int | Nested CASE inside ELSE branch. |
| `sql.case.condition_complexity_max` | int | Max predicate complexity inside WHEN conditions. |

CASE is one of the clearest bridges between declarative SQL and cognitive control-flow burden.

### 6.9 Aggregation and grouping metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.aggregate.function_count` | int | Aggregate function calls. |
| `sql.aggregate.distinct_count` | int | Aggregate calls with DISTINCT. |
| `sql.group_by.count` | int | GROUP BY clauses. |
| `sql.group_by.max_expression_count` | int | Max grouping expressions in one clause. |
| `sql.group_by.rollup_count` | int | ROLLUP usage. |
| `sql.group_by.cube_count` | int | CUBE usage. |
| `sql.group_by.grouping_sets_count` | int | GROUPING SETS usage. |
| `sql.having.count` | int | HAVING clauses. |
| `sql.distinct.count` | int | SELECT DISTINCT occurrences. |

Grouping modifiers can significantly increase semantic burden because they change output cardinality and subtotal semantics.

### 6.10 Window function metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.window.function_count` | int | Window function calls with `OVER`. |
| `sql.window.distinct_spec_count` | int | Distinct window specifications. |
| `sql.window.repeated_inline_spec_count` | int | Repeated inline specs that could be named/reused where dialect allows. |
| `sql.window.partition_expression_count` | int | Total PARTITION BY expressions. |
| `sql.window.order_expression_count` | int | Total ORDER BY expressions inside windows. |
| `sql.window.frame_count` | int | Explicit window frames. |
| `sql.window.max_spec_complexity` | int | Max weighted partition/order/frame complexity. |
| `sql.window.rank_function_count` | int | RANK/DENSE_RANK/ROW_NUMBER/NTILE etc. |
| `sql.window.percentile_function_count` | int | Percentile/distribution functions. |

Window functions often look compact but require reviewers to reason about partitions, order, frame semantics, and interaction with query-level grouping.

### 6.11 Set operation metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.set_op.count` | int | UNION, INTERSECT, EXCEPT/MINUS operations. |
| `sql.set_op.kind_count.<kind>` | int | Count by set operation kind. |
| `sql.set_op.max_depth` | int | Nested set-expression depth. |
| `sql.set_op.union_all_ratio` | float | UNION ALL / UNION total. |
| `sql.set_op.distinct_count` | int | Set operations with duplicate elimination semantics. |
| `sql.set_op.branch_count_max` | int | Maximum number of branches in one set expression. |

Set operations affect both output shape and duplicate semantics. `UNION` without explicit `ALL` or `DISTINCT` is also an ambiguity/style smell in some dialects.

### 6.12 Expression and function-call metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.expression.count` | int | Count of non-trivial expressions. |
| `sql.expression.max_depth` | int | Max expression AST depth. |
| `sql.expression.avg_depth` | float | Mean non-trivial expression depth. |
| `sql.expression.operator_count` | int | Arithmetic/string/comparison/special operators in expressions. |
| `sql.function.call_count` | int | Function calls. |
| `sql.function.distinct_count` | int | Distinct function names. |
| `sql.function.nested_call_depth` | int | Max nested function-call depth. |
| `sql.cast.count` | int | Casts and dialect cast operators. |
| `sql.json_path.count` | int | JSON/path operators/functions. |
| `sql.regex.count` | int | Regex predicates/functions. |
| `sql.literal.count` | int | Literal values. |
| `sql.literal.long_string_count` | int | String literals over configurable threshold. |

Expression depth and function nesting are especially important in SELECT lists, CASE arms, predicates, and ORDER BY clauses.

### 6.13 Output-shape and readability metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.select.star_count` | int | `*` or `table.*` projections. |
| `sql.select.outer_star_count` | int | Wildcards in outermost query blocks. |
| `sql.select.expression_without_alias_count` | int | Derived SELECT expressions without alias. |
| `sql.select.output_alias_coverage` | float | Aliased derived expressions / derived expressions. |
| `sql.identifier.unqualified_column_ratio` | float | Unqualified refs in multi-relation scopes / all column refs. |
| `sql.identifier.quoted_count` | int | Quoted identifiers. |
| `sql.identifier.keyword_identifier_count` | int | Identifiers that are reserved/keyword-like. |
| `sql.identifier.ordinal_reference_count` | int | ORDER BY/GROUP BY ordinal references. |
| `sql.alias.table_alias_count` | int | Table aliases. |
| `sql.alias.short_alias_count` | int | Aliases shorter than threshold. |
| `sql.alias.reused_count` | int | Reused aliases in overlapping scopes. |
| `sql.alias.unused_count` | int | Aliases defined but unused. |
| `sql.name.length_mean` | float | Mean identifier length for aliases and output columns. |

These metrics should be interpreted as reviewability signals, not universal style rules. Short aliases such as `o` and `c` can be acceptable in small queries; they become painful in large join graphs.

### 6.14 Object-touch and migration risk metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.object.read_count` | int | Distinct objects read. |
| `sql.object.write_count` | int | Distinct objects written/created/altered/dropped. |
| `sql.object.touch_count` | int | Distinct read or write objects. |
| `sql.object.temp_count` | int | Temporary/transient objects. |
| `sql.object.schema_qualified_ratio` | float | Schema-qualified object refs / object refs. |
| `sql.dml.insert_count` | int | INSERT statements. |
| `sql.dml.update_count` | int | UPDATE statements. |
| `sql.dml.delete_count` | int | DELETE statements. |
| `sql.dml.merge_count` | int | MERGE statements. |
| `sql.dml.update_without_where_count` | int | UPDATE with no WHERE / limited predicate. |
| `sql.dml.delete_without_where_count` | int | DELETE with no WHERE / limited predicate. |
| `sql.dml.returning_count` | int | RETURNING/OUTPUT clauses. |
| `sql.ddl.create_count` | int | CREATE statements. |
| `sql.ddl.alter_count` | int | ALTER statements. |
| `sql.ddl.drop_count` | int | DROP statements. |
| `sql.ddl.truncate_count` | int | TRUNCATE statements. |
| `sql.ddl.create_or_replace_count` | int | CREATE OR REPLACE statements. |
| `sql.dcl.grant_revoke_count` | int | GRANT/REVOKE statements. |
| `sql.transaction.control_count` | int | BEGIN/COMMIT/ROLLBACK/SAVEPOINT etc. |
| `sql.dynamic_sql.count` | int | EXECUTE IMMEDIATE / dynamic SQL constructs. |

This family is essential for standalone `.sql` files because migration risk can be much more important than query-expression complexity. A three-line `DROP TABLE` script is structurally simple but operationally critical.

### 6.15 Dialect and portability metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.dialect.requested` | enum | Dialect configured by user/project. |
| `sql.dialect.inferred` | enum | Best inferred dialect, if any. |
| `sql.dialect.confidence` | float | 0..1 confidence in inferred dialect. |
| `sql.dialect.feature_count` | int | Recognized dialect-specific features. |
| `sql.dialect.feature_count.<feature>` | int | Counts for `qualify`, `top`, `limit`, `connect_by`, `pivot`, `unpivot`, `lateral`, `apply`, `json`, arrays, etc. |
| `sql.dialect.portability_risk_count` | int | Features outside ANSI/core profile. |
| `sql.dialect.conflict_count` | int | Hints pointing to multiple dialects. |

This metric family is especially useful in multi-engine repositories or libraries that claim portability.

### 6.16 Parser health and analysis confidence metrics

| Metric key | Type | Definition |
|---|---:|---|
| `sql.parser.diagnostic_count` | int | Parser diagnostics. |
| `sql.parser.unparsable_segment_count` | int | Unparsable AST segments. |
| `sql.parser.unparsable_line_count` | int | Lines touched by unparsable segments. |
| `sql.parser.unparsable_ratio` | float | Unparsable code lines / code lines. |
| `sql.parser.recovery_count` | int | Parser recovery events, if available. |
| `sql.analysis.confidence.syntax` | float | Syntax analysis confidence. |
| `sql.analysis.confidence.scope` | float | Scope/reference resolution confidence. |
| `sql.analysis.confidence.line_spans` | float | Source attribution confidence. |
| `sql.templating.token_count` | int | Jinja/placeholders/template tokens. |
| `sql.templating.unresolved_count` | int | Template constructs not resolved to SQL. |

Standalone SQL often contains templating, variables, placeholders, or dialect features that parsers only partially understand. Confidence metrics prevent false precision.

### 6.17 Procedural SQL metrics

These should apply only when the parser recognizes PL/SQL, T-SQL, or another procedural dialect block.

| Metric key | Type | Definition |
|---|---:|---|
| `sql.procedural.block_count` | int | Anonymous/procedural blocks. |
| `sql.procedural.routine_count` | int | Procedures/functions/triggers. |
| `sql.procedural.cyclomatic_complexity` | int | Control-flow complexity using dialect rules. |
| `sql.procedural.cognitive_complexity` | int | Cognitive flow complexity for procedural blocks. |
| `sql.procedural.max_block_depth` | int | Nested procedural block depth. |
| `sql.procedural.loop_count` | int | Loops/cursors. |
| `sql.procedural.if_count` | int | IF/ELSIF branches. |
| `sql.procedural.case_statement_count` | int | CASE statements in control flow. |
| `sql.procedural.exception_handler_count` | int | Exception/catch handlers. |
| `sql.procedural.return_count` | int | Return statements. |
| `sql.procedural.raise_throw_count` | int | Raise/throw statements. |
| `sql.procedural.dynamic_sql_count` | int | Dynamic SQL in procedural code. |

For PL/SQL, Sonar’s documented increments are a useful starting point. For T-SQL, mehen should define a separate dialect table, because T-SQL has `TRY/CATCH`, `WHILE`, cursor constructs, `GOTO`, `RETURN`, `THROW`, and batch semantics.

---

## 7. SQL Halstead metrics

Halstead metrics are attractive for SQL because many SQL queries have rich symbolic structure without imperative branches. Radon’s general Halstead definitions use:

```text
η1 = number of distinct operators
η2 = number of distinct operands
N1 = total operators
N2 = total operands
η  = η1 + η2
N  = N1 + N2
V  = N * log2(η)
D  = (η1 / 2) * (N2 / η2)
E  = D * V
```

Vashistha and Jain adapted Halstead to SQLShare queries by treating referenced columns as operands and operators/expressions as Halstead operators. mehen should define a more complete, deterministic SQL operator/operand taxonomy.

### 7.1 Proposed SQL operator classes

Operators should include:

1. **Statement verbs:** `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `MERGE`, `CREATE`, `ALTER`, `DROP`, `TRUNCATE`, `GRANT`, `REVOKE`, `COMMIT`, `ROLLBACK`.
2. **Clause operators:** `WITH`, `FROM`, `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`, `OFFSET`, `FETCH`, `QUALIFY`, `CONNECT BY`, `START WITH`, `RETURNING`.
3. **Join operators:** `JOIN`, `INNER JOIN`, `LEFT JOIN`, `RIGHT JOIN`, `FULL JOIN`, `CROSS JOIN`, `NATURAL JOIN`, `LATERAL`, `APPLY`, `ON`, `USING`.
4. **Set operators:** `UNION`, `UNION ALL`, `INTERSECT`, `EXCEPT`, `MINUS`.
5. **Predicate operators:** `AND`, `OR`, `NOT`, `=`, `<>`, `!=`, `<`, `<=`, `>`, `>=`, `LIKE`, `ILIKE`, `SIMILAR TO`, `IN`, `BETWEEN`, `IS NULL`, `IS DISTINCT FROM`, `EXISTS`.
6. **Expression operators:** arithmetic, concatenation, JSON/path operators, array operators, casts, collations.
7. **Conditional operators:** `CASE`, `WHEN`, `THEN`, `ELSE`, `END`, dialect conditional functions if treated as built-ins.
8. **Aggregate/window operators:** aggregate function names, `OVER`, `PARTITION BY`, window `ORDER BY`, frame keywords.
9. **Function operators:** scalar function names, including dialect-specific built-ins.
10. **DDL type/constraint operators:** `PRIMARY KEY`, `FOREIGN KEY`, `UNIQUE`, `CHECK`, `DEFAULT`, `NOT NULL`, `REFERENCES`, `INDEX`, data type constructors.

### 7.2 Proposed SQL operand classes

Operands should include:

1. Table/view/materialized view names.
2. CTE names.
3. Table aliases.
4. Column names and qualified column references.
5. Output aliases.
6. Literal values.
7. Bind parameters and placeholders.
8. Data types and sizes in DDL/casts.
9. Constraint/index names.
10. Schema/database names.

### 7.3 Metric keys

| Metric key | Type | Definition |
|---|---:|---|
| `sql.halstead.distinct_operators` | int | `η1` |
| `sql.halstead.distinct_operands` | int | `η2` |
| `sql.halstead.total_operators` | int | `N1` |
| `sql.halstead.total_operands` | int | `N2` |
| `sql.halstead.vocabulary` | int | `η1 + η2` |
| `sql.halstead.length` | int | `N1 + N2` |
| `sql.halstead.volume` | float | `N * log2(η)` |
| `sql.halstead.difficulty` | float | `(η1 / 2) * (N2 / η2)` |
| `sql.halstead.effort` | float | `D * V` |

### 7.4 Implementation cautions

- Do not mix parser trivia with operator counts; comments and whitespace are not Halstead operators.
- Decide whether `AS` is an operator. Recommended: include it only when it creates an alias, not for dialect-noise appearances.
- Normalize equivalent operator spellings by dialect profile, for example `!=` and `<>` can be the same logical operator if the profile wants semantic normalization.
- Function names should be operators, not operands, because they transform operands.
- Aliases are operands because reviewers must track them; optionally distinguish alias operands from base object operands.

---

## 8. Composite metrics

Composite metrics are useful for ranking files and statements, but dangerous if opaque. mehen should report raw metrics first and composites second.

### 8.1 SQL Structural Complexity (`sql.structural_complexity`)

Purpose: a simple weighted syntactic complexity score for SELECT-like SQL. This is closest to `sqlfluff-complexity` CPX_C201 but expanded slightly.

Suggested initial formula per statement/query:

```text
SSC =
  1.00 * sql.query_block.count
+ 0.80 * sql.cte.count
+ 1.20 * sql.cte.max_dependency_depth
+ 1.00 * sql.join.count
+ 0.80 * sql.join.outer_count
+ 2.00 * sql.join.cross_count
+ 1.50 * sql.subquery.count
+ 1.25 * sql.subquery.max_depth
+ 2.00 * sql.subquery.correlated_count
+ 0.35 * sql.predicate.boolean_operator_count
+ 1.00 * sql.predicate.max_boolean_depth
+ 0.80 * sql.case.count
+ 0.80 * sql.case.max_depth
+ 0.60 * sql.window.function_count
+ 0.35 * sql.aggregate.function_count
+ 1.00 * sql.set_op.count
+ 0.50 * sql.expression.max_depth
+ 0.50 * sql.derived_table.count
```

Interpretation:

- Low: simple query/script section.
- Medium: normal analytics query or migration statement.
- High: likely review bottleneck.

Do not fail CI on this score initially. Use it to rank top offenders and calibrate thresholds.

### 8.2 SQL Cognitive Complexity (`sql.cognitive_complexity`)

Purpose: human comprehension burden. This should mimic the spirit of cognitive complexity, but use SQL-specific mental contexts rather than only imperative branches.

Suggested scoring rules:

1. Add `+1` for each query block.
2. Add `+nesting_depth` for subqueries and derived tables.
3. Add `+2` for each correlated subquery.
4. Add `+1` for each CTE and `+1` for each CTE dependency edge after the first edge in a chain.
5. Add `+1` for each join; add an extra `+1` for outer joins, non-equi joins, cross joins, natural joins, or APPLY/LATERAL joins.
6. Add `+1` for each CASE, plus `+1` for each nested CASE level, plus `+0.25` per WHEN arm beyond two.
7. Add `+0.25` per boolean operator, plus `+1` per boolean nesting level beyond two, plus `+1` for mixed AND/OR chains without explicit grouping.
8. Add `+0.5` per window function, plus `+1` for explicit frames.
9. Add `+0.5` per set operation, plus `+1` for nested set expressions.
10. Add `+0.25` per non-trivial SELECT expression without alias.
11. Add `+0.5` per wildcard projection in an outer query.
12. Subtract a small modularization credit, capped at `-5`, for CTEs that reduce derived-table nesting and have shallow dependency depth.

Formula sketch:

```text
SCC = max(0,
  query_context_points
+ relational_reasoning_points
+ nested_scope_points
+ predicate_reasoning_points
+ expression_reasoning_points
+ output_shape_points
- modularization_credit
)
```

This score should be computed per statement and per file as sum/max. Report both:

- `sql.cognitive_complexity.sum`
- `sql.cognitive_complexity.max_statement`

### 8.3 SQL Review Burden Index (`sql.review_burden_index`)

Purpose: rank SQL files/statements by likely PR review effort.

Suggested file-level formula:

```text
norm(x, t) = x / (x + t)

RBI = 100 * clamp01(
  0.30 * norm(sql.cognitive_complexity.sum, 60)
+ 0.18 * norm(sql.structural_complexity.sum, 80)
+ 0.14 * norm(sql.object.touch_count, 20)
+ 0.12 * norm(sql.change_risk_score, 25)
+ 0.10 * norm(sql.halstead.volume, 1500)
+ 0.08 * norm(sql.parser.diagnostic_count, 5)
+ 0.05 * norm(sql.dialect.portability_risk_count, 20)
+ 0.05 * norm(sql.loc.code, 300)
- 0.02 * clamp01(sql.loc.comment_density / 0.20)
)
```

The comment-density credit is intentionally tiny. Comments can help, but they should not hide severe structural risk.

### 8.4 SQL Change Risk Score (`sql.change_risk_score`)

Purpose: operational risk in migrations and deployment scripts.

Suggested scoring:

```text
CRS =
  8 * drop_count
+ 8 * truncate_count
+ 6 * alter_table_count
+ 6 * delete_without_where_count
+ 6 * update_without_where_count
+ 5 * grant_revoke_count
+ 5 * dynamic_sql_count
+ 4 * merge_count
+ 4 * create_or_replace_count
+ 3 * transaction_control_count
+ 2 * write_object_count
+ 1 * read_object_count
```

This score should have separate profiles. A migration repository will naturally have DDL, so the goal is not “zero DDL”; the goal is to surface risky or broad changes.

### 8.5 SQL Maintainability Index (`sql.maintainability_index`)

Classic Maintainability Index uses SLOC, cyclomatic complexity, and Halstead volume. Radon documents common formulas and warns that MI is experimental. For SQL, classic MI is not directly appropriate because ordinary declarative queries may have little or no cyclomatic complexity.

Recommended: do not call this classic MI. Use **SQL Maintainability Index** as a mehen-specific normalized score:

```text
SQL_MI = 100 * clamp01(1 - risk)

risk =
  0.22 * norm(sql.halstead.volume, 1500)
+ 0.22 * norm(sql.cognitive_complexity.sum, 60)
+ 0.16 * norm(sql.structural_complexity.sum, 80)
+ 0.12 * norm(sql.predicate.boolean_operator_count, 30)
+ 0.10 * norm(sql.cte.max_dependency_depth, 6)
+ 0.08 * norm(sql.subquery.max_depth, 4)
+ 0.05 * norm(sql.parser.diagnostic_count, 5)
+ 0.05 * norm(sql.dialect.portability_risk_count, 20)
```

Interpretation:

- `80..100`: likely easy to maintain/review.
- `60..79`: normal complexity; inspect top contributors.
- `40..59`: high review burden.
- `<40`: likely refactoring or decomposition candidate.

This should be marked experimental until calibrated on real repositories.

### 8.6 SQL Modularity Health (`sql.modularity_health`)

Purpose: avoid simplistic “more CTEs are good” or “fewer CTEs are good” conclusions.

Components:

```text
cte_use_ratio        = used_cte_count / max(1, cte_count)
cte_shallow_score    = 1 - norm(max_dependency_depth, 6)
cte_fanout_score     = 1 - norm(max_fan_out, 8)
derived_table_penalty = norm(derived_table.count, 5)
trivial_cte_penalty   = trivial_cte_count / max(1, cte_count)

modularity_health = 100 * clamp01(
  0.35 * cte_use_ratio
+ 0.25 * cte_shallow_score
+ 0.15 * cte_fanout_score
+ 0.15 * (1 - derived_table_penalty)
+ 0.10 * (1 - trivial_cte_penalty)
)
```

This metric is only meaningful for query-like files with CTEs/subqueries. It should be omitted or marked N/A for pure DDL migrations.

### 8.7 Optional lineage metrics

If sqruff exposes stable column-level lineage or mehen later implements it, add:

| Metric key | Type | Definition |
|---|---:|---|
| `sql.lineage.node_count` | int | Nodes in column lineage graph. |
| `sql.lineage.edge_count` | int | Edges in column lineage graph. |
| `sql.lineage.max_derivation_depth` | int | Longest derivation chain for output columns. |
| `sql.lineage.max_fan_in` | int | Max number of input columns feeding one output column. |
| `sql.lineage.max_fan_out` | int | Max number of downstream outputs using one input. |
| `sql.lineage.ambiguous_source_ratio` | float | Output columns with unresolved/ambiguous source lineage. |

Lineage metrics are especially valuable for analytics SQL because reviewers often ask, “Where did this output column come from?”

---

## 9. Suggested profiles and thresholds

Thresholds should be configurable by profile. The defaults below are initial seeds, not universal truth.

### 9.1 `sql.analytics_default`

For dbt-like models, warehouse transformations, views, reports, analytical SELECT scripts.

| Metric | Soft warning | Strong warning | Source/inspiration |
|---|---:|---:|---|
| `sql.cte.count` | >8 | >12 | `sqlfluff-complexity` CPX_C101 default 8 |
| `sql.join.count` | >8 | >12 | CPX_C102 default 8 |
| `sql.subquery.max_depth` | >3 | >5 | CPX_C103 default 3 |
| `sql.case.count` | >10 | >16 | CPX_C104 default 10 |
| `sql.predicate.boolean_operator_count` | >20 | >35 | CPX_C105 default 20 |
| `sql.window.function_count` | >10 | >16 | CPX_C106 default 10 |
| `sql.cte.max_dependency_depth` | >5 | >8 | CPX_C107 default 5 |
| `sql.case.max_depth` | >3 | >5 | stricter than CPX_C108 because 10 is very high for reviewability |
| `sql.set_op.count` | >12 | >20 | CPX_C109 default 12 |
| `sql.derived_table.count` | >4 | >8 | CPX_C110 default 4 |
| `sql.structural_complexity` | >60 | >100 | CPX_C201 default 60, adjusted by calibration |

### 9.2 `sql.migration_default`

For schema migrations, deployment scripts, seed scripts.

Primary signals:

- `sql.change_risk_score`
- `sql.object.write_count`
- `sql.ddl.drop_count`
- `sql.ddl.truncate_count`
- `sql.dml.update_without_where_count`
- `sql.dml.delete_without_where_count`
- `sql.transaction.control_count`
- `sql.parser.diagnostic_count`

Suggested gates:

| Metric | Warning | Critical |
|---|---:|---:|
| `sql.dml.update_without_where_count` | >0 | >0 with no transaction boundary |
| `sql.dml.delete_without_where_count` | >0 | >0 with no transaction boundary |
| `sql.ddl.drop_count` | >0 | >2 or affects non-temp objects |
| `sql.ddl.truncate_count` | >0 | >0 in production paths |
| `sql.object.write_count` | >10 | >25 |
| `sql.change_risk_score` | >25 | >60 |
| `sql.parser.unparsable_ratio` | >0.02 | >0.10 |

### 9.3 `sql.procedural_default`

For PL/SQL/T-SQL procedures, functions, triggers, anonymous blocks.

Primary signals:

- `sql.procedural.cyclomatic_complexity`
- `sql.procedural.cognitive_complexity`
- `sql.procedural.max_block_depth`
- `sql.procedural.exception_handler_count`
- `sql.procedural.dynamic_sql_count`
- embedded query structural complexity.

Suggested gates:

| Metric | Warning | Critical |
|---|---:|---:|
| `sql.procedural.cyclomatic_complexity` | >10 | >20 |
| `sql.procedural.cognitive_complexity` | >15 | >30 |
| `sql.procedural.max_block_depth` | >4 | >6 |
| `sql.procedural.dynamic_sql_count` | >0 | >3 |
| `sql.structural_complexity.max_embedded_query` | >60 | >100 |

### 9.4 Repository-calibrated profile

mehen should support percentile-driven thresholds:

```text
warn if metric > p90(repository_baseline)
critical if metric > p97(repository_baseline)
warn on diff if metric_delta > max(absolute_delta, percentage_delta)
```

This is likely better than universal thresholds for mature projects.

---

## 10. Metric contributors and top-offender examples

A metric without contributors is hard to act on. Suggested contributor reason codes:

```text
sql.cte.definition
sql.cte.dependency_edge
sql.cte.unused
sql.join.inner
sql.join.outer
sql.join.cross
sql.join.non_equi
sql.join.missing_condition
sql.subquery.nested
sql.subquery.correlated
sql.derived_table.inline
sql.predicate.boolean_operator
sql.predicate.deep_boolean_tree
sql.predicate.null_semantics_risk
sql.predicate.sargability_risk
sql.case.expression
sql.case.nested
sql.window.function
sql.window.frame
sql.aggregate.function
sql.set_op
sql.select.star
sql.select.expression_without_alias
sql.identifier.unqualified_column
sql.dml.update_without_where
sql.dml.delete_without_where
sql.ddl.drop
sql.ddl.truncate
sql.dynamic_sql
sql.parser.unparsable_segment
```

Example top-offender output:

```text
models/revenue_rollup.sql
  sql.review_burden_index: 84.2
  sql.cognitive_complexity.sum: 91
  sql.structural_complexity.sum: 116

  contributors:
    +18  sql.cte.max_dependency_depth=9          lines 1-136
    +15  sql.join.count=15                       lines 42-83
    +12  sql.window.function_count=14            lines 91-119
    +10  sql.predicate.boolean_operator_count=28 lines 55-61
    +8   sql.subquery.correlated_count=2         lines 122-133
```

---

## 11. Dialect strategy

### 11.1 Dialect selection

Recommended priority:

1. Explicit CLI/config setting: `--sql-dialect postgres`.
2. Project config mapping by path: `migrations/** = postgres`, `warehouse/snowflake/** = snowflake`.
3. Parser-supported dialect inference from syntax hints.
4. Conservative fallback: `ansi` with low confidence.

### 11.2 Dialect inference hints

Examples:

| Hint | Likely dialect(s) |
|---|---|
| `GO` batch separator, `TOP`, `CROSS APPLY` | T-SQL |
| `QUALIFY`, `IFF`, `::`, `COPY INTO` | Snowflake-ish, with ambiguity |
| `::` casts, `DISTINCT ON`, `ILIKE` | PostgreSQL-ish |
| backtick identifiers, `STRUCT`, `UNNEST` | BigQuery-ish |
| `CONNECT BY`, `MINUS`, PL/SQL blocks | Oracle/PLSQL |
| `LIMIT` | PostgreSQL/MySQL/SQLite/DuckDB/others; weak hint |

Dialect inference should be advisory, not hidden. Always expose `sql.dialect.confidence`.

### 11.3 Dialect-specific metric tables

Some constructs should map to common metric concepts:

| Common concept | Examples |
|---|---|
| row limiting | `LIMIT`, `FETCH FIRST`, `TOP` |
| lateral relation | `LATERAL`, `CROSS APPLY`, `OUTER APPLY` |
| set difference | `EXCEPT`, `MINUS` |
| conditional function | `IFF`, `IF`, `DECODE`, `NVL2` |
| null handling | `COALESCE`, `NVL`, `IFNULL` |
| temporary table | `#temp`, `CREATE TEMP TABLE`, `CREATE TEMPORARY TABLE` |

Common metrics should use normalized concepts while also retaining dialect-specific counts.

---

## 12. Implementation plan

### Phase 1: Parser adapter and raw metrics

Deliver:

- Dialect selection/configuration.
- Parse diagnostics and confidence metrics.
- Statement count and statement kind classification.
- LOC/comment/blank/code metrics.
- Query-block count/depth.
- CTE count and dependency graph.
- Join count/kind metrics.
- Subquery and derived-table metrics.
- CASE, boolean predicate, window, aggregate, set-operation counts.
- SELECT `*`, missing alias, unqualified column ratio.
- Basic DDL/DML risk metrics.
- SQL Halstead counts.

This phase is enough to produce valuable top-offender output.

### Phase 2: Composite scores and profiles

Deliver:

- `sql.structural_complexity`.
- `sql.cognitive_complexity`.
- `sql.review_burden_index`.
- `sql.change_risk_score`.
- `sql.maintainability_index`.
- `sql.modularity_health`.
- Profile-based thresholds.
- Diff-aware deltas.

### Phase 3: Procedural SQL

Deliver:

- PL/SQL and T-SQL procedural block detection.
- Procedural cyclomatic/cognitive complexity.
- Exception/cursor/loop/dynamic-SQL metrics.
- Embedded query complexity attribution inside routines.

### Phase 4: Optional schema and lineage enrichments

Deliver:

- Optional schema catalog input.
- More accurate object/column reference resolution.
- Foreign-key-aware join graph classification.
- Optional sqruff lineage integration or mehen lineage implementation.
- Schema blast-radius metrics.

---

## 13. JSON output sketch

```json
{
  "language": "sql",
  "dialect": {
    "requested": "postgres",
    "inferred": "postgres",
    "confidence": 0.91
  },
  "analysis_mode": "standalone",
  "metrics": {
    "sql.loc.code": 184,
    "sql.statement.count": 3,
    "sql.query_block.count": 12,
    "sql.cte.count": 9,
    "sql.cte.max_dependency_depth": 6,
    "sql.join.count": 11,
    "sql.subquery.correlated_count": 1,
    "sql.predicate.boolean_operator_count": 24,
    "sql.window.function_count": 7,
    "sql.structural_complexity.sum": 78.5,
    "sql.cognitive_complexity.sum": 69,
    "sql.review_burden_index": 72.4
  },
  "spaces": [
    {
      "kind": "sql.statement",
      "name": "statement#1 SELECT",
      "span": { "start_line": 1, "end_line": 144 },
      "metrics": {
        "sql.cte.count": 9,
        "sql.join.count": 11,
        "sql.cognitive_complexity": 69
      },
      "contributors": [
        {
          "metric": "sql.cognitive_complexity",
          "reason": "sql.cte.dependency_edge",
          "delta": 8,
          "span": { "start_line": 1, "end_line": 78 }
        }
      ]
    }
  ]
}
```

---

## 14. Validation strategy

### 14.1 Golden fixtures

Build fixtures by dialect and file role:

```text
fixtures/sql/ansi/simple_select.sql
fixtures/sql/postgres/cte_chain.sql
fixtures/sql/postgres/correlated_subquery.sql
fixtures/sql/snowflake/qualify_windows.sql
fixtures/sql/bigquery/unnest_struct.sql
fixtures/sql/tsql/procedure_control_flow.sql
fixtures/sql/plsql/anonymous_block.sql
fixtures/sql/migration/destructive_ddl.sql
fixtures/sql/migration/safe_idempotent.sql
```

Each fixture should assert raw metrics and contributors.

### 14.2 Prior-art compatibility tests

Create a fixture suite for CPX-equivalent metrics:

- CTE count.
- Join count.
- Nested subquery depth.
- CASE count.
- Boolean operator count.
- Window function count.
- CTE dependency depth.
- Nested CASE depth.
- Set operation count.
- Inline derived table count.

The goal is not to clone `sqlfluff-complexity`, but to ensure mehen’s metric interpretations are explainable when they differ.

### 14.3 Repository calibration

Use several real repositories:

- migration-heavy project.
- dbt/analytics project.
- app project with embedded standalone SQL files.
- PL/SQL/T-SQL stored procedure project.

Measure distributions and tune default weights only after raw metrics are stable.

### 14.4 Human validation

Ask reviewers to rank sampled SQL files by expected review effort. Compare rankings against:

- `sql.structural_complexity`.
- `sql.cognitive_complexity`.
- `sql.review_burden_index`.
- SQL Halstead volume/difficulty.
- LOC alone.

The SQLShare paper used hand-labeled query complexity and found useful signal in operators/expressions; mehen should replicate that style of validation on repository SQL.

---

## 15. Recommended first metric set for mehen 1.x SQL support

For a high-value first release, implement these metrics before composites:

```text
sql.loc.physical
sql.loc.code
sql.loc.comment
sql.loc.blank
sql.statement.count
sql.statement.kind_count.*
sql.query_block.count
sql.query_block.max_depth
sql.cte.count
sql.cte.max_dependency_depth
sql.cte.unused_count
sql.join.count
sql.join.kind_count.*
sql.join.cross_count
sql.join.non_equi_count
sql.subquery.count
sql.subquery.max_depth
sql.subquery.correlated_count
sql.derived_table.count
sql.predicate.boolean_operator_count
sql.predicate.max_boolean_depth
sql.case.count
sql.case.max_depth
sql.window.function_count
sql.aggregate.function_count
sql.set_op.count
sql.expression.max_depth
sql.function.call_count
sql.select.star_count
sql.select.expression_without_alias_count
sql.identifier.unqualified_column_ratio
sql.object.read_count
sql.object.write_count
sql.dml.update_without_where_count
sql.dml.delete_without_where_count
sql.ddl.drop_count
sql.ddl.truncate_count
sql.transaction.control_count
sql.parser.diagnostic_count
sql.parser.unparsable_ratio
sql.halstead.volume
sql.halstead.difficulty
```

Then add:

```text
sql.structural_complexity
sql.cognitive_complexity
sql.change_risk_score
sql.review_burden_index
sql.maintainability_index
```

---

## 16. Open questions and decisions to make

1. **Parser API stability:** sqruff is attractive because it is Rust-native, but mehen should verify the stability of its AST/source-span API before hard coupling.
2. **Dialect coverage:** choose a supported dialect subset for initial implementation. Recommended: `ansi`, `postgres`, `sqlite`, `mysql`, `tsql`, `oracle/plsql`, `snowflake`, `bigquery`, `duckdb` if parser support is mature enough.
3. **Templating stance:** this document targets standalone `.sql`, but real repositories often contain placeholders and Jinja. Decide whether mehen initially reports templating burden only or invokes SQLFluff/sqruff templaters.
4. **Threshold philosophy:** decide whether first release ships only raw metrics and top offenders, or also composite warnings.
5. **Procedural SQL boundary:** decide whether PL/SQL/T-SQL routines are part of initial SQL support or a separate milestone.
6. **Lineage dependency:** decide whether column lineage is optional enrichment or a core metric family.

---

## 17. Conclusions

SQL support should introduce a new `sql.*` metric namespace with SQL-specific structural, cognitive, object-risk, and confidence metrics. The strongest first implementation is not a single “SQL complexity” number; it is a layered model:

1. Raw AST-derived metrics.
2. Graph metrics for CTEs, joins, and optionally lineage.
3. SQL Halstead metrics with explicit operator/operand taxonomy.
4. Risk metrics for DDL/DML and migration scripts.
5. Procedural complexity only for dialects that actually contain procedural control flow.
6. Explainable composite scores for review prioritization.

This approach aligns with mehen’s language-owned metric model, builds on SQLFluff/sqruff parser infrastructure, incorporates current linter prior art, and follows the scientific literature’s main lesson: SQL complexity is a human comprehension and authoring burden as much as, and often more than, an execution-cost problem.

---

## 18. References

- mehen rewrite plan: <https://github.com/ophidiarium/mehen/blob/v1/docs/mehen-1-0-from-scratch-rewrite-plan.md>
- mehen Markdown metrics research foundation: <https://github.com/ophidiarium/mehen/blob/v1/docs/mehen_markdown_metrics_research_foundation.md>
- SonarQube Server metric definitions: <https://docs.sonarsource.com/sonarqube-server/10.7/user-guide/code-metrics/metrics-definition>
- SonarQube T-SQL language page: <https://docs.sonarsource.com/sonarqube-server/analyzing-source-code/languages/t-sql>
- SonarQube PL/SQL language page: <https://docs.sonarsource.com/sonarqube-server/analyzing-source-code/languages/pl-sql>
- Sonar Cognitive Complexity overview: <https://www.sonarsource.com/resources/cognitive-complexity/>
- SQLFluff docs: <https://docs.sqlfluff.com/en/stable/>
- SQLFluff architecture: <https://docs.sqlfluff.com/en/stable/guides/contributing/architecture.html>
- SQLFluff rules reference: <https://docs.sqlfluff.com/en/stable/reference/rules.html>
- sqruff docs: <https://playground.quary.dev/docs/>
- sqruff dialects: <https://playground.quary.dev/docs/dialects/>
- sqruff rules: <https://playground.quary.dev/docs/reference/rules/>
- `sqlfluff-complexity`: <https://github.com/yu-iskw/sqlfluff-complexity>
- `sqlfluff-complexity` rules: <https://raw.githubusercontent.com/yu-iskw/sqlfluff-complexity/main/docs/rules.md>
- `sqlfluff-complexity` configuration: <https://raw.githubusercontent.com/yu-iskw/sqlfluff-complexity/main/docs/configuration.md>
- Vashistha, A. & Jain, S. “Measuring Query Complexity in SQLShare Workload”: <https://uwescience.github.io/sqlshare/pdfs/Jain-Vashistha.pdf>
- Piattini, M. & Martínez, A. “Measuring for Database Programs Maintainability”: <https://link.springer.com/chapter/10.1007/3-540-44469-6_7>
- Siau, K. L., Chan, H. C., & Wei, K. K. “Effects of Query Complexity and Learning on Novice User Query Performance With Conceptual and Logical Database Interfaces”: <https://doi.org/10.1109/TSMCA.2003.820581>
- Taipalus, T. “The effects of database complexity on SQL query formulation”: <https://doi.org/10.1016/j.jss.2020.110576>
- Yu et al. “Spider: A Large-Scale Human-Labeled Dataset for Complex and Cross-Domain Semantic Parsing and Text-to-SQL Task”: <https://ar5iv.labs.arxiv.org/html/1809.08887>
- Spider benchmark site: <https://yale-lily.github.io/spider>
- Subali, M. A. P. & Rochimah, S. “A new model for measuring the complexity of SQL commands”: <https://scholar.its.ac.id/en/publications/a-new-model-for-measuring-the-complexity-of-sql-commands/>
- Miedema, D., Fletcher, G., & Aivaloglou, E. “So many brackets!: an analysis of how SQL learners (mis)manage complexity during query formulation”: <https://doi.org/10.1145/3524610.3529158>
- Radon code metrics documentation for Maintainability Index and Halstead formulas: <https://radon.readthedocs.io/en/latest/intro.html>
