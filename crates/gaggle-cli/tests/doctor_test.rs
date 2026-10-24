use std::process::Command;

fn temp_db_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "gaggle_doctor_{tag}_{}_{}.db",
        std::process::id(),
        nanos
    ))
}

#[test]
fn test_doctor_reports_build_core_health_and_capability_reasons() {
    let db_path = temp_db_path("human");
    let output = Command::new(env!("CARGO_BIN_EXE_gaggle"))
        .arg("--db")
        .arg(&db_path)
        .arg("doctor")
        .output()
        .expect("run gaggle doctor");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gaggle doctor failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    assert!(stdout.contains("Gaggle doctor"), "stdout:\n{stdout}");
    assert!(stdout.contains("Build: gaggle"));
    assert!(stdout.contains("Core health: ok"));
    assert!(stdout.contains("Optional capabilities:"));
    assert!(stdout.contains("Remote model providers [unavailable]"));
    assert!(stdout.contains("action: Open Settings"));
    assert!(stdout.contains("Automatic telemetry (disabled by design) [unavailable]"));
    assert!(stdout.contains("no telemetry"));

    // The human-readable report must not print prompts, secrets, or raw environment dumps.
    assert!(!stdout.contains("GAGGLE_CHATGPT_CLIENT_ID="));
    assert!(!stdout.contains("sk-"));

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}

#[test]
fn test_doctor_json_export_is_machine_readable_and_redacted() {
    let db_path = temp_db_path("json");
    let output = Command::new(env!("CARGO_BIN_EXE_gaggle"))
        .arg("--db")
        .arg(&db_path)
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("run gaggle doctor --json");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "gaggle doctor --json failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json emits valid JSON");
    assert_eq!(report["service"], "gaggle");
    assert_eq!(report["build"]["service"], "gaggle");
    assert!(report["build"]["version"].as_str().is_some());
    assert_eq!(report["core_health"]["status"], "ok");
    assert!(
        report["capabilities"]
            .as_array()
            .is_some_and(|capabilities| !capabilities.is_empty())
    );
    assert_eq!(
        report["capabilities"]
            .as_array()
            .expect("capabilities")
            .iter()
            .find(|capability| capability["id"] == "automatic_telemetry")
            .expect("telemetry capability")["state"],
        "unavailable"
    );

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}
