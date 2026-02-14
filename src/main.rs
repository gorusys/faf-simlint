//! FAF Unit Weapon Behavior Auditor — CLI.

use clap::{Parser, Subcommand};
use faf_simlint::config::{
    ScanConfig, DEFAULT_CADENCE_GAP_TOLERANCE_SECS, DEFAULT_SIMULATION_SECONDS,
};
use faf_simlint::config::{MAX_BLUEPRINT_FILES, MAX_BLUEPRINT_FILE_BYTES};
use faf_simlint::gamedata;
use faf_simlint::model::{
    normalize_projectile_path, projectile_from_lua, unit_summary_from_file, ProjectileData,
};
use faf_simlint::report::{write_html_report, write_json_report};
use faf_simlint::store::Store;
use faf_simlint::util::{check_file_bounds, init_logging, normalize_id};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "faf-simlint")]
#[command(about = "FAF Unit Weapon Behavior Auditor (Static Analyzer + Micro-Simulator)")]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract unit blueprints from FAF gamedata folder or gamedata.scd (.zip) into a directory for scanning.
    Extract {
        #[arg(
            long,
            value_name = "PATH",
            help = "Path to gamedata folder, gamedata.scd, or FAF install root"
        )]
        gamedata: PathBuf,
        #[arg(long, value_name = "DIR", default_value = "extracted_units")]
        out: PathBuf,
    },
    /// Scan data directory: parse units/weapons, compute effective DPS, store results and emit reports.
    Scan {
        #[arg(long, value_name = "PATH")]
        data_dir: PathBuf,
        #[arg(long, value_name = "DIR", default_value = "out")]
        out: PathBuf,
        #[arg(
            long,
            value_name = "JSON",
            help = "Optional: JSON file { \"unit_id\": dps } to use as declared DPS instead of blueprint"
        )]
        declared_dps: Option<PathBuf>,
        #[arg(long, default_value_t = DEFAULT_SIMULATION_SECONDS)]
        simulation_seconds: f64,
        #[arg(long, default_value_t = DEFAULT_CADENCE_GAP_TOLERANCE_SECS)]
        cadence_gap_tolerance: f64,
    },
    /// Print readable summary for one unit (by ID or name).
    Unit {
        #[arg(long, value_name = "PATH")]
        data_dir: Option<PathBuf>,
        #[arg(long, value_name = "DB")]
        scan_db: Option<PathBuf>,
        unit_id_or_name: String,
    },
    /// Compare two scans (e.g. before/after patch).
    Diff {
        #[arg(long)]
        a: PathBuf,
        #[arg(long)]
        b: PathBuf,
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
    },
}

/// Load declared DPS override from JSON: { "unit_id": dps_number, ... }
fn load_declared_dps(path: &PathBuf) -> Result<HashMap<String, f64>, String> {
    let s = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: HashMap<String, serde_json::Value> =
        serde_json::from_str(&s).map_err(|e| e.to_string())?;
    let mut out = HashMap::new();
    for (id, v) in raw {
        let dps = match v {
            serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => continue,
        };
        out.insert(id.to_lowercase(), dps);
    }
    Ok(out)
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    init_logging(cli.verbose);

    match cli.command {
        Commands::Extract { gamedata, out } => run_extract(gamedata, out),
        Commands::Scan {
            data_dir,
            out,
            declared_dps,
            simulation_seconds,
            cadence_gap_tolerance,
        } => run_scan(
            ScanConfig {
                data_dir,
                out_dir: out,
                simulation_seconds,
                cadence_gap_tolerance_secs: cadence_gap_tolerance,
            },
            declared_dps,
        ),
        Commands::Unit {
            data_dir,
            scan_db,
            unit_id_or_name,
        } => run_unit(data_dir, scan_db, unit_id_or_name),
        Commands::Diff { a, b, out } => run_diff(a, b, out),
    }
}

fn run_extract(gamedata: PathBuf, out: PathBuf) -> Result<(), String> {
    let resolved = gamedata::resolve_gamedata_path(&gamedata);
    let n = gamedata::extract_gamedata(&resolved, &out)?;
    tracing::info!("extracted {} unit blueprint(s) to {}", n, out.display());
    Ok(())
}

/// Resolve units root and projectiles root: if data_dir is FAF root (has units/ and projectiles/) use those;
/// if data_dir ends with "units", use it for units and sibling "projectiles" for projectiles.
fn resolve_scan_dirs(data_dir: &Path) -> (PathBuf, Option<PathBuf>) {
    let data_dir = data_dir
        .canonicalize()
        .unwrap_or_else(|_| data_dir.to_path_buf());
    let units_sub = data_dir.join("units");
    let projectiles_sub = data_dir.join("projectiles");
    if units_sub.is_dir() {
        let proj = if projectiles_sub.is_dir() {
            Some(projectiles_sub)
        } else {
            None
        };
        return (units_sub, proj);
    }
    if data_dir.file_name().map(|n| n == "units").unwrap_or(false) {
        if let Some(parent) = data_dir.parent() {
            let sibling_proj = parent.join("projectiles");
            if sibling_proj.is_dir() {
                return (data_dir.to_path_buf(), Some(sibling_proj));
            }
        }
    }
    (data_dir, None)
}

fn collect_projectile_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if out.len() >= MAX_BLUEPRINT_FILES {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for e in entries {
        let e = e.map_err(|e| e.to_string())?;
        let path = e.path();
        if path.is_dir() {
            collect_projectile_files(&path, out)?;
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with("_proj.bp"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Build projectile path key from file path: e.g. .../projectiles/XXX/XXX_proj.bp -> projectiles/xxx/xxx_proj.bp
fn projectile_file_to_key(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if let Some(i) = s.to_lowercase().find("projectiles/") {
        s[i..].to_lowercase()
    } else {
        s.to_lowercase()
    }
}

fn run_scan(cfg: ScanConfig, declared_dps_path: Option<PathBuf>) -> Result<(), String> {
    if !cfg.data_dir.is_dir() {
        return Err(format!(
            "data directory does not exist: {}",
            cfg.data_dir.display()
        ));
    }
    let data_dir_canon = cfg.data_dir.canonicalize().map_err(|e| e.to_string())?;
    let (units_root, projectiles_root) = resolve_scan_dirs(&data_dir_canon);
    let declared_dps_map = declared_dps_path
        .as_ref()
        .map(|p| load_declared_dps(p))
        .transpose()?;
    if declared_dps_map.is_some() {
        tracing::info!("using declared DPS override from file");
    }

    let mut unit_files = Vec::new();
    collect_lua_files(&units_root, &units_root, &mut unit_files)?;

    let mut projectile_map = HashMap::<String, ProjectileData>::new();
    if let Some(ref proj_dir) = projectiles_root {
        let mut proj_files = Vec::new();
        collect_projectile_files(proj_dir, &mut proj_files)?;
        tracing::info!(
            "loaded {} projectile blueprint(s) for fragment data",
            proj_files.len()
        );
        for path in &proj_files {
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if content.len() > MAX_BLUEPRINT_FILE_BYTES {
                continue;
            }
            let root = match faf_simlint::parser::parse_blueprint(&content) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let key = normalize_projectile_path(&projectile_file_to_key(path));
            let data = projectile_from_lua(&root).unwrap_or_default();
            projectile_map.insert(key, data);
        }
    }

    let mut units = Vec::new();
    let projectile_map_ref = if projectile_map.is_empty() {
        None
    } else {
        Some(&projectile_map)
    };
    for path in &unit_files {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        check_file_bounds(path, &units_root, MAX_BLUEPRINT_FILE_BYTES)?;
        if let Some(summary) = unit_summary_from_file(
            path,
            &content,
            cfg.simulation_seconds,
            cfg.cadence_gap_tolerance_secs,
            declared_dps_map.as_ref(),
            projectile_map_ref,
        )? {
            units.push(summary);
        }
    }

    fs::create_dir_all(&cfg.out_dir).map_err(|e| e.to_string())?;
    let db_path = cfg.out_dir.join("scan.sqlite");
    let store = Store::open(&db_path)?;
    store.insert_scan(data_dir_canon.to_string_lossy().as_ref(), &units)?;
    tracing::info!("stored scan with {} units", units.len());

    let json_path = cfg.out_dir.join("report.json");
    write_json_report(&units, &json_path)?;
    let html_dir = cfg.out_dir.join("html");
    write_html_report(&units, &html_dir)?;
    tracing::info!("wrote {} and {}", json_path.display(), html_dir.display());
    Ok(())
}

fn collect_lua_files(
    _root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if out.len() >= MAX_BLUEPRINT_FILES {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for e in entries {
        let e = e.map_err(|e| e.to_string())?;
        let path = e.path();
        if path.is_dir() {
            collect_lua_files(_root, &path, out)?;
        } else if path.extension().map(|x| x == "lua").unwrap_or(false) {
            // Skip FAF script files (behavior Lua), only parse blueprint-style .lua (e.g. fixtures)
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.ends_with("_script.lua") && !name.ends_with("_Script.lua") {
                out.push(path);
            }
        } else if path.extension().map(|x| x == "bp").unwrap_or(false) {
            // Real FAF layout: units/<ID>/<ID>_unit.bp; only scan unit blueprints, skip _mesh.bp etc.
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with("_unit.bp"))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn run_unit(
    data_dir: Option<PathBuf>,
    scan_db: Option<PathBuf>,
    unit_id_or_name: String,
) -> Result<(), String> {
    let key = normalize_id(&unit_id_or_name);
    if let Some(db_path) = scan_db {
        let store = Store::open(&db_path)?;
        let scans = store.list_scans()?;
        if let Some((scan_id, _data_dir, _created)) = scans.first() {
            let units = store.get_scan_units(*scan_id)?;
            for u in units {
                if normalize_id(&u.unit_id.id) == key
                    || u.unit_id
                        .name
                        .as_ref()
                        .map(|n| normalize_id(n) == key)
                        .unwrap_or(false)
                {
                    print_unit_summary(&u);
                    return Ok(());
                }
            }
        }
        return Err(format!("unit not found in scan: {}", unit_id_or_name));
    }
    if let Some(dir) = data_dir {
        let dir_canon = dir.canonicalize().map_err(|e| e.to_string())?;
        let (units_root, projectiles_root) = resolve_scan_dirs(&dir_canon);
        let mut lua_files = Vec::new();
        collect_lua_files(&units_root, &units_root, &mut lua_files)?;
        let mut projectile_map = HashMap::<String, ProjectileData>::new();
        if let Some(ref proj_dir) = projectiles_root {
            let mut proj_files = Vec::new();
            let _ = collect_projectile_files(proj_dir, &mut proj_files);
            for path in &proj_files {
                if let Ok(content) = fs::read_to_string(path) {
                    if content.len() <= MAX_BLUEPRINT_FILE_BYTES {
                        if let Ok(root) = faf_simlint::parser::parse_blueprint(&content) {
                            let key = normalize_projectile_path(&projectile_file_to_key(path));
                            let data = projectile_from_lua(&root).unwrap_or_default();
                            projectile_map.insert(key, data);
                        }
                    }
                }
            }
        }
        let projectile_map_ref = if projectile_map.is_empty() {
            None
        } else {
            Some(&projectile_map)
        };
        for path in &lua_files {
            let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
            if let Some(summary) = unit_summary_from_file(
                path,
                &content,
                DEFAULT_SIMULATION_SECONDS,
                DEFAULT_CADENCE_GAP_TOLERANCE_SECS,
                None,
                projectile_map_ref,
            )? {
                if normalize_id(&summary.unit_id.id) == key
                    || summary
                        .unit_id
                        .name
                        .as_ref()
                        .map(|n| normalize_id(n) == key)
                        .unwrap_or(false)
                {
                    print_unit_summary(&summary);
                    return Ok(());
                }
            }
        }
        return Err(format!("unit not found in data dir: {}", unit_id_or_name));
    }
    Err("provide --data-dir or --scan-db".to_string())
}

fn print_unit_summary(u: &faf_simlint::model::UnitSummary) {
    println!(
        "Unit: {} ({})",
        u.unit_id.id,
        u.unit_id.name.as_deref().unwrap_or("—")
    );
    println!("Blueprint: {}", u.blueprint_path);
    println!("\nDeclared weapons:");
    for w in &u.weapons {
        println!(
            "  {}  damage={}  projectiles={}  ROF={}  range={}",
            w.weapon_bp_id, w.damage, w.projectiles_per_fire, w.rate_of_fire, w.range
        );
    }
    println!("\nEffective (computed):");
    for e in &u.effective {
        println!(
            "  {}  nominal_dps={:.2}  effective_dps={:.2}  cycle_sec={:.3}",
            e.weapon_bp_id, e.nominal_dps, e.effective_dps, e.cycle_time_sec
        );
    }
    println!("\nAnomalies:");
    for a in &u.anomalies {
        let sev = match a.severity {
            faf_simlint::anomaly::AnomalySeverity::Info => "INFO",
            faf_simlint::anomaly::AnomalySeverity::Warn => "WARN",
            faf_simlint::anomaly::AnomalySeverity::Crit => "CRIT",
        };
        println!("  [{}] {} — {}", a.code, sev, a.summary);
    }
    if u.anomalies.is_empty() {
        println!("  None");
    }
}

fn run_diff(a: PathBuf, b: PathBuf, out: Option<PathBuf>) -> Result<(), String> {
    let store_a = Store::open(&a)?;
    let store_b = Store::open(&b)?;
    let scans_a = store_a.list_scans()?;
    let scans_b = store_b.list_scans()?;
    let (id_a, id_b) = (
        scans_a.first().map(|s| s.0).ok_or("scan A has no scans")?,
        scans_b.first().map(|s| s.0).ok_or("scan B has no scans")?,
    );
    let units_a = store_a.get_scan_units(id_a)?;
    let units_b = store_b.get_scan_units(id_b)?;
    let ids_a: std::collections::HashSet<_> =
        units_a.iter().map(|u| u.unit_id.id.as_str()).collect();
    let ids_b: std::collections::HashSet<_> =
        units_b.iter().map(|u| u.unit_id.id.as_str()).collect();
    let added: Vec<_> = ids_b.difference(&ids_a).collect();
    let removed: Vec<_> = ids_a.difference(&ids_b).collect();
    let common: Vec<_> = ids_a.intersection(&ids_b).copied().collect();

    println!("Diff: {} vs {}", a.display(), b.display());
    println!("Units added: {}", added.len());
    for id in &added {
        println!("  + {}", id);
    }
    println!("Units removed: {}", removed.len());
    for id in &removed {
        println!("  - {}", id);
    }
    println!("Common units: {}", common.len());

    let mut regressions = Vec::new();
    for id in &common {
        let ua = units_a.iter().find(|u| u.unit_id.id.as_str() == *id);
        let ub = units_b.iter().find(|u| u.unit_id.id.as_str() == *id);
        if let (Some(ua), Some(ub)) = (ua, ub) {
            let dps_a: f64 = ua.effective.iter().map(|e| e.effective_dps).sum();
            let dps_b: f64 = ub.effective.iter().map(|e| e.effective_dps).sum();
            if dps_b < dps_a * 0.95 {
                regressions.push((id.to_string(), dps_a, dps_b));
            }
        }
    }
    if !regressions.is_empty() {
        println!("DPS regressions (effective DPS dropped >5%):");
        for (id, before, after) in &regressions {
            println!("  {}  {:.2} -> {:.2}", id, before, after);
        }
    }

    if let Some(dir) = out {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let diff_json = serde_json::json!({
            "scan_a": a.to_string_lossy(),
            "scan_b": b.to_string_lossy(),
            "units_added": added,
            "units_removed": removed,
            "regressions": regressions.iter().map(|(id, a, b)| serde_json::json!({"unit": id, "dps_before": a, "dps_after": b})).collect::<Vec<_>>()
        });
        let path = dir.join("diff.json");
        fs::write(&path, serde_json::to_string_pretty(&diff_json).unwrap())
            .map_err(|e| e.to_string())?;
        tracing::info!("wrote {}", path.display());
    }
    Ok(())
}
