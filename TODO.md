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

- [x] **Error editor UX improvements** — Add loading spinner on "Validate & Save", confirmation dialog on "Start Over", click-to-jump on error badges, and table `<caption>` + `aria-label` on inputs for accessibility.

- [x] **Mobile responsiveness for editor table** — The 12-column edit table is unusable on narrow screens. Consider column hiding or a card-based layout on mobile.

## Priority 5: Documentation & Polish

- [x] **Add architecture documentation** — Create a `docs/architecture.md` explaining the 5-layer design (domain, data access, business logic, web, presentation), data flow, and module responsibilities.

- [x] **Add library usage examples** — Add `examples/` directory with runnable examples showing programmatic use of the library (loading data, running analysis, accessing results).

- [x] **Add CI/CD pipeline** — Set up GitHub Actions workflow for `cargo test`, `cargo clippy`, and `cargo fmt --check` on push/PR to master.

- [x] **Database-backed persistence** — The in-memory `HashMap` storage for inventories won't scale for concurrent users or server restarts. Consider SQLite or similar for persistence.

---

## Priority 6: Test Coverage & Robustness

- [x] **Add web handler integration tests** — `handlers.rs` (500+ lines) has zero tests. Add `actix_web::test` tests for: upload with valid CSV/JSON/Excel, validation error flow (invalid → edit → resubmit), UUID provenance rejection for unknown IDs, export with special characters in filenames, and error responses (404, 400, 422).

- [x] **Add SQLite state unit tests** — `state.rs` has no tests for the new SQLite persistence layer. Test: insert/get/remove round-trips, TTL eviction (insert with past timestamp, verify removal), capacity eviction (exceed `MAX_INVENTORIES`/`MAX_PENDING`, verify oldest removed), and behavior when DB file is missing or corrupted.

- [x] **Add I/O edge case tests** — Test: Excel files with 0 data rows, empty cells, and mixed types; CSV with UTF-8 BOM, Windows line endings (`\r\n`), and quoted commas in species names; JSON with extra/missing fields and null optional values.

## Priority 7: Error Handling & Resilience

- [x] **Return Result from AppState methods** — `state.rs` uses `expect()` for DB open, table creation, serialization, and every insert/select. These panic and crash the web server. Change `AppState::new()` to return `Result<Self, ForestError>` and propagate errors from `insert_inventory`, `insert_pending`, etc. through to handler responses.

- [x] **Add CORS middleware** — No CORS headers are configured in `src/web/mod.rs`. Add `actix-cors` with a restrictive default policy so the API can be safely called from other origins without exposing it to cross-origin attacks.

- [x] **Add health check endpoint** — No `/health` or `/ready` endpoint exists. Add a lightweight `GET /health` handler returning 200 for use with load balancers, Kubernetes probes, and uptime monitors.

## Priority 8: Publishing & Developer Experience

- [x] **Fix README placeholders** — `README.md` lines 3 and 21 contain `YOUR_USERNAME`. Replace with actual GitHub username. Add a section documenting the `examples/` directory and `cargo run --example` commands. Mention the web UI feature and `cargo run -- serve`.

- [x] **Add Cargo.toml publishing metadata** — Missing `repository`, `documentation`, and `homepage` fields needed for crates.io. Add `exclude` to keep `.github/`, `.claude/`, and test fixtures out of the published crate.

- [x] **Expand library re-exports** — `lib.rs` re-exports `Analyzer` but not `GrowthModel`, `DiameterDistribution`, `StandMetrics`, `SamplingStatistics`, `GrowthProjection`, or `ConfidenceInterval`. Add these so users don't need to reach into submodules for common types.

- [x] **Add doc comments with examples to public API** — Public methods like `Tree::basal_area_sqft`, `Tree::volume_cuft`, `Plot::trees_per_acre`, and `ForestInventory::mean_tpa` lack doc-test examples. Add `/// # Examples` blocks so `cargo doc` and docs.rs show usage inline.

## Priority 9: Features & Performance

- [ ] **Add batch processing CLI command** — Currently the CLI processes one file at a time. Add `analyze-batch --input-dir ./inventories/ --output-dir ./reports/` to process multiple inventory files and optionally produce aggregated cross-inventory statistics.

- [ ] **Add GeoJSON export** — Plots have elevation, aspect, and slope attributes but no geospatial export. Add `GeoJsonFormat` implementing `InventoryWriter`, and add `format=geojson` to the web export endpoint. Use the `geojson` crate.

- [ ] **Optimize species_list deduplication** — `ForestInventory::species_list()` in `inventory.rs` collects all species into a Vec, sorts, and deduplicates. Use a `HashSet` or `IndexSet` for O(n) dedup instead of O(n log n) sort+dedup, which matters on inventories with 1000+ trees.

- [ ] **Add configuration file support** — All settings (DB path, upload size limit, default growth model params, server port) are hardcoded or CLI-only. Add optional `config.toml` support via the `config` crate so deployment-specific settings persist without CLI flags.
