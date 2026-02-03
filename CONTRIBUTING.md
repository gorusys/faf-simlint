# Contributing to faf-simlint

## Development setup

- Rust stable
- `cargo test` — run tests
- `cargo fmt` — format
- `cargo clippy --all-targets --all-features -- -D warnings` — lint

## Commit style

Use [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `build:`, `ci:`.

## Scope

This project is a **weapon behavior auditor**: it reads FAF unit/weapon data, computes effective behavior, detects anomalies, and produces reports. It is not a Discord bot, node monitor, or generic wiki tool.

## No placeholders

- No TODO/FIXME/XXX in committed code
- No empty implementations or “stub” / “coming soon”
- Every file must be functional and meaningful

## Testing

- Parser tests in `src/parser/mod.rs` (fixture blueprint snippets)
- Weapon model tests in `src/model/mod.rs`
- Scheduler and anomaly tests in `src/scheduler`, `src/anomaly`
- Integration tests in `tests/integration.rs` (scan, unit, diff)
- Report sanity in `src/report/mod.rs`

Add or extend fixtures under `fixtures/` as needed.
