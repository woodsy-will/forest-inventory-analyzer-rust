# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Cruise format auto-detection**: Excel files from ArcGIS Survey123/Field Maps with `Plot_form` sheets are automatically recognized and imported, with BAF-to-TPA conversion for variable radius plots, per-log defect summation, and species code derivation
- **Height sanity check**: Tree heights exceeding 300 ft are flagged as data entry errors and excluded from volume calculations
- **GeoJSON export button** in web UI alongside CSV and JSON
- **Confidence level selector** (90/95/99%) in the statistics panel — change without re-uploading
- **Upload progress indicator** showing filename and spinner during file processing
- **"New Analysis" button** in the header to start over from the dashboard
- **Number formatting** with locale-aware comma separators for large values
- **Sampling error highlighting**: values >20% shown in red in the statistics table
- **Configuration file wired to web server**: `config.toml` settings for `server.port`, `server.max_upload_bytes`, `database.path` are now used by the web server instead of hardcoded constants

### Changed
- **QMD calculation corrected**: stand-level QMD now uses `sqrt(sum(EF * DBH^2) / sum(EF))` across all live trees instead of averaging per-plot QMDs, which was statistically incorrect
- **CORS policy restricted**: `allowed_origin` now set to `http://localhost:{port}` instead of allowing all origins
- **Eviction throttled**: TTL eviction queries run at most once per 60 seconds per table instead of on every database access
- **GeoJSON builder deduplicated**: web export now uses the same `build_geojson_value()` function as CLI export, fixing missing properties (`volume_cuft_per_acre`, `volume_bdft_per_acre`, `quadratic_mean_diameter`, `age`, `defect`) in web-exported GeoJSON
- **Web UI redesigned**: Inter font, sticky header with gradient, hover effects on metric cards, refined chart styling, better mobile responsiveness, section fade-in transitions
- **`AppState::new` accepts database path** parameter instead of hardcoding `"forest_analyzer.db"`

### Fixed
- Architecture documentation listed `TreeStatus::Ingrowth` but the actual enum variant is `Missing`

## [0.1.0] - 2024-12-10

### Added
- Core library with `ForestInventory`, `Plot`, `Tree`, `Species`, `TreeStatus` domain models
- Stand metrics computation: TPA, basal area, volume (cubic & board feet), QMD, species composition
- Statistical analysis with confidence intervals using Student's t-distribution
- Diameter distribution with configurable class width
- Growth projections: exponential, logistic, and linear models with configurable mortality
- Multi-format I/O: CSV, JSON, Excel (.xlsx) with trait-based `InventoryReader`/`InventoryWriter`
- GeoJSON export for plot data
- Batch processing CLI command (`analyze-batch`) for directory-wide analysis
- Format conversion between all supported formats
- Input validation with strict and lenient modes
- Web UI with Actix Web: file upload, interactive Chart.js dashboards, data editing, export
- SQLite persistence with TTL-based and capacity-based eviction
- In-browser data editor with click-to-jump error badges, loading spinner, mobile card layout
- Configurable volume equations (`VolumeEquation` struct)
- `Analyzer` struct as unified API for all analysis operations
- `AppConfig` with TOML-based configuration file support
- CORS middleware and health check endpoint
- CI/CD pipeline (GitHub Actions) with cross-platform testing, clippy, and fmt checks
- Architecture documentation (`docs/architecture.md`)
- Runnable examples: `basic_analysis`, `growth_projection`, `format_conversion`
- 307 tests: 231 unit, 16 CLI integration, 53 library integration, 7 doc-tests
