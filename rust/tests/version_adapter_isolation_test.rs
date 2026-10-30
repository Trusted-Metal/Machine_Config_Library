//! Guard test for the project-wide architectural rule: no version's adapter
//! may import or call into another version's adapter code, even for logic
//! that happens to be byte-identical between versions today. If v1.1 (or any
//! later version, or a test-only mock of one) depends on v1.0's code, v1.0
//! can never be changed or removed later without checking every dependent —
//! and every subsequent version compounds the problem. Python's and Node's
//! equivalents of this rule were enforced first; this is Rust's version,
//! extended to cover the mock v1.1 adapter under `tests/mock_v1_1/` as well
//! as the real `capabilities::v1_0` (and future `capabilities::v1_1`)
//! adapters.
//!
//! Deliberately regex-free and adds no new dependency: scans each adapter
//! file's raw text — after stripping `//` and `/* */` comments and `"..."`
//! string literals, so a comment merely *describing* the rule (as this file
//! and `mock_v1_1/mod.rs` both do, extensively) can't trip it — for tokens
//! shaped like a version-module name (`v`, digits, a single underscore,
//! digits — e.g. `v1_0`, `v2_13`) that appear either immediately after `::`
//! or on a `use`/`mod` statement line. Those are the two shapes an actual
//! inter-version reference takes: `capabilities::v1_0::hdf5::Foo` or
//! `use machine_config::capabilities::v1_0::...`. A file under a `vX_Y`
//! directory that references any such token other than its own `vX_Y` is a
//! violation.
//!
//! This test currently passes because only `v1_0` exists as a real Rust
//! adapter, plus the newly-independent `mock_v1_1`. It starts actually
//! enforcing the rule — not just documenting it — the moment a real `v1_1`
//! adapter is added under `src/capabilities/v1_1/`.

use std::fs;
use std::path::{Path, PathBuf};

/// Recursively collects every `.rs` file under `dir`. Returns an empty `Vec`
/// (not an error) if `dir` does not exist — e.g. `capabilities/v1_1/` isn't
/// in this repo yet, and that absence is not itself a violation.
fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(collect_rs_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}

/// Strips `//` line comments, `/* */` block comments (non-nested), and
/// `"..."` string literals (basic backslash-escape handling) from Rust
/// source text. Comment mentions of another version — this file's own doc
/// comment above is full of them, deliberately, to explain the rule — must
/// never trigger a false positive; only real code paths should.
fn strip_comments_and_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(n);
            continue;
        }
        if c == '"' {
            out.push(' ');
            i += 1;
            while i < n && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < n {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            i = (i + 1).min(n);
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// `true` if `tok` is shaped exactly like a version-module name: a leading
/// `v`, one or more ASCII digits, a single underscore, then one or more
/// ASCII digits — e.g. `v1_0`, `v2_13`. Rejects lookalikes such as `v1`,
/// `v1_0_1`, `version_1`, or `V1_0` (uppercase — real module paths are
/// always lowercase; the uppercase form only ever appears inside a type
/// name like `Hdf5AdapterV1_0`, which is one identifier token, not a
/// standalone module-path segment, and won't match `v` as its first char
/// anyway).
fn is_version_token(tok: &str) -> bool {
    if !tok.starts_with('v') {
        return false;
    }
    let rest = &tok[1..];
    if rest.matches('_').count() != 1 {
        return false;
    }
    let mut parts = rest.splitn(2, '_');
    let (a, b) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    !a.is_empty()
        && !b.is_empty()
        && a.chars().all(|c| c.is_ascii_digit())
        && b.chars().all(|c| c.is_ascii_digit())
}

/// Splits a line into maximal runs of alphanumeric/underscore characters.
fn identifier_tokens(line: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    for c in line.chars() {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else if !cur.is_empty() {
            toks.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    toks
}

/// Every version-module token referenced by `code` (comments/strings already
/// stripped): the identifier immediately following each `::`, plus every
/// version-shaped identifier on a `use`/`mod` (or `pub use`/`pub mod`) line.
fn version_tokens_referenced(code: &str) -> Vec<String> {
    let mut found = Vec::new();

    let mut idx = 0;
    while let Some(pos) = code[idx..].find("::") {
        let start = idx + pos + 2;
        let ident: String =
            code[start..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if is_version_token(&ident) {
            found.push(ident);
        }
        idx = start;
        if idx >= code.len() {
            break;
        }
    }

    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("use ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("mod ")
            || trimmed.starts_with("pub mod ")
        {
            for tok in identifier_tokens(trimmed) {
                if is_version_token(&tok) {
                    found.push(tok);
                }
            }
        }
    }

    found
}

/// Checks every `.rs` file under `dir`; returns one message per violation —
/// a file that references a version-module token other than `own_version`.
fn check_directory(dir: &Path, own_version: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for file in collect_rs_files(dir) {
        let src = match fs::read_to_string(&file) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let code = strip_comments_and_strings(&src);
        for tok in version_tokens_referenced(&code) {
            if tok != own_version {
                violations.push(format!(
                    "{}: references '{tok}' but this file belongs to '{own_version}'",
                    file.display()
                ));
            }
        }
    }
    violations
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn v1_0_adapter_does_not_reference_other_versions() {
    let dir = manifest_dir().join("src").join("capabilities").join("v1_0");
    assert!(dir.is_dir(), "expected {} to exist", dir.display());
    let violations = check_directory(&dir, "v1_0");
    assert!(
        violations.is_empty(),
        "found cross-version reference(s) in capabilities/v1_0:\n{}",
        violations.join("\n")
    );
}

/// No-op today — there is no real `capabilities/v1_1/` Rust adapter yet
/// (`capabilities::v1_0` is still the only one). This starts enforcing the
/// rule the moment that directory is added, with zero changes needed here.
#[test]
fn v1_1_adapter_does_not_reference_other_versions_if_present() {
    let dir = manifest_dir().join("src").join("capabilities").join("v1_1");
    let violations = check_directory(&dir, "v1_1");
    assert!(
        violations.is_empty(),
        "found cross-version reference(s) in capabilities/v1_1:\n{}",
        violations.join("\n")
    );
}

#[test]
fn mock_v1_1_does_not_reference_v1_0_adapter() {
    let dir = manifest_dir().join("tests").join("mock_v1_1");
    assert!(dir.is_dir(), "expected {} to exist", dir.display());
    let violations = check_directory(&dir, "v1_1");
    assert!(
        violations.is_empty(),
        "found cross-version reference(s) in tests/mock_v1_1:\n{}",
        violations.join("\n")
    );
}

#[test]
fn is_version_token_detects_valid_and_rejects_lookalike_shapes() {
    assert!(is_version_token("v1_0"));
    assert!(is_version_token("v2_13"));
    assert!(is_version_token("v10_100"));
    assert!(!is_version_token("v1"));
    assert!(!is_version_token("v1_0_1"));
    assert!(!is_version_token("version_1"));
    assert!(!is_version_token("v1_"));
    assert!(!is_version_token("v_0"));
    assert!(!is_version_token("V1_0"));
    assert!(!is_version_token("Hdf5AdapterV1_0"));
}

#[test]
fn comment_mentions_of_another_version_do_not_trigger_a_violation() {
    let code = "// this talks about v1_0 and v1_1 but writes no real code\n\
                /* also v1_0 here */\n\
                let s = \"contains v1_0 inside a string\";\n";
    let stripped = strip_comments_and_strings(code);
    assert!(version_tokens_referenced(&stripped).is_empty());
}

#[test]
fn a_real_use_statement_referencing_another_version_is_detected() {
    let code = "use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;\n";
    let stripped = strip_comments_and_strings(code);
    // The `::v1_0::` scan and the whole-line `use` scan both independently
    // catch this — duplicate hits are fine, since every caller only checks
    // "is there at least one violation", never an exact count.
    let found = version_tokens_referenced(&stripped);
    assert!(!found.is_empty());
    assert!(found.iter().all(|t| t == "v1_0"));
}
