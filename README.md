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

Use a **local path** to your FAF installation’s blueprint/weapon data. The tool does not fetch data; you point it at a directory or archive.

**Extract from gamedata (recommended):**  
Run `extract` to pull all `*_unit.bp` files from your FAF gamedata in one go:

```bash
# From gamedata folder, gamedata.scd (.zip), or FAF install root
./target/release/faf-simlint extract --gamedata /path/to/gamedata --out extracted_units
# Then scan the extracted folder
./target/release/faf-simlint scan --data-dir extracted_units --out out
```

- **gamedata folder:** expects `gamedata/units/` (or pass the `units` folder directly).
- **gamedata.scd / .zip:** reads the archive and extracts every `*_unit.bp` into `--out`.
- **FAF install root:** if you pass the install root, the command looks for `gamedata/` or `gamedata.scd` inside it.

You can also point `scan --data-dir` at any directory that already contains unit blueprint Lua/`.bp` files (e.g. from a mod).

**Real FAF / fa project layout:**  
To get correct data (including **fragmented weapons** like T1 Seraphim arty or Salvation), the tool must scan both the **units** folder and the **projectiles** folder. Weapon blueprint damage does not include fragments or DoT; the only reliable way to get actual fragment count is from projectiles data.

- Point `--data-dir` at the **repo root** (e.g. `/path/to/fa`): the tool will scan `units/` and `projectiles/` automatically.
- Or point at `units/` only: then projectiles are not loaded (fragment count will be missing for fragmentation weapons).
- `*_script.lua` files are never parsed (they are behavior scripts, not blueprints); parsing them would cause “trailing content” errors.

```bash
# Recommended: scan repo root so both units and projectiles are loaded (fragment data)
./target/release/faf-simlint scan --data-dir /path/to/fa --out out

# Or scan only units (no fragment data from projectiles)
./target/release/faf-simlint scan --data-dir /path/to/fa/units --out out

# Extract only pulls unit blueprints; use scan on repo root for full fragment data
./target/release/faf-simlint extract --gamedata /path/to/fa --out extracted_units
./target/release/faf-simlint scan --data-dir extracted_units --out out
```

## Quickstart

```bash
# Build
cargo build --release

# 1) Extract unit blueprints from gamedata (optional)
./target/release/faf-simlint extract --gamedata /path/to/faf/gamedata --out extracted_units

# 2) Scan; writes SQLite + JSON + HTML under out/
./target/release/faf-simlint scan --data-dir extracted_units --out out
# Or with declared DPS overrides (see below):
./target/release/faf-simlint scan --data-dir extracted_units --out out --declared-dps declared.json

# 3) Summarize one unit (from last scan in the DB)
./target/release/faf-simlint unit --scan-db out/scan.sqlite uel0101

# Or from data dir only (no prior scan)
./target/release/faf-simlint unit --data-dir extracted_units uel0101

# 4) Compare two scans (e.g. before/after a patch)
./target/release/faf-simlint diff --a out1/scan.sqlite --b out2/scan.sqlite --out diff_out
```

## Effective DPS vs declared

- **Declared / nominal:** From the blueprint (or from an override file, see below). Note: **ProjectilesPerOnFire is deprecated** in FAF; the game uses **RackSalvoSize**, **MuzzleSalvoSize**, **MuzzleSalvoDelay**, and **RackSalvoReloadTime**. **Weapon Damage does not include fragments or DoT**; the tool adds **InitialDamage** (e.g. UEF T1 bomber) and fragment damage from **projectiles** data when available (scan with `--data-dir` pointing at repo root so both `units/` and `projectiles/` are loaded).
- **Effective:** Computed from total damage per shot (weapon Damage + InitialDamage + fragment count × fragment damage), rate, salvo, and reload.

**Import your own declared DPS:**  
To compare against wiki/balance/measured values instead of blueprint-derived nominal, use a JSON file:

```bash
./target/release/faf-simlint scan --data-dir extracted_units --out out --declared-dps declared.json
```

Format: `{ "unit_id": dps_number, ... }` (e.g. `{ "uel0101": 20.5, "ual0107": 12 }`). Unit IDs are matched case-insensitively. The report then shows “Declared DPS (from override)” and compares it to the sum of effective weapon DPS for that unit.

## Sharing the HTML report

- Run `scan --out out`; open `out/html/index.html` in a browser.
- Use “Anomalies” for multi-weapon interference and other flags.
- Per-unit pages show declared vs effective and a short “why” summary.
- To share: zip `out/html` or host it; the report is static (no server required). When served locally (e.g. `python3 -m http.server` in `out/html`), you can share a link to the report.

## Adding more parsers / fixtures

- **Fixtures:** Add Lua or `.bp` files under `fixtures/units` or `fixtures/weapons`; they are used by tests and as examples.
- **Declared DPS:** Use `fixtures/declared_dps_example.json` as a template for `--declared-dps` (unit ID → DPS map).
- **Parsers:** The parser lives in `src/parser` (Lua-like tables, strings, numbers, booleans; no execution). Weapon fields are in `src/model/extract.rs`: FAF uses `RackSalvoSize`, `MuzzleSalvoSize`, `MuzzleSalvoDelay`, `RackSalvoReloadTime`; legacy `ProjectilesPerOnFire` is deprecated and only used as fallback when salvo fields are missing.

## License

MIT. See [LICENSE](LICENSE).
