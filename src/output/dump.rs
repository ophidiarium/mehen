use std::io::{self, Write};

use owo_colors::OwoColorize;

use crate::node::Node;

use crate::traits::*;

/// Dumps the `AST` of a code.
///
/// Returns a [`Result`] value, when an error occurs.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
///
/// use mehen::{dump_node, RustParser, ParserTrait};
///
/// let source_code = "fn main() { let a = 42; }";
///
/// let path = PathBuf::from("foo.rs");
/// let source_as_vec = source_code.as_bytes().to_vec();
///
/// let parser = RustParser::new(source_as_vec.clone(), &path, None);
///
/// let root = parser.get_root();
///
/// dump_node(&source_as_vec, &root, -1, None, None).unwrap();
/// ```
///
/// [`Result`]: #variant.Result
/// Line range filter for dump output.
type LineRange = (Option<usize>, Option<usize>);

pub(crate) fn dump_node(
    code: &[u8],
    node: &Node,
    depth: i32,
    line_start: Option<usize>,
    line_end: Option<usize>,
) -> std::io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    dump_tree_helper(
        code,
        node,
        "",
        true,
        &mut stdout,
        depth,
        (line_start, line_end),
    )
}

fn dump_tree_helper(
    code: &[u8],
    node: &Node,
    prefix: &str,
    last: bool,
    stdout: &mut io::StdoutLock,
    depth: i32,
    line_range: LineRange,
) -> std::io::Result<()> {
    if depth == 0 {
        return Ok(());
    }

    let (pref_child, pref) = if node.parent().is_none() {
        ("", "")
    } else if last {
        ("   ", "`- ")
    } else {
        ("|  ", "|- ")
    };

    let node_row = node.start_row() + 1;
    let mut display = true;
    if let Some(line_start) = line_range.0 {
        display = node_row >= line_start;
    }
    if let Some(line_end) = line_range.1 {
        display = display && node_row <= line_end;
    }

    if display {
        write!(stdout, "{}", format_args!("{prefix}{pref}").blue())?;

        write!(
            stdout,
            "{}",
            format_args!("{{{}:{}}} ", node.kind(), node.kind_id())
                .yellow()
                .bold()
        )?;

        write!(stdout, "{}", "from ".white())?;

        let (pos_row, pos_column) = node.start_position();
        write!(
            stdout,
            "{}",
            format_args!("({}, {}) ", pos_row + 1, pos_column + 1).green()
        )?;

        write!(stdout, "{}", "to ".white())?;

        let (pos_row, pos_column) = node.end_position();
        write!(
            stdout,
            "{}",
            format_args!("({}, {}) ", pos_row + 1, pos_column + 1).green()
        )?;

        if node.start_row() == node.end_row() {
            write!(stdout, "{}", ": ".white())?;

            let code = &code[node.start_byte()..node.end_byte()];
            if let Ok(code) = String::from_utf8(code.to_vec()) {
                write!(stdout, "{}", format_args!("{code} ").red().bold())?;
            } else {
                stdout.write_all(code).unwrap();
            }
        }

        writeln!(stdout)?;
    }

    let count = node.child_count();
    if count != 0 {
        let prefix = format!("{prefix}{pref_child}");
        let mut i = count;
        let mut cursor = node.cursor();
        cursor.goto_first_child();

        loop {
            i -= 1;
            dump_tree_helper(
                code,
                &cursor.node(),
                &prefix,
                i == 0,
                stdout,
                depth - 1,
                line_range,
            )?;
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    Ok(())
}

/// Configuration options for dumping the `AST` of a code.
#[derive(Debug)]
pub(crate) struct DumpCfg {
    /// The first line of code to dump
    ///
    /// If `None`, the code is dumped from the first line of code
    /// in a file
    pub(crate) line_start: Option<usize>,
    /// The last line of code to dump
    ///
    /// If `None`, the code is dumped until the last line of code
    /// in a file
    pub(crate) line_end: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct Dump {
    _guard: (),
}

impl Callback for Dump {
    type Res = std::io::Result<()>;
    type Cfg = DumpCfg;

    fn call<T: ParserTrait>(cfg: Self::Cfg, parser: &T) -> Self::Res {
        dump_node(
            parser.get_code(),
            &parser.get_root(),
            -1,
            cfg.line_start,
            cfg.line_end,
        )
    }
}
