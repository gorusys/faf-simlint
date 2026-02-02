//! Anomaly detection: severity, explanation, technical note.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Info,
    Warn,
    Crit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub code: String,
    pub severity: AnomalySeverity,
    /// Short explanation for Discord/users.
    pub summary: String,
    /// Technical note for devs.
    pub technical: String,
    pub weapon_ids: Vec<String>,
    pub unit_id: Option<String>,
}

impl Anomaly {
    pub fn cadence_interference(unit_id: &str, weapon_ids: &[String], technical: String) -> Self {
        Self {
            code: "CADENCE_INTERFERENCE".to_string(),
            severity: AnomalySeverity::Warn,
            summary: format!(
                "Unit {}: multi-weapon firing may reduce effective ROF (cadence interference suspected).",
                unit_id
            ),
            technical,
            weapon_ids: weapon_ids.to_vec(),
            unit_id: Some(unit_id.to_string()),
        }
    }

    pub fn declared_vs_effective_mismatch(
        unit_id: &str,
        weapon_id: &str,
        declared_dps: f64,
        effective_dps: f64,
    ) -> Self {
        let ratio = if declared_dps > 0.0 {
            effective_dps / declared_dps
        } else {
            0.0
        };
        Self {
            code: "DECLARED_VS_EFFECTIVE".to_string(),
            severity: if !(0.8..=1.25).contains(&ratio) {
                AnomalySeverity::Warn
            } else {
                AnomalySeverity::Info
            },
            summary: format!(
                "Unit {} weapon {}: declared DPS {:.1} vs effective {:.1} (ratio {:.2}).",
                unit_id, weapon_id, declared_dps, effective_dps, ratio
            ),
            technical: format!(
                "Salvo/reload timing changes effective DPS from nominal. declared={} effective={}",
                declared_dps, effective_dps
            ),
            weapon_ids: vec![weapon_id.to_string()],
            unit_id: Some(unit_id.to_string()),
        }
    }

    pub fn salvo_cooldown_suspicion(unit_id: &str, weapon_id: &str, note: String) -> Self {
        Self {
            code: "SALVO_COOLDOWN_PATTERN".to_string(),
            severity: AnomalySeverity::Info,
            summary: format!(
                "Unit {} weapon {}: salvo/cooldown pattern may cause unexpected gaps.",
                unit_id, weapon_id
            ),
            technical: note,
            weapon_ids: vec![weapon_id.to_string()],
            unit_id: Some(unit_id.to_string()),
        }
    }
}
