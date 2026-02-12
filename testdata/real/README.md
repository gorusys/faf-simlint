# Real FAF test data

Minimal copy of real FAF unit and projectile blueprints used by integration tests.

- **units/** — 5 units: UEL0101, UEL0103, UEA0103, UEB2303, XSL0304 (from `/path/to/fa/units`).
- **projectiles/** — 6 projectiles referenced by those units (for fragment count and weapon resolution).

Tests in `tests/integration.rs` (e.g. `real_data_scan_succeeds_and_produces_five_units`) run the CLI against this directory and assert on parsed results (weapon damage, InitialDamage, fragment count from projectiles).
