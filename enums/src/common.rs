use std::collections::BTreeMap;
use std::collections::hash_map::{Entry, HashMap};
use tree_sitter::Language;

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn sanitize_identifier(name: &str) -> String {
    if name == "ï»¿" {
        return "BOM".to_string();
    }
    if name == "_" {
        return "UNDERSCORE".to_string();
    }
    if name == "self" {
        return "Zelf".to_string();
    }
    if name == "Self" {
        return "SELF".to_string();
    }
    // A token composed solely of underscores (e.g. `__` emphasis delimiter in
    // Markdown grammars) survives the loop below as `__`, which then collapses
    // to an empty identifier in `camel_case`. Map such names to a run of
    // `UNDERSCORE` tokens joined with `_` so each contributes a word boundary
    // and the generated enum variant compiles.
    if !name.is_empty() && name.chars().all(|c| c == '_') {
        return std::iter::repeat_n("UNDERSCORE", name.len())
            .collect::<Vec<_>>()
            .join("_");
    }

    let mut result = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' {
            result.push(c);
        } else {
            let replacement = match c {
                '~' => "TILDE",
                '`' => "BQUOTE",
                '!' => "BANG",
                '@' => "AT",
                '#' => "HASH",
                '$' => "DOLLAR",
                '%' => "PERCENT",
                '^' => "CARET",
                '&' => "AMP",
                '*' => "STAR",
                '(' => "LPAREN",
                ')' => "RPAREN",
                '-' => "DASH",
                '+' => "PLUS",
                '=' => "EQ",
                '{' => "LBRACE",
                '}' => "RBRACE",
                '[' => "LBRACK",
                ']' => "RBRACK",
                '\\' => "BSLASH",
                '|' => "PIPE",
                ':' => "COLON",
                ';' => "SEMI",
                '"' => "DQUOTE",
                '\'' => "SQUOTE",
                '<' => "LT",
                '>' => "GT",
                ',' => "COMMA",
                '.' => "DOT",
                '?' => "QMARK",
                '/' => "SLASH",
                '\n' => "LF",
                '\r' => "CR",
                '\t' => "TAB",
                _ => continue,
            };
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            result += replacement;
        }
    }

    // If all characters were unmapped (e.g. Unicode symbols like `·`),
    // generate identifier from their codepoints.
    if result.is_empty() {
        if name.is_empty() {
            result = "EMPTY".to_string();
        } else {
            result = name
                .chars()
                .map(|c| format!("U{:04X}", c as u32))
                .collect::<Vec<_>>()
                .join("_");
        }
    }

    // Rust identifiers cannot start with a digit. Some tree-sitter grammars
    // expose tokens whose first character is a digit (e.g. PowerShell's
    // redirection operators `2>`, `3>&1`). Prefix such identifiers with `N`
    // so the generated enum variant compiles.
    if result.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        result.insert(0, 'N');
    }

    result
}

pub fn sanitize_string(name: &str, escape: bool) -> String {
    let mut result = String::with_capacity(name.len());
    if escape {
        for c in name.chars() {
            match c {
                '\"' => result += "\\\\\\\"",
                '\\' => result += "\\\\\\\\",
                '\t' => result += "\\\\t",
                '\n' => result += "\\\\n",
                '\r' => result += "\\\\r",
                _ => result.push(c),
            }
        }
    } else {
        for c in name.chars() {
            match c {
                '\"' => result += "\\\"",
                '\\' => result += "\\\\",
                '\t' => result += "\\t",
                '\n' => result += "\\n",
                '\r' => result += "\\r",
                _ => result.push(c),
            }
        }
    }
    result
}

pub fn camel_case(name: String) -> String {
    let mut result = String::with_capacity(name.len());
    let mut cap = true;
    for c in name.chars() {
        if c == '_' {
            cap = true;
        } else if cap {
            result.extend(c.to_uppercase().collect::<Vec<char>>());
            cap = false;
        } else {
            result.push(c);
        }
    }
    result
}

pub fn get_token_names(language: &Language, escape: bool) -> Vec<(String, bool, String)> {
    let count = language.node_kind_count();
    let mut names = BTreeMap::default();
    let mut name_count = HashMap::new();
    for anon in &[false, true] {
        for i in 0..count {
            let anonymous = !language.node_kind_is_named(i as u16);
            if anonymous != *anon {
                continue;
            }
            let kind = language.node_kind_for_id(i as u16).unwrap();
            let name = sanitize_identifier(kind);
            let ts_name = sanitize_string(kind, escape);
            let name = camel_case(name);
            let e = match name_count.entry(name.clone()) {
                Entry::Occupied(mut e) => {
                    *e.get_mut() += 1;
                    (format!("{}{}", name, e.get()), true, ts_name)
                }
                Entry::Vacant(e) => {
                    e.insert(1);
                    (name, false, ts_name)
                }
            };
            names.insert(i, e);
        }
    }
    let mut names: Vec<_> = names.values().cloned().collect();
    // The tree-sitter ERROR sentinel is always appended. A small number of
    // grammars (e.g. tree-sitter-markdown-text) also declare an anonymous
    // `_error` external token whose sanitized identifier collides with `Error`;
    // suffix the explicit sentinel in that case so both variants compile.
    let sentinel_name = if names.iter().any(|(n, _, _)| n == "Error") {
        "ErrorSentinel".to_string()
    } else {
        "Error".to_string()
    };
    names.push((sentinel_name, false, "ERROR".to_string()));

    names
}
