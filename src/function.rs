use std::io::{self, Write};
use std::path::{Path, PathBuf};

use owo_colors::OwoColorize;
use serde::Serialize;

use crate::traits::*;

use crate::checker::Checker;
use crate::getter::Getter;

/// Function span data.
#[derive(Debug, Serialize)]
pub(crate) struct FunctionSpan {
    /// The function name
    pub(crate) name: String,
    /// The first line of a function
    pub(crate) start_line: usize,
    /// The last line of a function
    pub(crate) end_line: usize,
    /// If `true`, an error is occurred in determining the span
    /// of a function
    pub(crate) error: bool,
}

/// Detects the span of each function in a code.
///
/// Returns a vector containing the [`FunctionSpan`] of each function
///
/// [`FunctionSpan`]: struct.FunctionSpan.html
pub(crate) fn function<T: ParserTrait>(parser: &T) -> Vec<FunctionSpan> {
    let root = parser.get_root();
    let code = parser.get_code();
    let mut spans = Vec::new();
    root.act_on_node(&mut |n| {
        if T::Checker::is_func(n) {
            let start_line = n.start_row() + 1;
            let end_line = n.end_row() + 1;
            if let Some(name) = T::Getter::get_func_name(n, code) {
                spans.push(FunctionSpan {
                    name: name.to_owned(),
                    start_line,
                    end_line,
                    error: false,
                });
            } else {
                spans.push(FunctionSpan {
                    name: String::new(),
                    start_line,
                    end_line,
                    error: true,
                });
            }
        }
    });

    spans
}

fn dump_span(span: &FunctionSpan, stdout: &mut io::StdoutLock, last: bool) -> std::io::Result<()> {
    let pref = if last { "   `- " } else { "   |- " };

    write!(stdout, "{}", pref.blue())?;

    if span.error {
        write!(stdout, "{}", "error: ".red().bold())?;
    } else {
        write!(
            stdout,
            "{}",
            format_args!("{}: ", span.name).magenta().bold()
        )?;
    }

    write!(stdout, "{}", "from line ".green())?;
    write!(stdout, "{}", span.start_line.white())?;
    write!(stdout, "{}", " to line ".green())?;
    writeln!(stdout, "{}", format_args!("{}.", span.end_line).white())
}

fn dump_spans(spans: &[FunctionSpan], path: &Path) -> std::io::Result<()> {
    if !spans.is_empty() {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        writeln!(
            stdout,
            "{}",
            format_args!("In file {}", path.to_str().unwrap_or("..."))
                .yellow()
                .bold()
        )?;

        let last_idx = spans.len() - 1;
        for span in &spans[..last_idx] {
            dump_span(span, &mut stdout, false)?;
        }
        dump_span(&spans[last_idx], &mut stdout, true)?;
    }
    Ok(())
}

/// Configuration options for detecting the span of
/// each function in a code.
#[derive(Debug)]
pub(crate) struct FunctionCfg {
    /// Path to the file containing the code
    pub(crate) path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct Function {
    _guard: (),
}

impl Callback for Function {
    type Res = std::io::Result<()>;
    type Cfg = FunctionCfg;

    fn call<T: ParserTrait>(cfg: Self::Cfg, parser: &T) -> Self::Res {
        let spans = function(parser);
        dump_spans(&spans, &cfg.path)
    }
}
