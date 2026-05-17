//! `xtask` — developer-only commands for mehen 1.0.
//!
//! Phase 1 scope: define the command surface so the rest of the workspace
//! can call into `cargo xtask` for codegen, parity, and audits. Real
//! implementations land alongside the phase that needs them:
//! - `tree-sitter generate <language>` — Phase 6.7;
//! - `tree-sitter check-generated` — Phase 6.7;
//! - `ast-dump` — Phase 11;
//! - `metric-contributions` — Phase 11;
//! - `audit-licenses` — Phase 11;
//! - `update-ruff` — Phase 6.

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
    /// Regenerate kind enums for one language into the owning crate.
    Generate { language: String },
    /// Verify checked-in generated kind enums match the grammar revision.
    CheckGenerated,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::TreeSitter(args) => match args.command {
            TreeSitterCommand::Generate { language } => {
                eprintln!("xtask tree-sitter generate {language}: not yet implemented");
                std::process::exit(1);
            }
            TreeSitterCommand::CheckGenerated => {
                eprintln!("xtask tree-sitter check-generated: not yet implemented");
                std::process::exit(1);
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
