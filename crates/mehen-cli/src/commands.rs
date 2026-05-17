//! Command implementations for the 1.0 CLI.

use std::io::{self, Write};

use camino::Utf8PathBuf;

use mehen_core::{AnalysisConfig, Language, SourceFile};
use mehen_engine::{
    AnalyzeMetricsInput, DiffInput, TopOffendersInput, analyze_diff, analyze_metrics,
    detect_language, rank_top_offenders,
};
use mehen_metrics::MetricSelector;
use mehen_report::{render_diff_json, render_metrics_json};

use crate::args::{
    DiffArgs, DiffFormat, MetricsArgs, OutputFormat, TopOffendersArgs, TopOffendersFormat,
};
use crate::exit::ExitCode;

pub fn metrics(args: MetricsArgs) -> ExitCode {
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

    let source = SourceFile::new(path, language, text);
    let input = AnalyzeMetricsInput {
        source,
        config: AnalysisConfig::production(),
    };

    let report = match analyze_metrics(input) {
        Ok(r) => r,
        Err(e) => {
            log::error!("analysis failed: {e}");
            return ExitCode::SetupError;
        }
    };

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

pub fn diff(args: DiffArgs) -> ExitCode {
    let paths: Vec<Utf8PathBuf> = match args
        .paths
        .into_iter()
        .map(Utf8PathBuf::try_from)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(p) => p,
        Err(e) => {
            log::error!("path is not valid UTF-8: {e}");
            return ExitCode::SetupError;
        }
    };

    let input = DiffInput {
        from: args.from,
        to: args.to,
        paths,
        thresholds: Vec::new(),
        config: AnalysisConfig::production(),
    };

    let report = match analyze_diff(input) {
        Ok(r) => r,
        Err(e) => {
            log::error!("diff failed: {e}");
            return ExitCode::SetupError;
        }
    };

    match args.format {
        DiffFormat::Json => match render_diff_json(&report, true) {
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
        DiffFormat::GithubMarkdown => {
            let rendered = mehen_report::render_diff_github_markdown(&report);
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "{rendered}").is_err() {
                return ExitCode::SerializationError;
            }
            ExitCode::Success
        }
    }
}

pub fn top_offenders(args: TopOffendersArgs) -> ExitCode {
    let paths: Vec<Utf8PathBuf> = match args
        .paths
        .into_iter()
        .map(Utf8PathBuf::try_from)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(p) => p,
        Err(e) => {
            log::error!("path is not valid UTF-8: {e}");
            return ExitCode::SetupError;
        }
    };

    let selectors: Vec<MetricSelector> = match args
        .metrics
        .iter()
        .map(|s| s.parse::<MetricSelector>())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(s) => s,
        Err(e) => {
            log::error!("invalid metric selector: {e}");
            return ExitCode::SetupError;
        }
    };

    let input = TopOffendersInput {
        paths,
        include: args.include,
        exclude: args.exclude,
        selectors,
        max_results: args.max_results,
        config: AnalysisConfig::production(),
    };

    let report = rank_top_offenders(input);

    match args.format {
        TopOffendersFormat::Json => match serde_json::to_string_pretty(&report) {
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
        TopOffendersFormat::Markdown => {
            let mut stdout = io::stdout().lock();
            let _ = writeln!(stdout, "# top-offenders");
            for entry in &report.entries {
                let _ = writeln!(
                    stdout,
                    "- `{}` ({}) score={:.4}",
                    entry.path,
                    entry.language.canonical(),
                    entry.score
                );
            }
            ExitCode::Success
        }
    }
}
