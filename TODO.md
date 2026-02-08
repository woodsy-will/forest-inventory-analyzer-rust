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

**Remaining medium-priority issues:**

- [ ] **Unify `validate` and `validate_all`** — Both methods in `tree.rs` duplicate the same 5 checks. Implement `validate_all` as canonical, then have `validate()` call it and return the first issue as `Err`. Prevents drift when adding new constraints.

- [ ] **Move `rows_to_inventory` to `io` module** — This helper in `handlers.rs` is effectively a fourth parser. It belongs in `io/` alongside the other data transformation functions to keep the web layer thin.

- [ ] **Consistent strict parsing for status** — `csv_io::parse_csv_records` rejects unknown status as a fatal error, but `excel_io::read_excel` silently defaults to Live with a `log::warn!`. Both strict parsers should behave the same way.

- [ ] **Add `NotFound` variant to `ForestError`** — Inventory-not-found currently uses `ForestError::ParseError`, which maps to HTTP 400. Should be a dedicated variant mapping to HTTP 404.

- [ ] **Handle Mutex poisoning gracefully** — `state.rs` methods use `.unwrap()` on `Mutex::lock()`. Replace with `.expect()` or propagate the error to prevent panic cascades if a thread panics while holding a lock.

- [ ] **Generate `ValidationIssue` for skipped Excel rows** — Both `read_excel` and `parse_excel_lenient` silently skip rows with <9 columns. The lenient parser should record these as issues instead of dropping data.

- [ ] **Use `Path` for filename parsing in upload handler** — `rsplit('.')` gives wrong stem for multi-dot filenames like `my.data.csv`. Use `Path::new(&filename).file_stem()` / `.extension()`.

**Lower-priority / nice-to-have:**

- [ ] **Error editor UX improvements** — Add loading spinner on "Validate & Save", confirmation dialog on "Start Over", click-to-jump on error badges, and table `<caption>` + `aria-label` on inputs for accessibility.

- [ ] **Mobile responsiveness for editor table** — The 12-column edit table is unusable on narrow screens. Consider column hiding or a card-based layout on mobile.

## Priority 5: Documentation & Polish

- [ ] **Add architecture documentation** — Create a `docs/architecture.md` explaining the 5-layer design (domain, data access, business logic, web, presentation), data flow, and module responsibilities.

- [ ] **Add library usage examples** — Add `examples/` directory with runnable examples showing programmatic use of the library (loading data, running analysis, accessing results).

- [ ] **Add CI/CD pipeline** — Set up GitHub Actions workflow for `cargo test`, `cargo clippy`, and `cargo fmt --check` on push/PR to master.

- [ ] **Database-backed persistence** — The in-memory `HashMap` storage for inventories won't scale for concurrent users or server restarts. Consider SQLite or similar for persistence.
