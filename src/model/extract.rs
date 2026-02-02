//! Extract unit and weapon data from parsed blueprint LuaValue.

use super::{
    cycle_time_sec, effective_dps, nominal_dps, salvo_duration_sec, TargetClassDps, UnitId,
    UnitSummary, WeaponDeclared, WeaponEffective,
};
use crate::anomaly::Anomaly;
use crate::parser::LuaValue;
use crate::scheduler;
use std::path::Path;

/// Extract a single weapon's declared stats from a Lua table (weapon blueprint).
pub fn weapon_from_lua(table: &LuaValue) -> Option<WeaponDeclared> {
    let damage = table.get_num("Damage")?;
    let rate_of_fire = table.get_num("RateOfFire").unwrap_or(1.0);
    let projectiles = table.get_num("ProjectilesPerOnFire").unwrap_or(1.0) as u32;
    let range = table.get_num("MaxRadius").unwrap_or(0.0);
    let radius = table.get_num("DamageRadius").unwrap_or(0.0);
    let salvo_size = table.get_num("SalvoSize").map(|n| n as u32);
    let salvo_delay = table.get_num("SalvoDelay");
    let reload = table.get_num("ReloadTime");
    let muzzle = table.get_num("MuzzleVelocity");
    let turret = table.get_bool("TurretCapable").unwrap_or(false);
    let categories = categories_from_lua(table);
    let weapon_bp_id = table
        .get_str("BlueprintId")
        .or_else(|| table.get_str("weapon_bp_id"))
        .unwrap_or("unknown")
        .to_string();
    Some(WeaponDeclared {
        weapon_bp_id,
        damage,
        damage_radius: radius,
        projectiles_per_fire: projectiles.max(1),
        rate_of_fire: rate_of_fire.max(0.001),
        muzzle_velocity: if muzzle.map(|x| x > 0.0).unwrap_or(false) {
            muzzle
        } else {
            None
        },
        range,
        salvo_size,
        salvo_delay,
        reload_time: reload,
        turret_capable: turret,
        target_categories: categories,
    })
}

fn categories_from_lua(table: &LuaValue) -> Vec<String> {
    let cats = table.get_table("TargetCategories");
    if let Some(t) = cats {
        if let Some(len) = t.table_len() {
            let mut out = Vec::with_capacity(len);
            for i in 1..=len {
                if let Some(v) = t.get_by_index(i as u32) {
                    if let Some(s) = v.as_str() {
                        out.push(s.to_string());
                    }
                }
            }
            return out;
        }
    }
    vec![]
}

/// Extract unit ID and name from root unit table.
pub fn unit_id_from_lua(root: &LuaValue) -> Option<UnitId> {
    let id = root
        .get_str("BlueprintId")
        .or_else(|| root.get_str("UnitId"))
        .or_else(|| root.get_str("ID"))
        .map(str::to_string)?;
    let name = root
        .get_str("DisplayName")
        .or_else(|| root.get_str("Name"))
        .map(str::to_string);
    Some(UnitId {
        id: id.clone(),
        name,
    })
}

/// Collect weapon tables from unit blueprint (Weapon array or Weapons table).
pub fn weapons_from_unit_lua(root: &LuaValue) -> Vec<WeaponDeclared> {
    let mut out = Vec::new();
    if let Some(weapons_table) = root.get_table("Weapon") {
        if let Some(len) = weapons_table.table_len() {
            for i in 1..=len {
                if let Some(w) = weapons_table.get_by_index(i as u32) {
                    if let Some(decl) = weapon_from_lua(w) {
                        out.push(decl);
                    }
                }
            }
        }
    }
    if out.is_empty() {
        if let Some(w) = root.get_table("Weapon") {
            if let Some(decl) = weapon_from_lua(w) {
                out.push(decl);
            }
        }
    }
    out
}

/// Build effective stats and anomalies for one unit.
pub fn build_unit_summary(
    unit_id: UnitId,
    blueprint_path: String,
    weapons: Vec<WeaponDeclared>,
    simulation_sec: f64,
    gap_tolerance_sec: f64,
) -> UnitSummary {
    let mut effective = Vec::with_capacity(weapons.len());
    let mut anomalies = Vec::new();

    for w in &weapons {
        let nominal = nominal_dps(w.damage, w.projectiles_per_fire, w.rate_of_fire);
        let cycle = cycle_time_sec(w.rate_of_fire, w.reload_time);
        let salvo_dur = salvo_duration_sec(w.salvo_size, w.salvo_delay);
        let eff_dps = effective_dps(
            w.damage,
            w.projectiles_per_fire,
            w.rate_of_fire,
            w.reload_time,
            w.salvo_size,
            w.salvo_delay,
        );
        let shots = w.salvo_size.unwrap_or(1).max(1);
        let target_class_modifiers: Vec<TargetClassDps> = w
            .target_categories
            .iter()
            .map(|c| TargetClassDps {
                category: c.clone(),
                effective_dps: eff_dps,
                modifier_note: None,
            })
            .collect();
        effective.push(WeaponEffective {
            weapon_bp_id: w.weapon_bp_id.clone(),
            nominal_dps: nominal,
            effective_dps: eff_dps,
            cycle_time_sec: cycle,
            shots_per_cycle: shots,
            salvo_duration_sec: salvo_dur,
            reload_sec: cycle,
            target_class_modifiers,
        });

        if (nominal - eff_dps).abs() > 0.01 * nominal.max(1.0) {
            anomalies.push(Anomaly::declared_vs_effective_mismatch(
                &unit_id.id,
                &w.weapon_bp_id,
                nominal,
                eff_dps,
            ));
        }
    }

    if weapons.len() > 1 && !effective.is_empty() {
        let result = scheduler::simulate(&weapons, &effective, simulation_sec, gap_tolerance_sec);
        let expected: u32 = result.weapon_expected_shots.values().sum();
        let actual: u32 = result.weapon_actual_shots.values().sum();
        if expected > 0 && (actual as f64) < (expected as f64) * 0.95 {
            let technical = format!(
                "Over {}s expected {} shots, got {}. Gaps: {}",
                result.window_sec,
                expected,
                actual,
                result.gaps.len()
            );
            anomalies.push(Anomaly::cadence_interference(
                &unit_id.id,
                &weapons
                    .iter()
                    .map(|w| w.weapon_bp_id.clone())
                    .collect::<Vec<_>>(),
                technical,
            ));
        }
        for g in &result.gaps {
            if g.duration_sec > gap_tolerance_sec * 2.0 {
                anomalies.push(Anomaly::salvo_cooldown_suspicion(
                    &unit_id.id,
                    &g.weapons_around.join(","),
                    format!(
                        "Gap {:.2}s between {}",
                        g.duration_sec,
                        g.weapons_around.join(",")
                    ),
                ));
            }
        }
    }

    UnitSummary {
        unit_id,
        blueprint_path,
        weapons,
        effective,
        anomalies,
    }
}

/// Try to parse file and extract one unit summary (if file looks like a unit blueprint).
pub fn unit_summary_from_file(
    path: &Path,
    content: &str,
    simulation_sec: f64,
    gap_tolerance_sec: f64,
) -> Result<Option<UnitSummary>, String> {
    let root = crate::parser::parse_blueprint(content).map_err(|e| e.to_string())?;
    let unit_id = match unit_id_from_lua(&root) {
        Some(id) => id,
        None => {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            UnitId {
                id: id.clone(),
                name: None,
            }
        }
    };
    let weapons = weapons_from_unit_lua(&root);
    if weapons.is_empty() {
        return Ok(None);
    }
    let blueprint_path = path.to_string_lossy().to_string();
    Ok(Some(build_unit_summary(
        unit_id,
        blueprint_path,
        weapons,
        simulation_sec,
        gap_tolerance_sec,
    )))
}
