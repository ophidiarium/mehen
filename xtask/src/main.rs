//! `xtask` — developer-only commands for mehen 1.0.
//!
//! Phase 1 scope: define the command surface so the rest of the workspace
//! can call into `cargo xtask` for codegen, parity, and audits. Real
//! implementations land alongside the phase that needs them:
//! - `tree-sitter generate <language>` — wired (per rewrite plan §6.7);
//! - `tree-sitter check-generated` — wired (CI guards drift between the
//!   checked-in `crates/mehen-<lang>/src/grammar.rs` and the grammar
//!   pinned in `xtask/Cargo.toml`);
//! - `ast-dump` — Phase 11;
//! - `metric-contributions` — Phase 11;
//! - `audit-licenses` — Phase 11;
//! - `update-ruff` — Phase 6.

mod tree_sitter;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Mehen developer commands.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Tree-sitter generator commands.
    TreeSitter(TreeSitterArgs),
    /// Dump a parsed AST for debugging.
    AstDump { path: String, language: String },
    /// Print metric contributions for a single file.
    MetricContributions { path: String },
    /// Run a license audit across the workspace.
    AuditLicenses,
    /// Bump the pinned Ruff git revision.
    UpdateRuff { rev: String },
}

#[derive(Debug, Parser)]
struct TreeSitterArgs {
    #[command(subcommand)]
    command: TreeSitterCommand,
}

#[derive(Debug, Subcommand)]
enum TreeSitterCommand {
    /// Regenerate kind enums for one language into the owning crate, or
    /// `--all` to regenerate every language.
    Generate {
        /// Language slug (e.g. `c`, `go`, `kotlin`).
        /// Required unless `--all` is set.
        language: Option<String>,
        /// Regenerate every checked-in `grammar.rs`. Use this after
        /// bumping a pinned grammar version.
        #[arg(long)]
        all: bool,
    },
    /// Verify checked-in generated kind enums match the grammar
    /// revision pinned in `xtask/Cargo.toml`. Exits non-zero on drift.
    /// Wire this into CI so pinned-grammar bumps without a regenerate
    /// are caught at PR time.
    CheckGenerated,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::TreeSitter(args) => match args.command {
            TreeSitterCommand::Generate { language, all } => {
                if let Err(err) = run_generate(language.as_deref(), all) {
                    eprintln!("xtask tree-sitter generate: {err}");
                    std::process::exit(1);
                }
            }
            TreeSitterCommand::CheckGenerated => {
                if let Err(err) = run_check_generated() {
                    eprintln!("xtask tree-sitter check-generated: {err}");
                    std::process::exit(1);
                }
            }
        },
        Command::AstDump { .. }
        | Command::MetricContributions { .. }
        | Command::AuditLicenses
        | Command::UpdateRuff { .. } => {
            eprintln!("xtask command not yet implemented");
            std::process::exit(1);
        }
    }
}

fn run_generate(language: Option<&str>, all: bool) -> Result<(), String> {
    let workspace = tree_sitter::workspace_root().map_err(|e| e.to_string())?;
    let targets: Vec<_> = if all {
        tree_sitter::TARGETS.iter().collect()
    } else {
        let slug = language.ok_or_else(|| {
            "specify a language slug or pass --all (e.g. `xtask tree-sitter generate go`)"
                .to_string()
        })?;
        let target = tree_sitter::target_for(slug).ok_or_else(|| {
            let known = tree_sitter::TARGETS
                .iter()
                .map(|t| t.slug)
                .collect::<Vec<_>>()
                .join(", ");
            format!("unknown language `{slug}`; known: {known}")
        })?;
        vec![target]
    };

    for target in targets {
        let path = tree_sitter::generate(&workspace, target).map_err(|e| e.to_string())?;
        let rel = path
            .strip_prefix(&workspace)
            .unwrap_or(path.as_path())
            .display();
        println!("wrote {rel}");
    }
    Ok(())
}

fn run_check_generated() -> Result<(), String> {
    let workspace = tree_sitter::workspace_root().map_err(|e| e.to_string())?;
    let drifted = tree_sitter::check_generated(&workspace).map_err(|e| e.to_string())?;
    if drifted.is_empty() {
        println!("ok: every grammar.rs matches the pinned grammar revision");
        Ok(())
    } else {
        let names: Vec<_> = drifted.iter().map(|t| t.slug).collect();
        Err(format!(
            "drift detected for: {}. \
             Re-run `cargo xtask tree-sitter generate --all` and commit the result.",
            names.join(", ")
        ))
    }
}
