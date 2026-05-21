// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Golden-output snapshot tests for the Markdown PR-comment section (§39.9).
//!
//! Each test fixture exercises one of the §39 paths documented in the
//! reference mock: improvement, new-file summary, regression with broken
//! links + long sentences, and a filler-risk high attention marker on an
//! otherwise-unchanged file. Output is captured via `insta` and must stay
//! byte-identical across runs — the emitter is deterministic by design.

use std::process::Command;

use insta::assert_snapshot;

/// Path to the `mehen` binary built by the surrounding `mehen-cli`
/// crate. The CLI delegates `diff` to the still-in-place
/// `mehen::diff::run_diff` orchestrator while phase-4/5 follow-ups
/// physically relocate it into `mehen-engine`/`mehen-report`.
fn mehen_bin() -> String {
    env!("CARGO_BIN_EXE_mehen").to_string()
}

fn git(repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "Mehen Test")
        .env("GIT_AUTHOR_EMAIL", "test@mehen.invalid")
        .env("GIT_COMMITTER_NAME", "Mehen Test")
        .env("GIT_COMMITTER_EMAIL", "test@mehen.invalid")
        .env("GIT_AUTHOR_DATE", "2025-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2025-01-01T00:00:00Z")
        .output()
        .expect("failed to spawn git")
}

fn git_ok(repo: &std::path::Path, args: &[&str]) {
    let out = git(repo, args);
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path();
    git_ok(path, &["init", "-q", "-b", "main"]);
    git_ok(path, &["config", "user.name", "Mehen Test"]);
    git_ok(path, &["config", "user.email", "test@mehen.invalid"]);
    git_ok(path, &["config", "commit.gpgsign", "false"]);
    dir
}

fn commit(path: &std::path::Path, msg: &str) {
    git_ok(path, &["add", "-A"]);
    git_ok(path, &["commit", "-q", "-m", msg, "--allow-empty"]);
}

fn write_file(root: &std::path::Path, relative: &str, content: &str) {
    let full = root.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("create dir");
    }
    std::fs::write(&full, content).expect("write fixture");
}

fn run_mehen_diff(repo: &std::path::Path, from: &str, to: &str) -> String {
    let out = Command::new(mehen_bin())
        .args([
            "diff",
            "--from",
            from,
            "--to",
            to,
            "--output-format",
            "markdown",
        ])
        .current_dir(repo)
        .env_remove("GITHUB_ACTIONS")
        .env_remove("GITHUB_EVENT_NAME")
        .env_remove("GITHUB_BASE_REF")
        .env_remove("GITHUB_SHA")
        .env_remove("GITHUB_REPOSITORY")
        .output()
        .expect("failed to run mehen diff");
    assert!(
        out.status.success(),
        "mehen diff failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout utf8")
}

fn redact_sha_markers(s: &str) -> String {
    // Shorten any 40-hex SHAs to a stable `[sha]` token so snapshots don't
    // drift across runs. We also collapse the `main` label when git's
    // friendly-ref resolution differs by host (e.g. some tempdir paths
    // yield `HEAD` instead).
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_hexdigit() {
            let mut run = String::from(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_hexdigit() {
                    run.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if run.len() == 40 {
                out.push_str("[sha]");
            } else {
                out.push_str(&run);
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn golden_new_file_summary() {
    // Fixture: repo starts with only a base commit; head adds a new
    // `docs/architecture/runtime.md`. The Markdown-docs section must emit
    // a `new_file_summary` callout.
    let dir = init_repo();
    let path = dir.path();
    write_file(path, "README.md", "# Placeholder\n");
    commit(path, "base commit");

    write_file(
        path,
        "docs/architecture/runtime.md",
        "# Runtime architecture\n\nThe runtime schedules tasks across a pool of workers. \
         Each worker owns a mailbox and pulls work from a central queue at startup. \
         When a worker receives a task, it dispatches through the router and executes \
         the handler. The handler may emit events which flow back into the queue.\n\n\
         ## Workers\n\nEach worker process registers with the supervisor on startup. \
         The supervisor tracks the set of healthy workers and routes new work based \
         on load. Workers heartbeat every five seconds and are reaped after three \
         missed beats.\n\n## Queue\n\nThe queue is a durable append-only log. \
         Producers append events with a monotonic sequence; consumers track their \
         read position in a companion log. Rebalancing happens transparently.\n\n\
         ```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n\
         See [the supervisor guide](../supervisor.md) for details.\n",
    );
    commit(path, "add runtime architecture doc");

    let rendered = run_mehen_diff(path, "HEAD~1", "HEAD");
    let redacted = redact_sha_markers(&rendered);
    assert_snapshot!("new_file_summary", redacted);
}

#[test]
fn golden_regression_broken_links_and_long_sentences() {
    // Fixture: base has a clean API doc; head adds a broken relative link
    // and several 35+-word sentences. Expected output includes
    // `broken_relative_link_added`, `long_sentences_added`, and
    // `readability_target_breach` callouts.
    let dir = init_repo();
    let path = dir.path();
    write_file(
        path,
        "docs/api/auth.md",
        "# Authentication\n\nThe authentication API issues session tokens to clients.\n\n\
         ## Sessions\n\nCall the login endpoint with a username and password. \
         On success, the server returns a signed session token. Clients include \
         this token in subsequent requests.\n\n## Tokens\n\nTokens expire after 24 hours.\n",
    );
    commit(path, "base auth doc");

    write_file(
        path,
        "docs/api/auth.md",
        "# Authentication\n\nThe authentication API issues session tokens to clients that have \
         completed the initial handshake which involves three round trips across the wire \
         and then one additional validation step which verifies the client certificate \
         chain back to the root authority which is pinned in configuration at build time.\n\n\
         ## Sessions\n\nCall the login endpoint with a username and password following \
         the protocol outlined in the companion guide at [session guide](../../guide/sessions.md) \
         and also read the tokens page at [tokens refresh](./tokens.md#refresh) before \
         proceeding so that every client understands the full handshake before the server \
         starts routing traffic to backend services that will reject unauthenticated \
         requests on the edge without providing any diagnostic context back to the caller.\n\n\
         On success, the server returns a signed session token. Clients include this \
         token in subsequent requests. The signing key rotates weekly and the rotation \
         ceremony requires at least two operators to be present in the secure room with \
         physical access badges that the facility manager distributes every Monday morning.\n\n\
         ## Tokens\n\nTokens expire after 24 hours of wall-clock time measured from the \
         moment the server issued the token to the requesting client in the response body \
         which includes both the token itself and a matching expiration timestamp.\n",
    );
    commit(path, "expand auth doc");

    let rendered = run_mehen_diff(path, "HEAD~1", "HEAD");
    let redacted = redact_sha_markers(&rendered);
    assert_snapshot!("regression_broken_links", redacted);
}

#[test]
fn golden_filler_risk_high_unchanged_file_attention() {
    // Fixture: both base and head contain the same 'generated-overview' file
    // with high filler risk. Attention marker is expected per §39.4.
    let dir = init_repo();
    let path = dir.path();

    // A deliberately low-grounded, heavy-fluff doc — high filler risk.
    let filler_doc = r#"# Project Overview

This is a comprehensive and robust approach to building scalable enterprise
solutions in a cloud-native architecture. The system leverages cutting-edge
technologies to deliver seamless user experiences across various touchpoints.

## Architecture

Our platform implements a sophisticated orchestration layer that provides
end-to-end visibility into the operational health of the system. Stakeholders
benefit from granular insights derived from telemetry across the stack.

## Deployment

Deployment follows industry best practices and delivers continuous value to
customers. The automation pipeline is resilient and supports rapid iteration.

## Observability

Observability is achieved through comprehensive instrumentation that captures
detailed operational metrics. Teams leverage these insights to drive data-
informed decisions across all organizational levels.

## Security

Security is embedded throughout the software development lifecycle. The
platform adheres to industry-leading standards and maintains rigorous
compliance posture against evolving regulatory landscapes.

## Scale

Scale is a first-class concern at every layer of the platform. The system
transparently handles fluctuations in demand and maintains performance
characteristics under sustained load with minimal operational overhead.
"#;

    write_file(path, "docs/generated/overview.md", filler_doc);
    commit(path, "base: generated overview");

    // Touch a trailing blank line so the overview is in the PR diff. The
    // filler score stays above the 0.60 warn threshold either way, so the
    // headline row renders ⚠️ per §39.4.
    write_file(
        path,
        "docs/generated/overview.md",
        &format!("{filler_doc}\n"),
    );
    commit(path, "head: touch overview");

    let rendered = run_mehen_diff(path, "HEAD~1", "HEAD");
    let redacted = redact_sha_markers(&rendered);
    assert_snapshot!("filler_risk_unchanged_attention", redacted);
}
