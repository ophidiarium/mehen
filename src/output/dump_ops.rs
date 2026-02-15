use std::io::{self, Write};

use owo_colors::OwoColorize;

use crate::ops::Ops;

/// Dumps all operands and operators of a code.
///
/// Returns a [`Result`] value, when an error occurs.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
///
/// use mehen::{dump_ops, operands_and_operators, RustParser, ParserTrait};
///
/// # fn main() {
/// let source_code = "fn main() { let a = 42; }";
///
/// let path = PathBuf::from("foo.rs");
/// let source_as_vec = source_code.as_bytes().to_vec();
///
/// let parser = RustParser::new(source_as_vec, &path, None);
///
/// let ops = operands_and_operators(&parser, &path).unwrap();
///
/// dump_ops(&ops).unwrap();
/// # }
/// ```
///
/// [`Result`]: #variant.Result
pub fn dump_ops(ops: &Ops) -> std::io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    dump_space(ops, "", true, &mut stdout)
}

fn dump_space(
    space: &Ops,
    prefix: &str,
    last: bool,
    stdout: &mut io::StdoutLock,
) -> std::io::Result<()> {
    let (pref_child, pref) = if last { ("   ", "`- ") } else { ("|  ", "|- ") };

    write!(stdout, "{}", format_args!("{prefix}{pref}").blue())?;
    write!(
        stdout,
        "{}",
        format_args!("{}: ", space.kind).yellow().bold()
    )?;
    write!(
        stdout,
        "{}",
        space.name.as_ref().map_or("", |name| name).cyan().bold()
    )?;
    writeln!(
        stdout,
        "{}",
        format_args!(" (@{})", space.start_line).red().bold()
    )?;

    let prefix = format!("{prefix}{pref_child}");
    dump_space_ops(space, &prefix, space.spaces.is_empty(), stdout)?;

    if let Some((last, spaces)) = space.spaces.split_last() {
        for space in spaces {
            dump_space(space, &prefix, false, stdout)?;
        }
        dump_space(last, &prefix, true, stdout)?;
    }

    Ok(())
}

fn dump_space_ops(
    ops: &Ops,
    prefix: &str,
    last: bool,
    stdout: &mut io::StdoutLock,
) -> std::io::Result<()> {
    dump_ops_values("operators", &ops.operators, prefix, last, stdout)?;
    dump_ops_values("operands", &ops.operands, prefix, last, stdout)
}

fn dump_ops_values(
    name: &str,
    ops: &[String],
    prefix: &str,
    last: bool,
    stdout: &mut io::StdoutLock,
) -> std::io::Result<()> {
    let (pref_child, pref) = if last { ("   ", "`- ") } else { ("|  ", "|- ") };

    write!(stdout, "{}", format_args!("{prefix}{pref}").blue())?;
    writeln!(stdout, "{}", name.green().bold())?;

    let prefix = format!("{prefix}{pref_child}");
    for op in ops.iter().take(ops.len() - 1) {
        write!(stdout, "{}", format_args!("{prefix}|- ").blue())?;
        writeln!(stdout, "{}", op.white())?;
    }

    write!(stdout, "{}", format_args!("{prefix}`- ").blue())?;
    writeln!(stdout, "{}", ops.last().unwrap().white())
}
