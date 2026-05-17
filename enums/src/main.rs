use std::path::PathBuf;

use clap::Parser;
use clap::builder::{PossibleValuesParser, TypedValueParser};

use enums::*;

#[derive(Debug, Clone)]
enum OutputLanguage {
    Rust,
    Go,
    Json,
}

impl std::str::FromStr for OutputLanguage {
    type Err = &'static str;

    fn from_str(env: &str) -> std::result::Result<Self, Self::Err> {
        match env {
            "rust" => Ok(Self::Rust),
            "go" => Ok(Self::Go),
            "json" => Ok(Self::Json),
            _ => Err("Not a valid value, run `--help` to know valid values"),
        }
    }
}

impl OutputLanguage {
    const fn variants() -> [&'static str; 4] {
        ["rust", "go", "json", "c_macros"]
    }
}

#[derive(Parser, Debug)]
#[clap(
    name = "enums",
    version,
    author,
    about = "Generate enums for a target language to use with tree-sitter."
)]
struct Opts {
    /// Output directory. When `--per-crate` is enabled, this is the
    /// workspace root and files land in
    /// `{output}/crates/mehen-{language}/src/grammar.rs`. Otherwise
    /// files are written directly to this directory.
    #[clap(long, short, default_value = ".", value_parser)]
    output: PathBuf,
    /// Target language for the emitted code (Rust by default).
    #[clap(long, short, default_value = "rust", value_parser = PossibleValuesParser::new(OutputLanguage::variants())
        .map(|s| s.parse::<OutputLanguage>().unwrap()))]
    language: OutputLanguage,
    /// File-name template for the legacy single-directory layout.
    /// Ignored when `--per-crate` is set.
    #[clap(long, short, default_value = "language_$")]
    file_template: String,
    /// Write each language's generated kind file to its owning analyzer
    /// crate at `crates/mehen-{lang}/src/grammar.rs`. This is the
    /// rewrite-plan §6.7 layout: kind enums become a private module
    /// inside the owning language crate.
    #[clap(long)]
    per_crate: bool,
}

fn main() {
    let opts = Opts::parse();

    match opts.language {
        OutputLanguage::Rust => {
            let result = if opts.per_crate {
                generate_rust_per_crate(&opts.output)
            } else {
                generate_rust(&opts.output, &opts.file_template)
            };
            if let Some(err) = result.err() {
                eprintln!("{:?}", err);
            }
        }
        OutputLanguage::Go => {
            if let Some(err) = generate_go(&opts.output, &opts.file_template).err() {
                eprintln!("{:?}", err);
            }
        }
        OutputLanguage::Json => {
            if let Some(err) = generate_json(&opts.output, &opts.file_template).err() {
                eprintln!("{:?}", err);
            }
        }
    }
}
