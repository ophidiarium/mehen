use askama::Template;
use std::env;
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
