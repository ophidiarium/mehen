//! Japanese sentence segmentation (§34.5).
//!
//! Tier-0 rules:
//!   - Primary terminators: `。` `！` `？` (and `.!?` when the context is JA).
//!   - Do not split inside `「…」` `『…』` `（…）` `(...)` brackets.
//!   - Treat `\n\n` (paragraph break) as a terminator.
//!   - Ellipsis `…` / `‥` / `...` is NOT a terminator.

pub(crate) fn split(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for paragraph in text.split("\n\n") {
        let p = paragraph.trim();
        if p.is_empty() {
            continue;
        }
        for s in split_paragraph(p) {
            let t = s.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

fn split_paragraph(p: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = p.chars().collect();
    let mut depth = 0i32;
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];

        match c {
            '「' | '『' | '(' | '（' | '[' | '【' | '《' | '〈' => depth += 1,
            '」' | '』' | ')' | '）' | ']' | '】' | '》' | '〉' => {
                depth = depth.saturating_sub(1).max(0);
            }
            _ => {}
        }

        buf.push(c);

        // Ellipsis check — three dots (ASCII or Japanese `…`) are NOT
        // terminators.
        let is_ellipsis = c == '…'
            || (c == '.' && {
                let next = chars.get(i + 1).copied();
                let nnext = chars.get(i + 2).copied();
                next == Some('.') && nnext == Some('.')
            });

        if is_ellipsis && c == '.' {
            // Emit the three dots as part of the buffer without terminating.
            if let Some(&dot2) = chars.get(i + 1) {
                buf.push(dot2);
            }
            if let Some(&dot3) = chars.get(i + 2) {
                buf.push(dot3);
            }
            i += 3;
            continue;
        }
        if is_ellipsis {
            i += 1;
            continue;
        }

        let is_terminator = matches!(c, '。' | '！' | '？' | '!' | '?');
        // ASCII `.` is a terminator only when the preceding char was a kana
        // or kanji (JA-context heuristic).
        let ascii_period_as_term = c == '.' && {
            chars
                .get(i.saturating_sub(1))
                .map(|&p| is_ja_char(p))
                .unwrap_or(false)
        };

        if (is_terminator || ascii_period_as_term) && depth == 0 {
            out.push(std::mem::take(&mut buf));
        }
        i += 1;
    }

    if !buf.trim().is_empty() {
        out.push(buf);
    }
    out
}

fn is_ja_char(c: char) -> bool {
    let u = c as u32;
    (0x3040..=0x309F).contains(&u)
        || (0x30A0..=0x30FF).contains(&u)
        || (0x4E00..=0x9FFF).contains(&u)
        || (0x3400..=0x4DBF).contains(&u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_kuten() {
        let t = "これは一文です。これも一文です。";
        let s = split(t);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn keeps_brackets_intact() {
        let t = "彼は「これは本だ。あれも本だ。」と言った。";
        let s = split(t);
        // Only the outermost `。` terminates; the two inner ones are inside
        // the quote.
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn ellipsis_is_not_terminator() {
        let t = "そして…続きがあります。";
        let s = split(t);
        assert_eq!(s.len(), 1);
    }
}
