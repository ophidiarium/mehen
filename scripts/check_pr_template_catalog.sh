#!/usr/bin/env bash
# shellcheck shell=bash
#
# check_pr_template_catalog.sh — Phase-F emitter linter (§39.5.3).
#
# Guards the strict §39.5.2 template catalog contract for
# `src/diff_markdown.rs`:
#
# 1. Every `format!(` / `write!(` / `writeln!(` call must be inside a helper
#    whose name starts with `tmpl_` (the catalog slot-fillers) or must match
#    the narrow allow-list of mechanical rendering helpers below. Doc and
#    line comments are skipped.
# 2. No §39.5.3 forbidden phrase appears in a non-comment line. The list is
#    identical to the spec: causation / intent / speculation verbs.
#
# The script exits non-zero on any violation and prints the offending
# line(s). It is designed to be shellcheck-clean.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET="${REPO_ROOT}/src/diff_markdown.rs"

if [[ ! -f "${TARGET}" ]]; then
    echo "error: target file not found: ${TARGET}" >&2
    exit 1
fi

# Allow-listed helpers: mechanical rendering that feeds a catalog template.
# Each name matches a `fn <name>(` declaration (at any indentation).
ALLOWED_FUNCS=(
    "render_doc_section"
    "render_drill_down"
    "render_drill_structural"
    "render_drill_en_wording"
    "render_drill_en_lexical"
    "render_drill_ja"
    "render_filler_contributors"
    "write_headline_table"
    "heading_scope"
    "format_int_thousands"
    "format_link_list"
    "format_surface_list_without_line"
    "format_value"
    "build_file_link"
    "render"
)

allowed_pattern="$(IFS='|'; echo "${ALLOWED_FUNCS[*]}")"

violations=0

# Build a (line_no, enclosing_fn_name) index via a single awk pass.
# The awk script tracks the most recent `fn NAME(` declaration (at any
# indentation level) and prints a `LINE\tFN` record for every `format!`,
# `write!`, or `writeln!` call that is not inside a comment.
mapfile -t fn_lines < <(awk '
    # Track the nearest preceding fn declaration (any indent).
    match($0, /(^|[[:space:]])fn +[A-Za-z_][A-Za-z0-9_]*/) {
        name_part = substr($0, RSTART, RLENGTH)
        sub(/.*fn +/, "", name_part)
        enclosing = name_part
        next
    }
    # Skip line comments outright.
    /^[[:space:]]*\/\// { next }
    /^[[:space:]]*\/\*/ { next }
    /^[[:space:]]*\*/  { next }
    /format!\(|write!\(|writeln!\(/ {
        printf "%d\t%s\n", NR, enclosing
    }
' "${TARGET}")

for record in "${fn_lines[@]}"; do
    line_no="${record%%$'\t'*}"
    enclosing_fn="${record#*$'\t'}"
    case "${enclosing_fn}" in
        tmpl_*)
            continue
            ;;
    esac
    if [[ -n "${enclosing_fn}" ]] \
        && echo "${enclosing_fn}" | grep -Eq "^(${allowed_pattern})\$"; then
        continue
    fi
    # Read the offending source line for the error message.
    line_text="$(sed -n "${line_no}p" "${TARGET}")"
    echo "violation: format!/write! outside template catalog" >&2
    echo "    enclosing_fn: ${enclosing_fn:-(top-level)}" >&2
    echo "    ${TARGET}:${line_no}: ${line_text}" >&2
    violations=$((violations + 1))
done

# §39.5.3 forbidden phrases. Matching is case-insensitive, against
# non-comment lines only.
FORBIDDEN_PHRASES=(
    'because'
    'due to'
    'caused by'
    'following'
    'since'
    'likely'
    'probably'
    'appears to'
    'seems'
    'may indicate'
    'suggests'
    'possibly'
)

for phrase in "${FORBIDDEN_PHRASES[@]}"; do
    # Strip comments, then grep for the phrase.
    matches="$(grep -n -F -i -- "${phrase}" "${TARGET}" \
        | awk -F: '$0 !~ /^[0-9]+:[[:space:]]*\/\//' \
        || true)"
    if [[ -n "${matches}" ]]; then
        echo "violation: §39.5.3 forbidden phrase '${phrase}' found:" >&2
        while IFS= read -r m; do
            echo "    ${TARGET}:${m}" >&2
        done <<< "${matches}"
        violations=$((violations + 1))
    fi
done

if (( violations > 0 )); then
    echo "" >&2
    echo "scripts/check_pr_template_catalog.sh found ${violations} violation(s)." >&2
    exit 1
fi

echo "scripts/check_pr_template_catalog.sh: OK"
exit 0
