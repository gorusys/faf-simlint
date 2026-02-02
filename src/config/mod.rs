//! Configuration loading and validation.

use serde::Deserialize;
use std::path::PathBuf;

/// Maximum number of blueprint files to process in one scan (bound input size).
pub const MAX_BLUEPRINT_FILES: usize = 50_000;

/// Maximum size in bytes for a single blueprint file.
pub const MAX_BLUEPRINT_FILE_BYTES: usize = 2 * 1024 * 1024;

/// Default simulation window in seconds for cadence analysis.
pub const DEFAULT_SIMULATION_SECONDS: f64 = 30.0;

/// Default tolerance (seconds) for cadence gap detection.
pub const DEFAULT_CADENCE_GAP_TOLERANCE_SECS: f64 = 0.05;

#[derive(Debug, Clone, Deserialize)]
pub struct ScanConfig {
    /// Path to FAF blueprint/weapon data directory.
    pub data_dir: PathBuf,
    /// Output directory for reports and SQLite DB.
    pub out_dir: PathBuf,
    /// Simulation duration in seconds for multi-weapon cadence analysis.
    #[serde(default = "default_simulation_seconds")]
    pub simulation_seconds: f64,
    /// Gap tolerance in seconds; gaps larger than this may be flagged.
    #[serde(default = "default_cadence_gap_tolerance")]
    pub cadence_gap_tolerance_secs: f64,
}

fn default_simulation_seconds() -> f64 {
    DEFAULT_SIMULATION_SECONDS
}

fn default_cadence_gap_tolerance() -> f64 {
    DEFAULT_CADENCE_GAP_TOLERANCE_SECS
}

impl ScanConfig {
    pub fn new(data_dir: PathBuf, out_dir: PathBuf) -> Self {
        Self {
            data_dir,
            out_dir,
            simulation_seconds: DEFAULT_SIMULATION_SECONDS,
            cadence_gap_tolerance_secs: DEFAULT_CADENCE_GAP_TOLERANCE_SECS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnitQueryConfig {
    pub data_dir: PathBuf,
    pub scan_db: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct DiffConfig {
    pub scan_a: PathBuf,
    pub scan_b: PathBuf,
    pub out_dir: Option<PathBuf>,
}
