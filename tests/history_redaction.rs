use buildtimer::{history_entries, record_run};
use std::ffi::OsString;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn persisted_history_never_keeps_url_credentials_or_query_tokens() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("buildtimer-redaction-{}-{nonce}", std::process::id()));
    let path = root.join("history.json");
    let command = vec![
        OsString::from("curl"),
        OsString::from("https://user:pass@example.com/api?token=top-secret&mode=fast"),
    ];

    record_run(&path, &command, Duration::from_millis(10)).unwrap();
    let entries = history_entries(&path).unwrap();

    assert_eq!(entries.len(), 1);
    assert!(entries[0].command.contains("https://<redacted>@example.com/api?token=<redacted>&mode=fast"));
    assert!(!entries[0].command.contains("user:pass"));
    assert!(!entries[0].command.contains("top-secret"));

    let raw = fs::read_to_string(&path).unwrap();
    assert!(!raw.contains("user:pass"));
    assert!(!raw.contains("top-secret"));

    let _ = fs::remove_dir_all(root);
}
