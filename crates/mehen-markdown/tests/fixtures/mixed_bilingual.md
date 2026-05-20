# Bilingual Documentation Sample

このドキュメントは英語と日本語の両方で書かれています。バイリンガルな構成のテスト用サンプルです。

## English Section

The following paragraphs demonstrate how per-block language detection works
across heading boundaries. Each paragraph is classified independently so a
single document can hold multiple locales without losing fidelity.

Software documentation frequently mixes English prose with Japanese commentary,
especially in internationalization guides and migration notes.

## 日本語セクション

この段落は日本語で書かれています。検出器はひらがなとカタカナの比率を確認して、ブロック単位で言語を判定します。

複数のブロックに日本語が含まれている場合でも、各ブロックが独立して分類されるため、適切な読みやすさメトリックを計算できます。

## Shared Conclusions

The mehen analyzer reports both English and Japanese metrics when a document
is bilingual. The overall document-level language is labeled `mixed` and each
block carries its own classification in the output.
