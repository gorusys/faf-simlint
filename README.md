# faf-simlint — FAF Unit Weapon Behavior Auditor

**Static analyzer + micro-simulator** for Forged Alliance Forever (FAF) unit and weapon blueprints. It produces reproducible, shareable reports that explain *effective* behavior, not just declared blueprint values.

## Why this exists

Community confusion often looks like:

- “Unit stats say X DPS but it loses 1v1”
- Hidden cadence/ROF issues when a unit fires multiple weapons
- Debates about which unit database is accurate

faf-simlint helps by:

- Parsing FAF unit/weapon Lua data (no execution, safe and deterministic)
- Computing **effective DPS** and cadence from rate of fire, salvo, reload, and target class
- Detecting **multi-weapon cadence interference** with a micro-scheduler
- Emitting **anomaly reports** (INFO/WARN/CRIT) with user- and dev-facing explanations
- Storing scan history in SQLite and generating JSON + HTML reports you can share (e.g. in Discord)

## What it does and does not do

- **Does:** Read local FAF blueprint/weapon Lua, compute effective behavior, flag anomalies, diff two scans, generate static HTML reports.
- **Does not:** Connect to the network, execute Lua, or modify game files. It does not replace in-game testing; it complements it with reproducible numbers and explanations.

## Getting FAF data

Use a **local path** to your FAF installation’s blueprint/weapon data (e.g. after extracting game archives or from a mod). The tool does not fetch data; you point it at a directory.

Typical locations (you may need to extract from `.zip`/game files):

- FAF game data: e.g. `gamedata/` or similar under your FAF install
- Mods: mod-specific unit/weapon Lua under the mod’s folder

Provide this path via `--data-dir` for `scan` and `unit` (when not using a prior scan DB).

## Quickstart

```bash
# Build
cargo build --release

# Scan a directory of unit/weapon Lua; writes SQLite + JSON + HTML under out/
./target/release/faf-simlint scan --data-dir /path/to/faf/units --out out

# Summarize one unit (from last scan in the DB)
./target/release/faf-simlint unit --scan-db out/scan.sqlite uel0101

# Or from data dir only (no prior scan)
./target/release/faf-simlint unit --data-dir /path/to/faf/units uel0101

# Compare two scans (e.g. before/after a patch)
./target/release/faf-simlint diff --a out1/scan.sqlite --b out2/scan.sqlite --out diff_out
```

## Effective DPS vs declared

- **Declared / nominal:** Often “damage × projectiles / rate” or what the blueprint states.
- **Effective:** Accounts for reload, salvo timing, and (when modeled) target class. Effective DPS is what the micro-simulator and reports use for comparisons and regressions.

If effective DPS is much lower than nominal, the report will flag it and explain (e.g. salvo/cooldown pattern).

## Sharing the HTML report

- Run `scan --out out`; open `out/html/index.html` in a browser.
- Use “Anomalies” for multi-weapon interference and other flags.
- Per-unit pages show declared vs effective and a short “why” summary.
- To share: zip `out/html` or host it; the report is static (no server required). When served locally (e.g. `python3 -m http.server` in `out/html`), you can share a link to the report.

## Adding more parsers / fixtures

- **Fixtures:** Add Lua files under `fixtures/units` or `fixtures/weapons`; they are used by tests and as examples.
- **Parsers:** The parser lives in `src/parser` (Lua-like tables, strings, numbers, booleans; no execution). Extend field names in `src/model/extract.rs` (e.g. `BlueprintId`, `Damage`, `RateOfFire`) to match more FAF formats.

## License

MIT. See [LICENSE](LICENSE).
