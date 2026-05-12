# Cognitive Cluster Fixture

This fixture triggers MCC penalties in a cluster: unlabelled code, dense
links, nested lists, a blockquote, and an admonition.

## Nested Lists

- item 1
  - sub 1
    - deep 1
    - deep 2
  - sub 2
- item 2

## Blockquote And Callout

> Quoted text that introduces the callout.

> [!WARNING]
> Attention: this flag disables retries.

## Link Cluster

See [one](https://a.example.com), [two](https://b.example.com),
[three](https://c.example.com), [four](https://d.example.com),
[five](https://e.example.com), and [six](https://f.example.com).

## Unlabelled Code

```
echo 'no language tag'
```

## Labelled Code

Here is the same command properly labelled.

```bash
echo 'labelled fence'
```

Runbook closes with a short note.
