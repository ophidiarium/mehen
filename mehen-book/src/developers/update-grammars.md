# Update grammars

Each programming language needs to be parsed in order to extract its syntax and semantic: the so-called grammar of a language.
In `mehen`, we use [tree-sitter](https://github.com/tree-sitter) as parsing library since it provides a set of distinct grammars for each of our
supported programming languages. Grammars change over time and may have bugs, so they need to be updated periodically.

Grammars can be updated on **Linux** and **macOS** natively, or on **Windows** using **WSL**.

## Updating Grammars

Mehen uses **third-party grammars** published on `crates.io` and maintained by external developers.

### Current Supported Grammars

- `tree-sitter-go` = "=0.23.4"
- `tree-sitter-python` = "=0.23.6"
- `tree-sitter-rust` = "=0.23.2"
- `tree-sitter-typescript` = "=0.23.2"

### Update Process

1. Update the grammar version in both `Cargo.toml` and `enums/Cargo.toml`:

```toml
tree-sitter-go = "=x.xx.x"
```

2. Run the grammar regeneration script:

```bash
./recreate-grammars.sh
```

This script regenerates all language enum files in `src/languages/`.

3. Fix any failing tests or compilation errors introduced by grammar changes.

4. Test thoroughly:

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

5. Commit your changes and create a pull request.
