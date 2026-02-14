//! CLI and scan integration tests.

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn units_fixture_dir() -> PathBuf {
    fixtures_dir().join("units")
}

/// Real FAF dataset copied into the project (units + projectiles).
fn real_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join("real")
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

// ---- Real data tests (testdata/real: copied units + projectiles from FAF) ----

#[test]
fn real_data_scan_succeeds_and_produces_five_units() {
    let real = real_data_dir();
    if !real.join("units").is_dir() {
        eprintln!("skip real_data tests: testdata/real/units not found");
        return;
    }
    let out = tempfile::tempdir().expect("tempdir");
    let out_path = out.path();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            real.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
        ])
        .status()
        .expect("run scan");
    assert!(status.success(), "scan on real data should succeed");
    assert!(out_path.join("scan.sqlite").exists());
    assert!(out_path.join("report.json").exists());
    assert!(out_path.join("html").join("index.html").exists());

    let store = faf_simlint::store::Store::open(&out_path.join("scan.sqlite")).expect("open db");
    let scans = store.list_scans().expect("list scans");
    let (scan_id, _data_dir, _created) = scans.first().expect("one scan");
    let units = store.get_scan_units(*scan_id).expect("get units");
    assert_eq!(units.len(), 5, "real dataset has 5 units");
}

#[test]
fn real_data_unit_uel0101_has_expected_weapon() {
    let real = real_data_dir();
    if !real.join("units").is_dir() {
        return;
    }
    let out = tempfile::tempdir().expect("tempdir");
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            real.to_str().unwrap(),
            "--out",
            out.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan");
    let store = faf_simlint::store::Store::open(&out.path().join("scan.sqlite")).expect("open db");
    let scans = store.list_scans().expect("list scans");
    let units = store.get_scan_units(scans[0].0).expect("get units");
    let u = units
        .iter()
        .find(|u| u.unit_id.id.eq_ignore_ascii_case("uel0101"))
        .expect("UEL0101 in scan");
    assert!(!u.weapons.is_empty(), "UEL0101 has at least one weapon");
    let w = &u.weapons[0];
    assert!((w.damage - 4.0).abs() < 0.01, "UEL0101 weapon damage 4");
    assert!(
        (w.rate_of_fire - 0.5).abs() < 0.01,
        "UEL0101 ROF 0.5 (10/20 ticks)"
    );
    assert_eq!(w.range, 26.0);
}

#[test]
fn real_data_unit_uea0103_has_initial_damage() {
    let real = real_data_dir();
    if !real.join("units").is_dir() {
        return;
    }
    let out = tempfile::tempdir().expect("tempdir");
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            real.to_str().unwrap(),
            "--out",
            out.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan");
    let store = faf_simlint::store::Store::open(&out.path().join("scan.sqlite")).expect("open db");
    let units = store
        .get_scan_units(store.list_scans().expect("list")[0].0)
        .expect("get units");
    let u = units
        .iter()
        .find(|u| u.unit_id.id.eq_ignore_ascii_case("uea0103"))
        .expect("UEA0103 in scan");
    let bomb = u
        .weapons
        .iter()
        .find(|w| w.initial_damage.is_some())
        .expect("UEA0103 has weapon with InitialDamage (bomber)");
    assert!(
        (bomb.initial_damage.unwrap() - 42.5).abs() < 0.01,
        "UEF T1 bomber InitialDamage 42.5"
    );
}

#[test]
fn real_data_unit_uel0103_has_fragment_projectile() {
    let real = real_data_dir();
    if !real.join("units").is_dir() {
        return;
    }
    let out = tempfile::tempdir().expect("tempdir");
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            real.to_str().unwrap(),
            "--out",
            out.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan");
    let store = faf_simlint::store::Store::open(&out.path().join("scan.sqlite")).expect("open db");
    let units = store
        .get_scan_units(store.list_scans().expect("list")[0].0)
        .expect("get units");
    let u = units
        .iter()
        .find(|u| u.unit_id.id.eq_ignore_ascii_case("uel0103"))
        .expect("UEL0103 in scan");
    let frag = u
        .weapons
        .iter()
        .find(|w| w.fragment_count.is_some())
        .expect("UEL0103 has weapon with fragment count from projectiles");
    assert_eq!(
        frag.fragment_count,
        Some(5),
        "TIFFragmentationSensorShell01 has 5 fragments"
    );
}

#[test]
fn real_data_unit_command_works() {
    let real = real_data_dir();
    if !real.join("units").is_dir() {
        return;
    }
    let out = tempfile::tempdir().expect("tempdir");
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            real.to_str().unwrap(),
            "--out",
            out.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan");
    let db = out.path().join("scan.sqlite");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args(["unit", "--scan-db", db.to_str().unwrap(), "uel0101"])
        .output()
        .expect("run unit");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("UEL0101") || stdout.contains("uel0101"));
    assert!(stdout.contains("damage") || stdout.contains("ROF"));
}

#[test]
fn real_data_diff_works() {
    let real = real_data_dir();
    if !real.join("units").is_dir() {
        return;
    }
    let out1 = tempfile::tempdir().expect("tempdir1");
    let out2 = tempfile::tempdir().expect("tempdir2");
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            real.to_str().unwrap(),
            "--out",
            out1.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan 1");
    std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "scan",
            "--data-dir",
            real.to_str().unwrap(),
            "--out",
            out2.path().to_str().unwrap(),
        ])
        .status()
        .expect("scan 2");
    let a = out1.path().join("scan.sqlite");
    let b = out2.path().join("scan.sqlite");
    let diff_out = tempfile::tempdir().expect("tempdir diff");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_faf-simlint"))
        .args([
            "diff",
            "--a",
            a.to_str().unwrap(),
            "--b",
            b.to_str().unwrap(),
            "--out",
            diff_out.path().to_str().unwrap(),
        ])
        .status()
        .expect("run diff");
    assert!(status.success());
    assert!(diff_out.path().join("diff.json").exists());
}
