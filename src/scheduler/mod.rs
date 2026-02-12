//! Micro-scheduler: simulates weapon firing over N seconds to detect cadence interference.

use crate::model::{WeaponDeclared, WeaponEffective};
use std::collections::BTreeMap;

/// Single fire event in the schedule.
#[derive(Debug, Clone)]
pub struct FireEvent {
    pub time_sec: f64,
    pub weapon_bp_id: String,
    pub shot_index: u32,
}

/// Result of running the micro-scheduler.
#[derive(Debug, Clone)]
pub struct ScheduleResult {
    pub events: Vec<FireEvent>,
    pub window_sec: f64,
    pub weapon_expected_shots: BTreeMap<String, u32>,
    pub weapon_actual_shots: BTreeMap<String, u32>,
    pub gaps: Vec<Gap>,
}

#[derive(Debug, Clone)]
pub struct Gap {
    pub start_sec: f64,
    pub end_sec: f64,
    pub duration_sec: f64,
    pub weapons_around: Vec<String>,
}

/// Build a fire schedule for multiple weapons over `window_sec` seconds.
/// Each weapon fires at its cycle rate; when two would fire at the same time (within 1ms),
/// we serialize them (first weapon first, then second). Detects gaps larger than tolerance.
pub fn simulate(
    weapons: &[WeaponDeclared],
    effective: &[WeaponEffective],
    window_sec: f64,
    gap_tolerance_sec: f64,
) -> ScheduleResult {
    let mut events = Vec::new();
    let mut weapon_actual_shots: BTreeMap<String, u32> = weapons
        .iter()
        .map(|w| (w.weapon_bp_id.clone(), 0))
        .collect();
    let mut weapon_expected_shots: BTreeMap<String, u32> = weapons
        .iter()
        .map(|w| (w.weapon_bp_id.clone(), 0))
        .collect();

    for (eff, w) in effective.iter().zip(weapons.iter()) {
        if eff.cycle_time_sec > 0.0 {
            let cycles = (window_sec / eff.cycle_time_sec).floor() as u32;
            let expected = cycles * eff.shots_per_cycle;
            let _ = weapon_expected_shots.insert(w.weapon_bp_id.clone(), expected);
        }
    }

    // Per-weapon: (next fire time, shot index in current cycle).
    let mut next_fire: Vec<(f64, u32)> = Vec::with_capacity(weapons.len());
    for _ in effective.iter() {
        next_fire.push((0.0f64, 0));
    }

    const TIME_EPS: f64 = 0.001;
    let mut sim_time = 0.0f64;
    while sim_time < window_sec {
        let mut next_any = f64::MAX;
        let mut fire_weapon = None;
        for (i, (t, _)) in next_fire.iter().enumerate() {
            if *t <= sim_time + TIME_EPS && *t < window_sec {
                fire_weapon = Some(i);
                break;
            }
            if *t < next_any && *t < window_sec {
                next_any = *t;
            }
        }
        if let Some(i) = fire_weapon {
            let (t, shot_idx) = next_fire[i];
            sim_time = t;
            let id = &weapons[i].weapon_bp_id;
            events.push(FireEvent {
                time_sec: t,
                weapon_bp_id: id.clone(),
                shot_index: shot_idx,
            });
            weapon_actual_shots
                .entry(id.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            let eff = &effective[i];
            let cycle = eff.cycle_time_sec;
            let shots = eff.shots_per_cycle;
            let salvo_dur = eff.salvo_duration_sec;
            let next_shot = shot_idx + 1;
            let (next_t, next_shot_idx) = if next_shot < shots && salvo_dur > 0.0 {
                let spread = salvo_dur / shots as f64;
                (t + spread, next_shot)
            } else {
                (t + cycle, 0)
            };
            next_fire[i] = (next_t, next_shot_idx);
            continue;
        }
        if next_any < f64::MAX {
            sim_time = next_any;
        } else {
            break;
        }
    }

    let gaps = find_gaps(&events, window_sec, gap_tolerance_sec);
    ScheduleResult {
        events,
        window_sec,
        weapon_expected_shots,
        weapon_actual_shots,
        gaps,
    }
}

fn find_gaps(events: &[FireEvent], _window_sec: f64, tolerance_sec: f64) -> Vec<Gap> {
    let mut sorted: Vec<&FireEvent> = events.iter().collect();
    sorted.sort_by(|a, b| {
        a.time_sec
            .partial_cmp(&b.time_sec)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut gaps = Vec::new();
    for w in sorted.windows(2) {
        let (a, b) = (w[0], w[1]);
        let dur = b.time_sec - a.time_sec;
        if dur >= tolerance_sec {
            gaps.push(Gap {
                start_sec: a.time_sec,
                end_sec: b.time_sec,
                duration_sec: dur,
                weapons_around: vec![a.weapon_bp_id.clone(), b.weapon_bp_id.clone()],
            });
        }
    }
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_produces_events() {
        let weapons = vec![WeaponDeclared {
            weapon_bp_id: "W1".to_string(),
            damage: 10.0,
            initial_damage: None,
            projectile_id: None,
            fragment_count: None,
            fragment_damage: None,
            damage_radius: 0.0,
            projectiles_per_fire: 1,
            rate_of_fire: 2.0,
            muzzle_velocity: None,
            range: 20.0,
            salvo_size: None,
            salvo_delay: None,
            reload_time: None,
            rack_salvo_size: None,
            rack_salvo_reload_time: None,
            muzzle_salvo_size: None,
            muzzle_salvo_delay: None,
            turret_capable: true,
            target_categories: vec!["GROUND".to_string()],
        }];
        let effective = vec![WeaponEffective {
            weapon_bp_id: "W1".to_string(),
            nominal_dps: 20.0,
            effective_dps: 20.0,
            cycle_time_sec: 0.5,
            shots_per_cycle: 1,
            salvo_duration_sec: 0.0,
            reload_sec: 0.5,
            target_class_modifiers: vec![],
        }];
        let r = simulate(&weapons, &effective, 2.0, 0.05);
        assert!(!r.events.is_empty());
        assert!(r.weapon_actual_shots.get("W1").copied().unwrap_or(0) >= 2);
    }
}
