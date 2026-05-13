# Supported Metrics

**mehen** implements two metric families: source-code metrics for the
supported programming languages, and documentation metrics for Markdown
files. Both families can be mixed in a single run.

## Markdown (documentation) metrics

Markdown files get a dedicated metric suite that treats code fences,
diagrams, tables, links, math, and images as first-class constructs
instead of stripping them before counting words. The suite is split
across three pages:

- [Markdown Metrics](./metrics/markdown.md) — structural, language-opaque
  layer: LOC family, section tree, MRPC (Markdown Reading Path
  Complexity), MCC (Markdown Cognitive Complexity), Markdown Halstead,
  DMI (Documentation Maintainability Index), Link Debt, Table
  Burden/Scaffold, Visual Scaffold/Net Effect, Artifact Debt, Repository
  Grounding, Evidence Coverage, Filler / Lazy Structure Risk, Review
  Criticality Index, Section Balance, and Good Scaffold.
- [Markdown Prose Metrics](./metrics/markdown-prose.md) — language-aware
  layer: per-block language detection, English readability ensemble
  (Flesch, Flesch-Kincaid, Fog, SMOG, ARI, Coleman-Liau, Dale-Chall,
  FORCAST, LIX/RIX), lexical diversity (MATTR, hapax, density), wording
  quality (passive, hedges, weasels, wordy, adverbs, nominalizations,
  expletives, illusions, cliches, nonwords), inclusive-language flags,
  Japanese script composition, Tateishi simplified readability,
  Jōyō-grade proxy, JTF rule conformance, and a textlint-ja subset.
- [`mehen diff` PR comment](./commands/pr-comment.md) — the design
  spec for the sticky GitHub comment surface that reports Markdown
  metric deltas on pull requests.

## Source-code metrics

- **ABC**: it measures the size of a source code by counting the number of
Assignments (`A`), Branches (`B`) and Conditions (`C`).
- **BLANK**: it counts the number of blank lines in a source file.
- **CC**: it calculates the _Cyclomatic complexity_ examining the
  control flow of a program.
- **CLOC**: it counts the number of comments in a source file.
- **COGNITIVE**: it calculates the _Cognitive complexity_, measuring how complex
it is to understand a unit of code.
- **HALSTEAD**: it is a suite that provides a series of information, such as the
  effort required to maintain the analyzed code, the size in bits to store the
  program, the difficulty to understand the code, an estimate of the number of
  bugs present in the codebase, and an estimate of the time needed to
  implement the software.
- **LLOC**: it counts the number of logical lines (statements) contained in a
source file.
- **MI**: it is a suite that allows to evaluate the maintainability of a software.
- **NARGS**: it counts the number of arguments of a function/method.
- **NEXITS**: it counts the number of possible exit points from a method/function.
- **NOM**: it counts the number of functions and closures in a file/trait/class.
- **NPA**: it counts the number of public attributes in classes/interfaces.
- **NPM**: it counts the number of public methods in classes/interfaces.
- **PLOC**: it counts the number of physical lines (instructions) contained in
a source file.
- **SLOC**: it counts the number of lines in a source file.
- **WMC**: it sums the _Cyclomatic complexity_ of every method defined in a class.
