//! Command implementations for the 1.0 CLI.

use std::io::{self, Write};

use camino::Utf8PathBuf;

use mehen_core::{AnalysisConfig, Language, SourceFile};
use mehen_engine::{AnalyzeMetricsInput, detect_language};
use mehen_report::render_metrics_json;

use crate::args::{MetricsArgs, OutputFormat};
use crate::exit::ExitCode;

pub fn metrics(args: MetricsArgs) -> ExitCode {
    // Resolve language: explicit flag wins, else extension detection.
    let path = match Utf8PathBuf::try_from(args.path.clone()) {
        Ok(p) => p,
        Err(_) => {
            log::error!("path is not valid UTF-8: {}", args.path.display());
            return ExitCode::SetupError;
        }
    };

    let language = if let Some(lang_str) = args.language.as_deref() {
        match lang_str.parse::<Language>() {
            Ok(l) => l,
            Err(_) => {
                log::error!("unknown --language value: {lang_str}");
                return ExitCode::SetupError;
            }
        }
    } else {
        match detect_language(path.as_path()) {
            Some(l) => l,
            None => {
                log::error!(
                    "could not detect language from path `{path}`; pass --language explicitly"
                );
                return ExitCode::SetupError;
            }
        }
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            log::error!("failed to read `{path}`: {e}");
            return ExitCode::SetupError;
        }
    };

    let source = SourceFile::new(path.clone(), language, text);
    let input = AnalyzeMetricsInput {
        source,
        config: AnalysisConfig::production(),
    };

    let mut report = match mehen_engine::analyze_metrics(input) {
        Ok(r) => r,
        Err(e) => {
            log::error!("analysis failed: {e}");
            return ExitCode::SetupError;
        }
    };
    report.path = path;

    match args.format {
        OutputFormat::Json => match render_metrics_json(&report, args.pretty) {
            Ok(rendered) => {
                let mut stdout = io::stdout().lock();
                if writeln!(stdout, "{rendered}").is_err() {
                    return ExitCode::SerializationError;
                }
                ExitCode::Success
            }
            Err(e) => {
                log::error!("failed to render JSON: {e}");
                ExitCode::SerializationError
            }
        },
        OutputFormat::Markdown => {
            let rendered = mehen_report::render_metrics_markdown(&report);
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "{rendered}").is_err() {
                return ExitCode::SerializationError;
            }
            ExitCode::Success
        }
        OutputFormat::Yaml | OutputFormat::Toml => {
            log::error!(
                "the {:?} format is reserved for a future phase; use --format json or markdown.",
                args.format
            );
            ExitCode::SetupError
        }
    }
}
