# Embedded Code Large Fixture

Exercises §9.4 embedded-volume accumulation across supported languages.
Each fence is large enough to move the sqrt-scaled contribution noticeably.

## Rust

```rust
fn collatz(n: u64) -> u64 {
    let mut steps = 0;
    let mut x = n;
    while x > 1 {
        if x % 2 == 0 {
            x /= 2;
        } else {
            x = 3 * x + 1;
        }
        steps += 1;
    }
    steps
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    for a in &args[1..] {
        if let Ok(n) = a.parse::<u64>() {
            println!("{}: {}", n, collatz(n));
        }
    }
}
```

## Python

```python
def fibonacci(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

def main():
    import sys
    for arg in sys.argv[1:]:
        try:
            n = int(arg)
        except ValueError:
            continue
        print(n, fibonacci(n))

if __name__ == "__main__":
    main()
```

## TypeScript

```typescript
interface Shape {
    area(): number;
}

class Circle implements Shape {
    constructor(private radius: number) {}
    area(): number {
        return Math.PI * this.radius * this.radius;
    }
}

class Rectangle implements Shape {
    constructor(private w: number, private h: number) {}
    area(): number {
        return this.w * this.h;
    }
}

function totalArea(shapes: Shape[]): number {
    return shapes.reduce((t, s) => t + s.area(), 0);
}
```

## Unsupported Tag

This fence is ignored by the §9.4 dispatcher because `sql` is not in mehen's
language list — its content still contributes to the `FenceTag` operator,
but not to `embedded_volume`.

```sql
SELECT id, name FROM users WHERE active = 1 ORDER BY id;
```
