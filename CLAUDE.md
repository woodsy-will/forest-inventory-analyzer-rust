# Forest Inventory Analyzer — Developer Guide

## Architecture

Six-layer design with strict downward-only dependencies:

```
CLI (main.rs)  ←→  Web (web/)
       ↓               ↓
   Visualization    Analysis
       ↓               ↓
         I/O (io/)
            ↓
       Models (models/)
```

Feature gate: `web` (default on) adds `actix-web`, `rusqlite`, `tokio`, `uuid`, `actix-cors`.
Build without: `cargo build --no-default-features`.

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `ForestInventory` | models/inventory.rs | Root domain type: named collection of Plots |
| `Plot` | models/plot.rs | Sample plot with trees and site attributes |
| `Tree` | models/tree.rs | Individual tree record with DBH, height, species, status |
| `VolumeEquation` | models/volume.rs | Configurable volume coefficients (cuft/bdft) |
| `Analyzer<'a>` | analysis/analyzer.rs | Unified analysis API over an inventory reference |
| `StandMetrics` | analysis/metrics.rs | Per-acre stand summary (TPA, BA, volume, QMD) |
| `SamplingStatistics` | analysis/statistics.rs | Confidence intervals via Student's t |
| `GrowthModel` | analysis/growth.rs | Enum: Exponential, Logistic, Linear (each with mortality) |
| `AppState` | web/state.rs | SQLite-backed persistence with TTL eviction |
| `AppConfig` | config.rs | TOML config: server, analysis, growth, database sections |
| `ForestError` | error.rs | 10-variant error enum mapped to HTTP status codes |

## Data Flow

**CLI**: `load_inventory(path)` → `compute_stand_metrics()` → `print_stand_summary()`

**Web upload**: `POST /api/upload` → `parse_*_lenient()` → `EditableTreeRow[]` + `ValidationIssue[]` → browser editor → `POST /api/validate` → `rows_to_inventory()` → stored in SQLite → `GET /api/{id}/metrics`

**Cruise import**: Auto-detected by `Plot_form` sheet names. BAF→TPA conversion for variable plots, per-log defect summation, species code derivation. Heights >300ft flagged as data entry errors.

## Build Environment (Windows)

Git Bash shadows MSVC `link.exe`. Fix:
```bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64:$PATH"
```

Use `edition = "2021"` (not 2024) for compatibility.

## Key Design Decisions

- **Expansion factor** = trees-per-acre this sample tree represents (not plot weight)
- **plot_size_acres** stored but NOT used in calculations — all plots equally weighted
- **Defect** is a proportion (0.0–1.0) applied multiplicatively to volume
- **Height optional**: volume returns `None` if height absent (no DBH-height estimation)
- **QMD** = `sqrt(Σ(EF × DBH²) / Σ(EF))` across all live trees (not averaged per-plot)
- **Species matching** by code only — same code with different common_name treated as same
- **Eviction** is lazy (on next read), throttled to at most once per 60 seconds
- **Static files** embedded via `include_str!` — must rebuild to see changes

## Gotchas

- `calamine` crate requires importing `DataType` trait to use `get_float()`
- Board foot volume returns 0 for DBH < `bdft_min_dbh` (default 6")
- Linear growth volume increment uses heuristic multipliers (10× for cuft, 50× for bdft)
- CORS restricted to `http://localhost:{port}` — update if deploying behind a domain
- `rows_to_inventory()` silently defaults unknown status to `Live` without recording a ValidationIssue

## Commands

```bash
cargo test --all-features          # 307 tests
cargo clippy --all-features        # lint
cargo run --all-features -- serve  # web UI on :8080
cargo run -- analyze --input file.csv
cargo run -- analyze-batch --input-dir ./data/ --output-dir ./reports/
```

## Releasing

Tag a version and push to trigger the automated release pipeline:

```bash
git tag v0.2.0
git push origin v0.2.0
```

This builds binaries for 4 platforms (Windows, Linux, macOS Intel, macOS ARM), creates a Windows MSI installer via cargo-wix, generates SHA256 checksums, and publishes a GitHub Release with all artifacts.

- WiX config: `wix/main.wxs` — version is auto-injected from the git tag
- Release workflow: `.github/workflows/release.yml`
- The `UpgradeCode` in `main.wxs` must never change (MSI identity across versions)

## Test Coverage

- 231 unit tests (models, analysis, I/O, config, error, visualization, web handlers, web state, cruise import)
- 16 CLI integration tests (assert_cmd)
- 53 library integration tests (end-to-end workflows, format conversion, edge cases)
- 7 doc-tests on public API methods
