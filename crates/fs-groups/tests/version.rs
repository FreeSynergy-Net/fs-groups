//! The packaged binary says its version without opening a window or a server (the start test of the release).
//!
//! Pattern: Example-based test of the real binary.

use std::process::Command;

fn run(flag: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fs-groups"))
        .arg(flag)
        .output()
        .unwrap()
}

#[test]
fn the_version_is_printed_and_nothing_is_started() {
    for flag in ["--version", "-V"] {
        let out = run(flag);
        assert!(
            out.status.success(),
            "{flag}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let said = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            said.trim(),
            format!("fs-groups {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn help_succeeds_and_names_the_program() {
    for flag in ["--help", "-h"] {
        let out = run(flag);
        assert!(out.status.success(), "{flag}");
        assert!(!out.stdout.is_empty(), "{flag}");
    }
}

#[test]
fn an_unknown_option_fails() {
    let out = run("--gibtsnicht");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown option"));
}
