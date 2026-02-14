# mehen

**mehen** is a Rust library to analyze and extract information
from source code written in many different programming languages.
It is based on a parser generator tool and an incremental parsing library
called
<a href="https://tree-sitter.github.io/tree-sitter/" target="_blank">Tree Sitter</a>.


A command line tool called **mehen** is provided to interact with the API of the library in an easy way.

This tool can be used to:

- Call **mehen** API
- Print nodes and metrics information
- Export metrics in different formats

In addition, we provide a **mehen-web** tool to use the library through a REST API.


# Usage

**mehen** computes a variety of software metrics for Go, Python, Rust, and TypeScript/TSX code.

Run `mehen --help` to see all available commands and options.

## Building

To build the `mehen` library, you need to run the following
command:

```console
cargo build
```

If you want to build the `cli`:

```console
cargo build -p mehen-cli
```

If you want to build the `web` server:

```console
cargo build -p mehen-web
```

If you want to build everything in one fell swoop:

```console
cargo build --workspace
```

## Testing

To verify whether all tests pass, run the `cargo test` command.

```console
cargo test --workspace --all-features --verbose
```

### Updating insta tests
We use [insta](https://insta.rs), to update the snapshot tests you should install [cargo insta](https://crates.io/crates/cargo-insta)

``` console
cargo insta test --review
```

Will run the tests, generate the new snapshot references and let you review them.

### Updating grammars

See `mehen-book/src/developers/update-grammars.md` to learn how to update language grammars.

# Contributing

If you want to contribute to the development of this software, please open an issue or pull request on our
[GitHub repository](https://github.com/ophidiarium/mehen). See `mehen-book/src/developers/` for developer documentation.


# License

**mehen**, **mehen-cli** (binary: `mehen`), and **mehen-web** are released under the
<a href="https://www.mozilla.org/MPL/2.0/" target="_blank">Mozilla Public License v2.0</a>.

# Credits

Mehen is based on the excellent [rust-code-analysis](https://github.com/mozilla/rust-code-analysis) project by Mozilla. While mehen has taken a different direction by focusing on a streamlined set of languages (Go, Python, Rust, and TypeScript/TSX), the core architecture and metric implementations are built upon that foundation.

If you use this software in academic work, please cite the original rust-code-analysis paper:

```bibtex
@article{ARDITO2020100635,
    title = {rust-code-analysis: A Rust library to analyze and extract maintainability information from source codes},
    journal = {SoftwareX},
    volume = {12},
    pages = {100635},
    year = {2020},
    issn = {2352-7110},
    doi = {https://doi.org/10.1016/j.softx.2020.100635},
    url = {https://www.sciencedirect.com/science/article/pii/S2352711020303484},
    author = {Luca Ardito and Luca Barbato and Marco Castelluccio and Riccardo Coppola and Calixte Denizet and Sylvestre Ledru and Michele Valsesia},
    keywords = {Algorithm, Software metrics, Software maintainability, Software quality}
}
```

We thank the Mozilla team and all contributors to rust-code-analysis for their foundational work.
