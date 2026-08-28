use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const HISTORY_VERSION: u32 = 1;
const HISTORY_LIMIT: usize = 500;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub command_key: String,
    pub command: String,
    pub duration_ms: u64,
    pub timestamp_unix: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct HistoryFile {
    version: u32,
    entries: Vec<HistoryEntry>,
}

impl Default for HistoryFile {
    fn default() -> Self {
        Self {
            version: HISTORY_VERSION,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct CommandOutcome {
    pub duration: Duration,
    pub exit_code: i32,
}

#[derive(Debug, PartialEq)]
pub enum Trend {
    Faster { delta_ms: u64, percent: Option<f64> },
    Slower { delta_ms: u64, percent: Option<f64> },
    Same,
}

pub fn data_file_path() -> io::Result<PathBuf> {
    if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(xdg_data_home)
            .join("buildtimer")
            .join("history.json"));
    }

    if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("buildtimer")
            .join("history.json"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "could not determine a local data directory (XDG_DATA_HOME and HOME are unset)",
    ))
}

pub fn run_wrapped(argv: &[OsString]) -> io::Result<CommandOutcome> {
    let program = argv
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no command was provided"))?;

    let started = Instant::now();
    let status = Command::new(program).args(&argv[1..]).status()?;

    Ok(CommandOutcome {
        duration: started.elapsed(),
        exit_code: exit_code_from_status(status),
    })
}

pub fn command_start_error_code(error: &io::Error) -> i32 {
    match error.kind() {
        io::ErrorKind::NotFound => 127,
        io::ErrorKind::PermissionDenied => 126,
        _ => 1,
    }
}

pub fn record_run(
    path: &Path,
    argv: &[OsString],
    duration: Duration,
) -> io::Result<Option<HistoryEntry>> {
    let sanitized = sanitize_argv(argv);
    let command_key = stable_key(&sanitized);
    let command = sanitized
        .iter()
        .map(|arg| quote_for_display(arg))
        .collect::<Vec<_>>()
        .join(" ");

    let mut history = load_history(path)?;
    let previous = history
        .entries
        .iter()
        .rev()
        .find(|entry| entry.command_key == command_key)
        .cloned();

    history.entries.push(HistoryEntry {
        command_key,
        command,
        duration_ms: duration_to_millis(duration),
        timestamp_unix: unix_timestamp(),
    });

    if history.entries.len() > HISTORY_LIMIT {
        let excess = history.entries.len() - HISTORY_LIMIT;
        history.entries.drain(0..excess);
    }

    save_history(path, &history)?;
    Ok(previous)
}

pub fn history_entries(path: &Path) -> io::Result<Vec<HistoryEntry>> {
    Ok(load_history(path)?.entries)
}

pub fn clear_history(path: &Path) -> io::Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub fn compare_duration(previous_ms: u64, current_ms: u64) -> Trend {
    if current_ms == previous_ms {
        return Trend::Same;
    }

    let (delta_ms, faster) = if current_ms < previous_ms {
        (previous_ms - current_ms, true)
    } else {
        (current_ms - previous_ms, false)
    };

    let percent = if previous_ms == 0 {
        None
    } else {
        Some((delta_ms as f64 / previous_ms as f64) * 100.0)
    };

    if faster {
        Trend::Faster { delta_ms, percent }
    } else {
        Trend::Slower { delta_ms, percent }
    }
}

pub fn duration_to_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

pub fn format_duration_ms(duration_ms: u64) -> String {
    if duration_ms < 1_000 {
        format!("{duration_ms} ms")
    } else {
        format!("{:.3} s", duration_ms as f64 / 1_000.0)
    }
}

fn load_history(path: &Path) -> io::Result<HistoryFile> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(HistoryFile::default()),
        Err(error) => return Err(error),
    };

    let history: HistoryFile = serde_json::from_slice(&contents)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    if history.version != HISTORY_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "unsupported history version {}; expected {HISTORY_VERSION}",
                history.version
            ),
        ));
    }

    Ok(history)
}

fn save_history(path: &Path, history: &HistoryFile) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "history path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }

    let mut serialized = serde_json::to_vec_pretty(history).map_err(io::Error::other)?;
    serialized.push(b'\n');

    let temporary = parent.join(format!(".history.json.tmp-{}", std::process::id()));
    fs::write(&temporary, serialized)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }

    fs::rename(temporary, path)
}

fn sanitize_argv(argv: &[OsString]) -> Vec<String> {
    let mut sanitized = Vec::with_capacity(argv.len());
    let mut redact_next = false;

    for argument in argv {
        let value = argument.to_string_lossy().into_owned();

        if redact_next {
            sanitized.push("<redacted>".to_owned());
            redact_next = false;
            continue;
        }

        if let Some((name, _)) = value.split_once('=') {
            if looks_like_environment_name(name) && !name.starts_with('-') {
                sanitized.push(format!("{name}=<redacted>"));
                continue;
            }

            if is_sensitive_name(name) {
                sanitized.push(format!("{name}=<redacted>"));
                continue;
            }
        }

        if is_sensitive_name(&value) {
            sanitized.push(value);
            redact_next = true;
            continue;
        }

        if looks_like_sensitive_inline_value(&value) {
            sanitized.push("<redacted>".to_owned());
            continue;
        }

        sanitized.push(redact_url_credentials(&value));
    }

    sanitized
}

fn is_sensitive_name(value: &str) -> bool {
    let normalized = value
        .trim_start_matches('-')
        .to_ascii_lowercase()
        .replace('_', "-");
    let compact = normalized.replace('-', "");

    matches!(compact.as_str(), "auth" | "authorization")
        || [
            "token",
            "secret",
            "password",
            "passwd",
            "apikey",
            "accesskey",
            "authorization",
            "credential",
            "privatekey",
        ]
        .iter()
        .any(|marker| compact.contains(marker))
}

fn looks_like_environment_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn looks_like_sensitive_inline_value(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    lowercase.contains("authorization:")
        || lowercase.starts_with("bearer ")
        || lowercase.starts_with("basic ")
}

fn redact_url_credentials(value: &str) -> String {
    let Some(scheme_end) = value.find("://") else {
        return value.to_owned();
    };
    let after_scheme = &value[scheme_end + 3..];
    let Some(at_sign) = after_scheme.find('@') else {
        return value.to_owned();
    };

    let prefix = &value[..scheme_end + 3];
    let remainder = &after_scheme[at_sign + 1..];
    format!("{prefix}<redacted>@{remainder}")
}

fn quote_for_display(value: &str) -> String {
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

fn stable_key(arguments: &[String]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    let mut hash = OFFSET_BASIS;
    for argument in arguments {
        for byte in argument
            .as_bytes()
            .iter()
            .copied()
            .chain(std::iter::once(0xff))
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
    }
    format!("{hash:016x}")
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn exit_code_from_status(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn temporary_history_path(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "buildtimer-{test_name}-{}-{unique}/history.json",
            process::id()
        ))
    }

    #[test]
    fn sanitizes_environment_assignments_and_common_secrets() {
        let sanitized = sanitize_argv(&args(&[
            "env",
            "TOKEN=top-secret",
            "cargo",
            "build",
            "--api-key=abc123",
            "--password",
            "hunter2",
            "https://user:pass@example.com/repo",
        ]));

        assert_eq!(
            sanitized,
            vec![
                "env",
                "TOKEN=<redacted>",
                "cargo",
                "build",
                "--api-key=<redacted>",
                "--password",
                "<redacted>",
                "https://<redacted>@example.com/repo",
            ]
        );
    }

    #[test]
    fn secret_values_do_not_change_the_persisted_command_key() {
        let first = sanitize_argv(&args(&["tool", "--token", "secret-one"]));
        let second = sanitize_argv(&args(&["tool", "--token", "secret-two"]));
        assert_eq!(stable_key(&first), stable_key(&second));
    }

    #[test]
    fn compares_faster_and_slower_runs() {
        assert_eq!(
            compare_duration(2_000, 1_500),
            Trend::Faster {
                delta_ms: 500,
                percent: Some(25.0)
            }
        );
        assert_eq!(
            compare_duration(1_000, 1_250),
            Trend::Slower {
                delta_ms: 250,
                percent: Some(25.0)
            }
        );
        assert_eq!(compare_duration(1_000, 1_000), Trend::Same);
    }

    #[test]
    fn stores_loads_and_clears_history() {
        let path = temporary_history_path("store-load-clear");
        let command = args(&["cargo", "test"]);

        let previous = record_run(&path, &command, Duration::from_millis(123)).unwrap();
        assert!(previous.is_none());

        let previous = record_run(&path, &command, Duration::from_millis(100)).unwrap();
        assert_eq!(previous.unwrap().duration_ms, 123);

        let entries = history_entries(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "cargo test");

        assert!(clear_history(&path).unwrap());
        assert!(!clear_history(&path).unwrap());
        assert!(history_entries(&path).unwrap().is_empty());

        if let Some(directory) = path.parent().and_then(Path::parent) {
            let _ = fs::remove_dir_all(directory);
        }
    }

    #[test]
    fn missing_command_returns_invalid_input() {
        let error = run_wrapped(&[]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[cfg(unix)]
    #[test]
    fn wrapped_exit_code_is_preserved() {
        let outcome = run_wrapped(&args(&["sh", "-c", "exit 7"])).unwrap();
        assert_eq!(outcome.exit_code, 7);
    }
}
