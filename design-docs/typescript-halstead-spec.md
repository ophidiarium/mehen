# TypeScript Halstead classification spec (Phase 7)

**Status:** decision record
**Date:** 2026-05-18
**Scope:** `mehen-typescript` Halstead operator/operand classification

## Decision

The Oxc-backed TypeScript / JavaScript / TSX / JSX analyzer treats *pure
type metadata* as Halstead-skipped — neither operator nor operand. This
is a **principled improvement** over the pre-1.0 tree-sitter-typescript
classification, not a parity bug.

## Rationale

Halstead's original definitions:

- **Operator**: a token that "does something" — punctuation, keywords
  that drive control flow, arithmetic / logic operations, function-call
  parens, etc.
- **Operand**: a token that "is something" — identifiers, literals.

TypeScript's type system is purely structural; type annotations are
erased at runtime and do not contribute to the program's executable
mental complexity in the Halstead sense. A function with a `: number`
return annotation is not more complex *to mentally execute* than the
same function without it — the annotation only constrains what the
type-checker accepts.

The pre-1.0 tree-sitter-typescript path counted type-position tokens
inconsistently:

| Token in type position | Pre-1.0 classification | Issue |
|---|---|---|
| `:` (`area(): number`) | operator (anonymous `:` token) | counted |
| `[` `]` (`Shape[]`) | operator | counted |
| `,` (`<T, U>`) | operator | counted |
| `number` keyword | unknown (`predefined_type` wrapper) | skipped |
| `string` keyword | unknown | skipped |
| `Shape` (type identifier) | unknown (`type_identifier`) | skipped |

This split means a TypeScript function with rich type annotations gets a
*higher* operator count than the same function with `any` everywhere —
inflating volume despite the runtime program being unchanged. That's
the opposite of what Halstead aims to capture.

## Spec

The Oxc-backed walker tracks the byte ranges of every TS-only AST
subtree it visits. The post-walk token sweep skips lexer tokens whose
span falls entirely inside any of those ranges. The classified subtrees:

- `TSTypeAnnotation` (every `: T` annotation)
- `TSInterfaceDeclaration` (the entire interface — body, name,
  `extends` clause)
- `TSClassImplements` (the `implements` clause of a class)
- `TSTypeParameterDeclaration` and `TSTypeParameter` (`<T extends U>`)
- `TSTypeParameterInstantiation` (`<T>` at call sites)
- `TSTypeReference`, `TSQualifiedName`
- Specific TS type forms: `TSUnionType`, `TSIntersectionType`,
  `TSArrayType`, `TSTupleType`, `TSConditionalType`,
  `TSIndexedAccessType`, `TSLiteralType`, `TSTypeLiteral`,
  `TSTypeOperator`, `TSParenthesizedType`, `TSFunctionType`,
  `TSConstructorType`, `TSTypePredicate`, `TSTypeQuery`,
  `TSImportType`, `TSMappedType`, `TSInferType`, `TSThisType`,
  `TSTemplateLiteralType`
- All `TS*Keyword` predefined-type nodes (`TSStringKeyword`,
  `TSNumberKeyword`, …)
- TS-only structural nodes: `TSInterfaceBody`, `TSPropertySignature`,
  `TSMethodSignature`, `TSCallSignatureDeclaration`,
  `TSConstructSignatureDeclaration`, `TSIndexSignature`,
  `TSIndexSignatureName`, `TSInterfaceHeritage`

Tokens *outside* any TS-only range are classified normally — class
names, function names, parameter binding identifiers, and runtime
expressions all participate in Halstead.

## Counterexamples preserved

The Phase 7 parity tests in `crates/mehen-typescript/tests/parity.rs`
all reproduce the pre-1.0 snapshot byte-for-byte for fixtures that have
**no TypeScript-specific syntax** (the canonical
`function main() { var a, b, c, avg; … }` case). The drift only
manifests when a fixture contains type annotations, interface
declarations, or `implements` clauses.

## Effect on the embedded TS fence

`crates/mehen-markdown/tests/fixtures/embedded_code_large.md` has a
TypeScript fence that uses `interface Shape`, `class Circle implements
Shape`, parameter properties (`constructor(private radius: number)`),
and `: number` return-type annotations.

| Metric | Pre-1.0 (tree-sitter) | Phase 7 (Oxc) |
|---|---:|---:|
| `n1` (unique operators) | 11 | 10 |
| `N1` (total operators) | 48 | 38 |
| `n2` (unique operands) | 20 | 22 |
| `N2` (total operands) | 35 | 36 |
| `volume` | 411.20 | 370.0 |

Operand count is up by 2 because class/function names that tree-sitter
classified as `type_identifier` (and skipped) are now correctly counted
as runtime bindings. Operator count is down by 10 because type-position
`:`, `[`, `,` punctuation no longer inflates the score.

The markdown-level `embedded_volume` accordingly shifts from 25.954 to
25.746 (`0.20 * sqrt(volume_per_fence)` summed across all fences).

## Re-evaluating in the future

If a future analyzer needs to surface a "type-system complexity"
signal (how rich are the type annotations), it should be a *separate
metric* (e.g. `typescript.type_volume`) rather than overloading the
plain Halstead family. The classical Halstead suite stays a measure of
runtime program complexity.
