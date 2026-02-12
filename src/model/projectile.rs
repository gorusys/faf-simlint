//! Projectile blueprint extraction for fragment count and fragment damage.
//! The only reliable way to get actual fragment count is from projectiles data; weapon damage does not include fragments or DoT.

use crate::parser::LuaValue;

/// Parsed projectile data from a *_proj.bp file. Used to resolve fragment count (and optionally fragment damage) for weapons.
#[derive(Debug, Clone, Default)]
pub struct ProjectileData {
    /// Number of fragments spawned on impact (Physics.Fragments). Reliable source; weapon blueprint does not include this.
    pub fragment_count: Option<u32>,
    /// Path to fragment projectile (Physics.FragmentId); fragment damage may be on that projectile.
    pub fragment_id: Option<String>,
    /// Damage from this projectile (e.g. when used as a fragment); not all FAF bps define it.
    pub damage: Option<f64>,
}

/// Normalize projectile path for lookup: lowercase, consistent slashes.
pub fn normalize_projectile_path(path: &str) -> String {
    let s = path.trim().trim_matches('"').to_string();
    let s = s.replace('\\', "/");
    if s.to_lowercase().starts_with("/projectiles/") {
        s[1..].to_lowercase()
    } else if s.to_lowercase().starts_with("projectiles/") {
        s.to_lowercase()
    } else {
        s.to_lowercase()
    }
}

/// Extract projectile data from a parsed ProjectileBlueprint root table.
/// Reads Physics.Fragments, Physics.FragmentId, and optional Damage (for fragment projectiles).
/// Returns Default when no Physics or no fragment/damage fields (so every projectile can be stored for lookup).
pub fn projectile_from_lua(root: &LuaValue) -> Option<ProjectileData> {
    let physics = root.get_table("Physics")?;
    let fragment_count = physics.get_num("Fragments").map(|n| n as u32);
    let fragment_id = physics.get_str("FragmentId").map(str::to_string);
    let damage = root.get_num("Damage").or_else(|| physics.get_num("Damage"));
    Some(ProjectileData {
        fragment_count,
        fragment_id,
        damage,
    })
}
