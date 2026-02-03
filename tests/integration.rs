//! CLI and scan integration tests.

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn units_fixture_dir() -> PathBuf {
    fixtures_dir().join("units")
}

#[test]
fn scan_fixtures_produces_output() {
    let out = tempfile::tempdir().expect("tempdir");
    let out_path = out.path().to_path_buf();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            units_fixture_dir().to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
        ])
        .status()
        .expect("run scan");
    assert!(status.success(), "scan should succeed");
    assert!(
        out_path.join("scan.sqlite").exists(),
        "scan.sqlite should exist"
    );
    assert!(
        out_path.join("report.json").exists(),
        "report.json should exist"
    );
    assert!(
        out_path.join("html").join("index.html").exists(),
        "html/index.html should exist"
    );
}

#[test]
fn unit_command_from_scan_db() {
    let out = tempfile::tempdir().expect("tempdir");
    let out_path = out.path().to_path_buf();
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            units_fixture_dir().to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
        ])
        .status()
        .expect("run scan");
    let db = out_path.join("scan.sqlite");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args(["unit", "--scan-db", db.to_str().unwrap(), "uel0101"])
        .output()
        .expect("run unit");
    assert!(
        output.status.success(),
        "unit command should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("uel0101") || stdout.contains("UEF"),
        "output should mention unit"
    );
}

#[test]
fn diff_two_scans() {
    let out1 = tempfile::tempdir().expect("tempdir1");
    let out2 = tempfile::tempdir().expect("tempdir2");
    let dir = units_fixture_dir();
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            dir.to_str().unwrap(),
            "--out",
            out1.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan 1");
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            dir.to_str().unwrap(),
            "--out",
            out2.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan 2");
    let a = out1.path().join("scan.sqlite");
    let b = out2.path().join("scan.sqlite");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "diff",
            "--a",
            a.to_str().unwrap(),
            "--b",
            b.to_str().unwrap(),
        ])
        .output()
        .expect("run diff");
    assert!(output.status.success());
}
