# Architecture

## Overview

Forest Inventory Analyzer is a Rust application with three execution modes:

- **Library** (`forest_inventory_analyzer`) — programmatic API for loading, analyzing, and exporting forest inventory data
- **CLI** (`forest-analyzer`) — command-line interface with five subcommands
- **Web server** (feature-gated `web`) — Actix Web server with file upload, analysis dashboards, and data editing

## Data Flow

```
 Input Files              I/O Layer            Domain Models
 ──────────────          ──────────           ──────────────
 ┌─────────┐    read     ┌──────────┐  parse  ┌──────────────────┐
 │ CSV      │───────────>│ CsvFormat │───────>│ ForestInventory  │
 │ JSON     │───────────>│ JsonFormat│───────>│   └─ Plot[]      │
 │ Excel    │───────────>│ExcelFormat│───────>│       └─ Tree[]  │
 └─────────┘             └──────────┘         └────────┬─────────┘
                                                       │
                              Analysis Layer           │
                              ──────────────           │
                              ┌─────────────┐          │
                              │  Analyzer    │<─────────┘
                              │  ├ metrics() │
                              │  ├ stats()   │
                              │  ├ dist()    │
                              │  └ growth()  │
                              └──────┬──────┘
                                     │
                  ┌──────────────────┼──────────────────┐
                  │                  │                   │
          Visualization         Web Layer           Output Files
          ─────────────        ─────────           ────────────
          ┌───────────┐     ┌────────────┐     ┌─────────────┐
          │ Tables     │     │ Actix Web  │     │ CSV / JSON  │
          │ Histograms │     │ REST API   │     │ Excel       │
          └───────────┘     │ Dashboard  │     └─────────────┘
                            └────────────┘
```

## Layer Descriptions

### Models (`src/models/`)

Domain types representing forest inventory data.

| Type | Description |
|------|-------------|
| `Tree` | Individual tree record (DBH, height, species, status, expansion factor) |
| `Plot` | Sample plot containing trees with site attributes (size, slope, aspect, elevation) |
| `ForestInventory` | Root type: named collection of plots with aggregate methods |
| `Species` | Species code + common name pair |
| `VolumeEquation` | Configurable volume equation coefficients (cubic foot and board foot) |
| `TreeStatus` | Enum: `Live`, `Dead`, `Cut`, `Ingrowth` |
| `ValidationIssue` | Field-level validation error (plot, tree, row, field, message) |

### I/O (`src/io/`)

File reading/writing with trait-based abstraction.

| Type | Description |
|------|-------------|
| `InventoryReader` | Trait: `fn read(&self, path) -> Result<ForestInventory>` |
| `InventoryWriter` | Trait: `fn write(&self, inventory, path) -> Result<()>` |
| `CsvFormat` | Implements both traits for CSV files |
| `JsonFormat` | Implements both traits for JSON (with `pretty` option) |
| `ExcelFormat` | Implements both traits for `.xlsx` files |

Lenient parsing functions (`parse_csv_lenient`, `parse_json_lenient`, `parse_excel_lenient`) collect all validation issues instead of failing on the first error, enabling the web UI's in-browser data editor.

### Analysis (`src/analysis/`)

Statistical computations and growth modeling.

| Type | Description |
|------|-------------|
| `Analyzer` | Unified API grouping all analysis operations on an inventory reference |
| `StandMetrics` | Per-acre stand summary: TPA, basal area, volume, QMD, species composition |
| `SamplingStatistics` | Confidence intervals for TPA, BA, and volume across plots |
| `DiameterDistribution` | Diameter class frequency distribution with configurable class width |
| `GrowthModel` | Enum: `Exponential`, `Logistic`, `Linear` — each with configurable mortality rate |
| `GrowthProjection` | Year-by-year projected TPA, BA, volume, and mortality |

### Visualization (`src/visualization/`)

Text-based output formatting using `comfy-table`.

| Function | Description |
|----------|-------------|
| `print_stand_summary` / `format_stand_summary` | Formatted stand metrics table |
| `print_species_table` / `format_species_table` | Species composition breakdown |
| `print_statistics_table` / `format_statistics_table` | Sampling statistics with confidence intervals |
| `print_growth_table` / `format_growth_table` | Year-by-year growth projection table |
| `print_diameter_histogram` / `format_diameter_histogram` | ASCII bar chart of diameter classes |

Each `print_*` function writes to stdout; `format_*` returns a `String` for testing or embedding.

### Web (`src/web/`, feature-gated)

Actix Web server providing a REST API and embedded single-page dashboard.

| Component | Description |
|-----------|-------------|
| `start_server(port)` | Configures routes, payload limits (50 MB), and launches the server |
| `handlers.rs` | Request handlers: upload, validate, metrics, statistics, distribution, growth, export |
| `state.rs` | `AppState` with SQLite-backed persistence (inventories + pending editable rows) |
| `static/` | Embedded HTML/JS/CSS dashboard with Chart.js visualizations |

API endpoints:
- `POST /api/upload` — multipart file upload (CSV/JSON/Excel)
- `POST /api/validate` — revalidate edited rows and promote to inventory
- `GET /api/{id}/metrics` — stand metrics JSON
- `GET /api/{id}/statistics?confidence=0.95` — sampling statistics JSON
- `GET /api/{id}/distribution?class_width=2` — diameter distribution JSON
- `POST /api/{id}/growth` — growth projection JSON
- `GET /api/{id}/export?format=csv` — download as CSV or JSON
- `GET /api/{id}/inventory` — raw inventory JSON

### CLI (`src/main.rs`)

Five subcommands built with `clap` derive:

| Command | Description |
|---------|-------------|
| `analyze` | Full analysis: metrics, species table, diameter histogram, sampling statistics |
| `growth` | Growth projection with configurable model, rate, capacity, and mortality |
| `convert` | Format conversion between CSV, JSON, and Excel |
| `summary` | Quick one-screen inventory summary |
| `serve` | Start the web UI (requires `web` feature) |

### Error (`src/error.rs`)

`ForestError` enum with 9 variants:

| Variant | Description |
|---------|-------------|
| `Io` | File system errors |
| `Csv` | CSV parsing errors |
| `Json` | JSON serialization errors |
| `Excel` | Excel read/write errors |
| `ParseError` | Data format/type errors |
| `ValidationError` | Domain validation failures (DBH, height, crown ratio) |
| `AnalysisError` | Computation errors |
| `InsufficientData` | Not enough data for analysis (e.g., < 2 plots for statistics) |
| `NotFound` | Resource not found (maps to HTTP 404 in web layer) |

## Feature Flags

| Feature | Default | Dependencies Added |
|---------|---------|-------------------|
| `web` | Yes | `actix-web`, `actix-multipart`, `tokio`, `uuid`, `futures`, `mime`, `rusqlite` |

Disable with `cargo build --no-default-features` for a minimal library + CLI without the web server.

## Dependencies

| Category | Crates |
|----------|--------|
| CLI | `clap` (derive) |
| Serialization | `serde`, `serde_json`, `csv`, `calamine`, `rust_xlsxwriter` |
| Statistics | `statrs` |
| Error handling | `thiserror`, `anyhow` |
| Output | `comfy-table`, `colored` |
| Logging | `log`, `env_logger` |
| Temp files | `tempfile` |
| Web (optional) | `actix-web`, `actix-multipart`, `tokio`, `uuid`, `futures`, `mime`, `rusqlite` |
