# Forest Inventory Analyzer - TODO

## Priority 1: Quick Wins

- [ ] **Add input validation on data load** — Validate DBH > 0, 0 <= crown_ratio <= 1.0, height > 0 when present, expansion_factor > 0. Use the existing `ValidationError` variant which is currently defined but never used anywhere in the codebase. Apply validation in `csv_io::read_csv`, `json_io::read_json`, and `excel_io::read_excel`.

- [ ] **Log warning for invalid tree status in Excel reader** — `excel_io.rs` silently defaults invalid status strings to `TreeStatus::Live` via `unwrap_or(TreeStatus::Live)`. Replace with `log::warn!()` to alert users about data quality issues while still loading the file.

- [ ] **Remove or use `indicatif` crate** — `indicatif` is listed in Cargo.toml dependencies but never used in the codebase. Either remove it or add progress bars to large file load/write operations.

## Priority 2: Structural Improvements

- [ ] **Extract volume equations into configurable structs** — The hardcoded coefficients `b1 = 0.002454` (cubic foot) and `b1 = 0.01159` (board foot Scribner) in `tree.rs` are general-purpose approximations. Define a `VolumeEquation` trait or struct that allows species-specific coefficients to be injected, improving accuracy for real-world use.

- [ ] **Make mortality rate configurable in growth models** — The hardcoded `0.005` (0.5% annual) mortality rate in `growth.rs` is embedded in all three model variants. Add a `mortality_rate` field to `GrowthModel::Exponential`, `GrowthModel::Logistic`, and `GrowthModel::Linear` so users can adjust mortality assumptions.

- [ ] **Add convenience re-exports to lib.rs** — Add `pub use models::{Tree, Plot, ForestInventory, Species, TreeStatus};` to `lib.rs` so library consumers can use `forest_inventory_analyzer::Tree` instead of `forest_inventory_analyzer::models::Tree`.

## Priority 3: Extensibility & Future-Proofing

- [ ] **Define I/O traits for reader/writer abstraction** — Create `InventoryReader` and `InventoryWriter` traits that `csv_io`, `json_io`, and `excel_io` implement. This enables adding new formats (GeoJSON, SQLite, Shapefile) without modifying existing code, and allows consumers to implement custom readers.

- [ ] **Group analysis functions under a configurable struct** — Currently `compute_stand_metrics()`, `SamplingStatistics::compute()`, and `project_growth()` are standalone functions. An `Analyzer` struct could hold default configuration (confidence level, diameter class width, growth model) and provide a unified API.

- [ ] **Add visualization tests** — `tables.rs` and `charts.rs` have no tests. Capture stdout output in tests to verify table formatting and histogram rendering don't panic and produce expected content structure.

- [ ] **Add CLI integration tests** — No tests for `main.rs` CLI argument parsing or subcommand dispatch. Use `assert_cmd` or similar crate to test that the binary handles valid/invalid inputs correctly.

## Priority 4: Documentation & Polish

- [ ] **Add architecture documentation** — Create a `docs/architecture.md` explaining the 4-layer design (domain, data access, business logic, presentation), data flow, and module responsibilities.

- [ ] **Add library usage examples** — Add `examples/` directory with runnable examples showing programmatic use of the library (loading data, running analysis, accessing results).

- [ ] **Add CI/CD pipeline** — Set up GitHub Actions workflow for `cargo test`, `cargo clippy`, and `cargo fmt --check` on push/PR to master.
