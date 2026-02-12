//! JSON and HTML report generation.

use crate::model::UnitSummary;
use std::fs;
use std::path::Path;

pub fn write_json_report(units: &[UnitSummary], path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(units).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn write_html_report(units: &[UnitSummary], out_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let index = render_index(units);
    fs::write(out_dir.join("index.html"), index).map_err(|e| e.to_string())?;
    let anomalies = render_anomalies_page(units);
    fs::write(out_dir.join("anomalies.html"), anomalies).map_err(|e| e.to_string())?;
    for u in units {
        let name = crate::util::normalize_id(&u.unit_id.id).replace(' ', "_");
        let path = out_dir.join(format!("unit_{}.html", name));
        let content = render_unit_page(u);
        fs::write(path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn render_index(units: &[UnitSummary]) -> String {
    let rows: String = units
        .iter()
        .map(|u| {
            let id = &u.unit_id.id;
            let name = crate::util::normalize_id(id).replace(' ', "_");
            let display = u.unit_id.name.as_deref().unwrap_or(id);
            let anomaly_count = u.anomalies.len();
            format!(
                r#"<tr><td><a href="unit_{}.html">{}</a></td><td>{}</td><td>{}</td></tr>"#,
                name,
                html_escape(display),
                u.weapons.len(),
                anomaly_count
            )
        })
        .collect();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>FAF Simlint – Units</title>
<style>body{{font-family:system-ui,sans-serif;margin:1rem;}} table{{border-collapse:collapse;}} th,td{{border:1px solid #ccc;padding:6px;}} a{{color:#06c;}}</style>
</head>
<body>
<h1>FAF Unit Weapon Behavior Report</h1>
<p>Unit list. <a href="anomalies.html">Anomalies</a></p>
<input type="text" id="search" placeholder="Search unit ID or name…" style="margin-bottom:8px;">
<table><thead><tr><th>Unit</th><th>Weapons</th><th>Anomalies</th></tr></thead>
<tbody>{}</tbody>
</table>
<script>
document.getElementById('search').oninput=function(){{
 var q=this.value.toLowerCase(), rows=document.querySelectorAll('tbody tr');
 rows.forEach(function(r){{
   r.style.display=r.textContent.toLowerCase().indexOf(q)===-1?'none':'';
 }});
}};
</script>
</body>
</html>"#,
        rows
    )
}

fn render_anomalies_page(units: &[UnitSummary]) -> String {
    let mut items = Vec::new();
    for u in units {
        for a in &u.anomalies {
            let sev = match a.severity {
                crate::anomaly::AnomalySeverity::Info => "info",
                crate::anomaly::AnomalySeverity::Warn => "warn",
                crate::anomaly::AnomalySeverity::Crit => "crit",
            };
            items.push(format!(
                r#"<div class="{}"><strong>{} – {}:</strong> {} <br><em>{}</em></div>"#,
                sev,
                html_escape(a.unit_id.as_deref().unwrap_or("")),
                html_escape(&a.code),
                html_escape(&a.summary),
                html_escape(&a.technical)
            ));
        }
    }
    let body = if items.is_empty() {
        "<p>No anomalies detected.</p>".to_string()
    } else {
        items.join("\n")
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>FAF Simlint – Anomalies</title>
<style>body{{font-family:system-ui,sans-serif;margin:1rem;}} .info{{color:#666;}} .warn{{color:#c60;}} .crit{{color:#c00;}} .info,.warn,.crit{{margin:8px 0;padding:6px;border-left:4px solid;}}</style>
</head>
<body>
<h1>Anomalies</h1>
<p><a href="index.html">Back to units</a></p>
{}
</body>
</html>"#,
        body
    )
}

fn render_unit_page(u: &UnitSummary) -> String {
    let id = &u.unit_id.id;
    let name = u.unit_id.name.as_deref().unwrap_or(id);
    let declared_rows: String = u
        .weapons
        .iter()
        .map(|w| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&w.weapon_bp_id),
                w.damage,
                w.projectiles_per_fire,
                w.rate_of_fire
            )
        })
        .collect();
    let effective_rows: String = u
        .effective
        .iter()
        .map(|e| {
            format!(
                "<tr><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{}</td></tr>",
                html_escape(&e.weapon_bp_id),
                e.nominal_dps,
                e.effective_dps,
                e.cycle_time_sec
            )
        })
        .collect();
    let declared_override_note = u
        .declared_dps_override
        .map(|d| format!("<p>Declared DPS (from override): {:.2}</p>", d))
        .unwrap_or_default();
    let anomaly_list: String = u
        .anomalies
        .iter()
        .map(|a| {
            format!(
                "<li><strong>{}:</strong> {} </li>",
                a.code,
                html_escape(&a.summary)
            )
        })
        .collect();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>{} – FAF Simlint</title>
<style>body{{font-family:system-ui,sans-serif;margin:1rem;}} table{{border-collapse:collapse;}} th,td{{border:1px solid #ccc;padding:6px;}} a{{color:#06c;}}</style>
</head>
<body>
<h1>{}</h1>
<p><a href="index.html">Back to list</a></p>
{}
<h2>Declared weapon stats (blueprint)</h2>
<table><thead><tr><th>Weapon</th><th>Damage</th><th>Projectiles</th><th>ROF</th></tr></thead><tbody>{}</tbody></table>
<h2>Effective (computed)</h2>
<table><thead><tr><th>Weapon</th><th>Nominal DPS</th><th>Effective DPS</th><th>Cycle (s)</th></tr></thead><tbody>{}</tbody></table>
<h2>Anomalies</h2>
<ul>{}</ul>
</body>
</html>"#,
        html_escape(name),
        html_escape(name),
        declared_override_note,
        declared_rows,
        effective_rows,
        if anomaly_list.is_empty() {
            "<li>None</li>".to_string()
        } else {
            anomaly_list
        }
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{UnitId, UnitSummary, WeaponDeclared, WeaponEffective};

    #[test]
    fn html_report_sanity() {
        let units = vec![UnitSummary {
            unit_id: UnitId {
                id: "test01".to_string(),
                name: Some("Test Unit".to_string()),
            },
            blueprint_path: "test.lua".to_string(),
            weapons: vec![WeaponDeclared {
                weapon_bp_id: "W1".to_string(),
                damage: 10.0,
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
                target_categories: vec![],
            }],
            effective: vec![WeaponEffective {
                weapon_bp_id: "W1".to_string(),
                nominal_dps: 20.0,
                effective_dps: 20.0,
                cycle_time_sec: 0.5,
                shots_per_cycle: 1,
                salvo_duration_sec: 0.0,
                reload_sec: 0.5,
                target_class_modifiers: vec![],
            }],
            anomalies: vec![],
            declared_dps_override: None,
        }];
        let dir = tempfile::tempdir().unwrap();
        write_html_report(&units, dir.path()).unwrap();
        assert!(dir.path().join("index.html").exists());
        assert!(dir.path().join("anomalies.html").exists());
        assert!(dir.path().join("unit_test01.html").exists());
    }
}
