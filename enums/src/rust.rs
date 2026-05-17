use askama::Template;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::common::*;
use crate::languages::*;

#[derive(Debug, Template)]
#[template(path = "rust.rs", escape = "none")]
struct RustTemplate {
    c_name: String,
    names: Vec<(String, bool, String)>,
}

pub fn generate_rust(output: &Path, file_template: &str) -> std::io::Result<()> {
    for lang in Lang::into_enum_iter() {
        let language = get_language(&lang);
        let name = get_language_name(&lang);
        let c_name = camel_case(name.to_string());

        let file_name = format!("{}.rs", file_template.replace('$', &c_name.to_lowercase()));
        let path = output.join(file_name);
        let mut file = File::create(path)?;

        let mut names = get_token_names(&language, false);
        if c_name == "Rust" {
            for (token_name, _is_duplicate, ts_name) in &mut names {
                if token_name == "Pub" && ts_name == "pub(crate)" {
                    *ts_name = "pub".to_string();
                }
            }
        }

        let args = RustTemplate { c_name, names };

        file.write_all(args.render().unwrap().as_bytes())?;
    }

    Ok(())
}

/// Per rewrite-plan §6.7: write each language's kind enum into the
/// owning analyzer crate at `{workspace}/crates/mehen-{slug}/src/grammar.rs`.
///
/// The TypeScript and TSX flavors share `mehen-typescript`, so the
/// generator writes both files into that crate (`grammar.rs` for
/// TypeScript and `grammar_tsx.rs` for TSX) instead of overwriting one
/// with the other.
pub fn generate_rust_per_crate(workspace: &Path) -> std::io::Result<()> {
    for lang in Lang::into_enum_iter() {
        let language = get_language(&lang);
        let name = get_language_name(&lang);
        let c_name = camel_case(name.to_string());

        let (slug, file_name) = crate_target(&c_name);
        let crate_dir = workspace
            .join("crates")
            .join(format!("mehen-{slug}"))
            .join("src");
        std::fs::create_dir_all(&crate_dir)?;
        let path = crate_dir.join(file_name);
        let mut file = File::create(&path)?;

        let mut names = get_token_names(&language, false);
        if c_name == "Rust" {
            for (token_name, _is_duplicate, ts_name) in &mut names {
                if token_name == "Pub" && ts_name == "pub(crate)" {
                    *ts_name = "pub".to_string();
                }
            }
        }

        let args = RustTemplate { c_name, names };
        file.write_all(args.render().unwrap().as_bytes())?;
    }

    Ok(())
}

/// Map a generated `c_name` (e.g. `Tsx`, `Powershell`, `Markdown`) to
/// the workspace `(crate-slug, file-name)` pair.
fn crate_target(c_name: &str) -> (&'static str, &'static str) {
    match c_name {
        "Tsx" => ("typescript", "grammar_tsx.rs"),
        "Typescript" => ("typescript", "grammar.rs"),
        "Powershell" => ("powershell", "grammar.rs"),
        "Python" => ("python", "grammar.rs"),
        "Rust" => ("rust", "grammar.rs"),
        "Go" => ("go", "grammar.rs"),
        "Ruby" => ("ruby", "grammar.rs"),
        "Kotlin" => ("kotlin", "grammar.rs"),
        "C" => ("c", "grammar.rs"),
        "Php" => ("php", "grammar.rs"),
        "Markdown" => ("markdown", "grammar.rs"),
        other => panic!("no per-crate mapping for language `{other}`"),
    }
}
