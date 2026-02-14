//! Weapon and unit model: DPS, cadence, salvo, target class.

mod extract;
mod projectile;

pub use extract::{
    build_unit_summary, unit_id_from_lua, unit_summary_from_file, weapon_from_lua,
    weapons_from_unit_lua,
};
pub use projectile::{normalize_projectile_path, projectile_from_lua, ProjectileData};
use serde::{Deserialize, Serialize};

/// Identifies a unit blueprint (ID or name).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitId {
    pub id: String,
    pub name: Option<String>,
}

/// Declared weapon stats from blueprint.
/// FAF engine uses RackSalvoSize/MuzzleSalvoSize/MuzzleSalvoDelay (not ProjectilesPerOnFire, which is deprecated).
/// Weapon Damage does not include fragments or DoT; fragment count/damage come from projectiles data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponDeclared {
    pub weapon_bp_id: String,
    /// Direct hit damage from weapon blueprint.
    pub damage: f64,
    /// Extra damage on impact (e.g. UEF T1 bomber); not included in Damage.
    pub initial_damage: Option<f64>,
    /// Projectile path from blueprint (e.g. "/projectiles/.../..._proj.bp"); used to resolve fragment data.
    pub projectile_id: Option<String>,
    /// Fragment count from projectile blueprint (reliable source; weapon blueprint does not include fragments).
    pub fragment_count: Option<u32>,
    /// Damage per fragment from fragment projectile; total fragment damage = fragment_count * fragment_damage.
    pub fragment_damage: Option<f64>,
    pub damage_radius: f64,
    /// Deprecated in FAF; engine uses MuzzleSalvoSize × muzzles. Kept as fallback for nominal DPS when salvo not set.
    pub projectiles_per_fire: u32,
    pub rate_of_fire: f64,
    pub muzzle_velocity: Option<f64>,
    pub range: f64,
    pub salvo_size: Option<u32>,
    pub salvo_delay: Option<f64>,
    pub reload_time: Option<f64>,
    /// RackSalvoSize: number of rack firings before reload. From FAF blueprint.
    pub rack_salvo_size: Option<u32>,
    /// RackSalvoReloadTime (RackSalvoReloadTime). From FAF blueprint.
    pub rack_salvo_reload_time: Option<f64>,
    /// MuzzleSalvoSize: shots per rack fire. From FAF blueprint.
    pub muzzle_salvo_size: Option<u32>,
    /// MuzzleSalvoDelay: delay between muzzle shots. From FAF blueprint.
    pub muzzle_salvo_delay: Option<f64>,
    pub turret_capable: bool,
    pub target_categories: Vec<String>,
}

/// Computed effective stats for one weapon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponEffective {
    pub weapon_bp_id: String,
    pub nominal_dps: f64,
    pub effective_dps: f64,
    pub cycle_time_sec: f64,
    pub shots_per_cycle: u32,
    pub salvo_duration_sec: f64,
    pub reload_sec: f64,
    pub target_class_modifiers: Vec<TargetClassDps>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetClassDps {
    pub category: String,
    pub effective_dps: f64,
    pub modifier_note: Option<String>,
}

/// Full unit summary: declared + computed + anomalies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitSummary {
    pub unit_id: UnitId,
    pub blueprint_path: String,
    pub weapons: Vec<WeaponDeclared>,
    pub effective: Vec<WeaponEffective>,
    pub anomalies: Vec<crate::anomaly::Anomaly>,
    /// When set, this is the declared DPS for the unit (e.g. from --declared-dps JSON); used for comparison with sum(effective).
    pub declared_dps_override: Option<f64>,
}

/// Total damage per shot: weapon Damage + InitialDamage + (fragment_count * fragment_damage). Weapon blueprint damage does not include fragments or DoT.
pub fn total_damage_per_shot(w: &WeaponDeclared) -> f64 {
    let base = w.damage + w.initial_damage.unwrap_or(0.0);
    let frag = w.fragment_count.unwrap_or(0) as f64 * w.fragment_damage.unwrap_or(0.0);
    base + frag
}

/// Compute nominal DPS: (total_damage_per_shot * projectiles) * rate, where rate is shots per second.
pub fn nominal_dps(damage: f64, projectiles: u32, rate_of_fire: f64) -> f64 {
    if rate_of_fire <= 0.0 {
        return 0.0;
    }
    damage * (projectiles as f64) * rate_of_fire
}

/// Cycle time from ROF: 1/rate_of_fire seconds per shot, or reload_time if present.
pub fn cycle_time_sec(rate_of_fire: f64, reload_time: Option<f64>) -> f64 {
    if let Some(r) = reload_time {
        if r > 0.0 {
            return r;
        }
    }
    if rate_of_fire > 0.0 {
        1.0 / rate_of_fire
    } else {
        0.0
    }
}

/// Salvo duration: salvo_size * salvo_delay (or 0 if no salvo).
pub fn salvo_duration_sec(salvo_size: Option<u32>, salvo_delay: Option<f64>) -> f64 {
    match (salvo_size, salvo_delay) {
        (Some(n), Some(d)) if n > 0 && d >= 0.0 => (n as f64) * d,
        _ => 0.0,
    }
}

/// Effective DPS accounting for salvo and reload: damage per cycle / cycle time.
pub fn effective_dps(
    damage: f64,
    projectiles: u32,
    rate_of_fire: f64,
    reload_time: Option<f64>,
    salvo_size: Option<u32>,
    salvo_delay: Option<f64>,
) -> f64 {
    let cycle = cycle_time_sec(rate_of_fire, reload_time);
    if cycle <= 0.0 {
        return 0.0;
    }
    let salvo_dur = salvo_duration_sec(salvo_size, salvo_delay);
    let shots_per_cycle = if salvo_size.unwrap_or(1) > 0 {
        salvo_size.unwrap_or(1)
    } else {
        1
    };
    let damage_per_cycle = damage * (projectiles as f64) * (shots_per_cycle as f64);
    let total_cycle = cycle + salvo_dur;
    if total_cycle <= 0.0 {
        return 0.0;
    }
    damage_per_cycle / total_cycle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_dps_basic() {
        assert!((nominal_dps(100.0, 1, 2.0) - 200.0).abs() < 1e-6);
        assert!((nominal_dps(50.0, 2, 1.0) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn cycle_time_from_rof() {
        assert!((cycle_time_sec(2.0, None) - 0.5).abs() < 1e-6);
        assert!((cycle_time_sec(1.0, Some(1.5)) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn salvo_duration() {
        assert!((salvo_duration_sec(Some(3), Some(0.1)) - 0.3).abs() < 1e-6);
    }

    #[test]
    fn effective_dps_simple() {
        let d = effective_dps(100.0, 1, 2.0, None, None, None);
        assert!(d > 0.0 && d < 250.0);
    }
}
