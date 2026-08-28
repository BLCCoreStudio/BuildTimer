#![cfg(unix)]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn buildtimer() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_buildtimer"))
}

fn temporary_data_home(test_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!(
        "buildtimer-cli-{test_name}-{}-{unique}",
        process::id()
    ))
}

#[test]
fn preserves_wrapped_exit_code() {
    let data_home = temporary_data_home("exit-code");
    let status = Command::new(buildtimer())
        .env("XDG_DATA_HOME", &data_home)
        .args(["--", "sh", "-c", "exit 7"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(7));
    let _ = fs::remove_dir_all(data_home);
}

#[test]
fn missing_command_returns_127_without_history_entry() {
    let data_home = temporary_data_home("missing-command");
    let status = Command::new(buildtimer())
        .env("XDG_DATA_HOME", &data_home)
        .args(["--", "buildtimer-command-that-does-not-exist"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(127));
    assert!(!data_home.join("buildtimer/history.json").exists());
    let _ = fs::remove_dir_all(data_home);
}
