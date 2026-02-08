# Forest Inventory Analyzer - TODO

## Priority 1: Quick Wins

- [x] **Add input validation on data load** — Validate DBH > 0, 0 <= crown_ratio <= 1.0, height > 0 when present, expansion_factor > 0. Use the existing `ValidationError` variant which is currently defined but never used anywhere in the codebase. Apply validation in `csv_io::read_csv`, `json_io::read_json`, and `excel_io::read_excel`.

- [x] **Log warning for invalid tree status in Excel reader** — `excel_io.rs` silently defaults invalid status strings to `TreeStatus::Live` via `unwrap_or(TreeStatus::Live)`. Replace with `log::warn!()` to alert users about data quality issues while still loading the file.

- [x] **Remove or use `indicatif` crate** — `indicatif` is listed in Cargo.toml dependencies but never used in the codebase. Either remove it or add progress bars to large file load/write operations.

## Priority 2: Structural Improvements

- [x] **Extract volume equations into configurable structs** — The hardcoded coefficients `b1 = 0.002454` (cubic foot) and `b1 = 0.01159` (board foot Scribner) in `tree.rs` are general-purpose approximations. Define a `VolumeEquation` trait or struct that allows species-specific coefficients to be injected, improving accuracy for real-world use.

- [x] **Make mortality rate configurable in growth models** — The hardcoded `0.005` (0.5% annual) mortality rate in `growth.rs` is embedded in all three model variants. Add a `mortality_rate` field to `GrowthModel::Exponential`, `GrowthModel::Logistic`, and `GrowthModel::Linear` so users can adjust mortality assumptions.

- [x] **Add convenience re-exports to lib.rs** — Add `pub use models::{Tree, Plot, ForestInventory, Species, TreeStatus};` to `lib.rs` so library consumers can use `forest_inventory_analyzer::Tree` instead of `forest_inventory_analyzer::models::Tree`.

## Priority 3: Extensibility & Future-Proofing

- [x] **Define I/O traits for reader/writer abstraction** — Create `InventoryReader` and `InventoryWriter` traits that `csv_io`, `json_io`, and `excel_io` implement. This enables adding new formats (GeoJSON, SQLite, Shapefile) without modifying existing code, and allows consumers to implement custom readers.

- [x] **Group analysis functions under a configurable struct** — Currently `compute_stand_metrics()`, `SamplingStatistics::compute()`, and `project_growth()` are standalone functions. An `Analyzer` struct could hold default configuration (confidence level, diameter class width, growth model) and provide a unified API.

- [x] **Add visualization tests** — `tables.rs` and `charts.rs` have no tests. Capture stdout output in tests to verify table formatting and histogram rendering don't panic and produce expected content structure.

- [x] **Add CLI integration tests** — No tests for `main.rs` CLI argument parsing or subcommand dispatch. Use `assert_cmd` or similar crate to test that the binary handles valid/invalid inputs correctly.

## Priority 4: Web UI & Validation

- [x] **Lenient validation with in-browser data editor** — Upload collects all validation errors instead of failing on the first. Returns editable rows so users can fix values in-browser and resubmit via `/api/validate`.

- [x] **Security hardening** — XSS prevention (DOM APIs instead of innerHTML), Content-Disposition header sanitization, UUID provenance checks on `/api/validate`, TTL-based memory eviction for pending_rows/inventories, explicit upload size limits (50 MB).

### Architecture Review Notes (from code review)

**Medium-priority issues (resolved):**

- [x] **Unify `validate` and `validate_all`** — `validate()` now delegates to `validate_all()` and returns the first issue as `Err`. Single source of truth for all validation checks.

- [x] **Move `rows_to_inventory` to `io` module** — Moved from `handlers.rs` to `csv_io.rs`, re-exported via `io/mod.rs`. Web layer no longer owns data transformation logic.

- [x] **Consistent strict parsing for status** — `excel_io::read_excel` now rejects unknown status as a fatal `ParseError`, matching `csv_io::parse_csv_records` behavior.

- [x] **Add `NotFound` variant to `ForestError`** — Added `NotFound(String)` variant mapped to HTTP 404. All inventory-not-found errors now use it instead of `ParseError`.

- [x] **Handle Mutex poisoning gracefully** — Replaced `.unwrap()` with `.expect("descriptive message")` on all `Mutex::lock()` calls in `state.rs`.

- [x] **Generate `ValidationIssue` for skipped Excel rows** — `parse_excel_lenient` now records a `ValidationIssue` for rows with <9 columns instead of silently dropping them.

- [x] **Use `Path` for filename parsing in upload handler** — Replaced `rsplit('.')` with `Path::file_stem()` / `Path::extension()` for correct multi-dot filename handling.

**Lower-priority / nice-to-have:**

- [ ] **Error editor UX improvements** — Add loading spinner on "Validate & Save", confirmation dialog on "Start Over", click-to-jump on error badges, and table `<caption>` + `aria-label` on inputs for accessibility.

- [ ] **Mobile responsiveness for editor table** — The 12-column edit table is unusable on narrow screens. Consider column hiding or a card-based layout on mobile.

## Priority 5: Documentation & Polish

- [ ] **Add architecture documentation** — Create a `docs/architecture.md` explaining the 5-layer design (domain, data access, business logic, web, presentation), data flow, and module responsibilities.

- [ ] **Add library usage examples** — Add `examples/` directory with runnable examples showing programmatic use of the library (loading data, running analysis, accessing results).

- [ ] **Add CI/CD pipeline** — Set up GitHub Actions workflow for `cargo test`, `cargo clippy`, and `cargo fmt --check` on push/PR to master.

- [ ] **Database-backed persistence** — The in-memory `HashMap` storage for inventories won't scale for concurrent users or server restarts. Consider SQLite or similar for persistence.
