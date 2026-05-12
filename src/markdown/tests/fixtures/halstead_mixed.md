# Halstead Mixed Fixture

This fixture mixes many operator classes to exercise the Halstead operator
and operand table: headings, list markers, blockquote, link, image, table,
inline code, math, and punctuation.

## Prose

Regular prose. It ends with a period.

## List

- one
- two

## Link And Image

![alt text](./local.png)

See [rust-lang](https://www.rust-lang.org).

## Table

| key | value |
|-----|-------|
| a   | 1     |
| b   | 2     |

## Inline And Math

Use `let x: i32 = 1;` inline. The energy is $E = mc^2$.

## Fence With Language Tag

```python
def foo(x):
    return x * 2
```
