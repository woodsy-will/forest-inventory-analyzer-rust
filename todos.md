# Project Todos

## Active

## Completed
- [x] Add `cargo-audit` security vulnerability scanning to CI pipeline | Done: 03-14-2026
- [x] Add fuzz testing for CSV/JSON/Excel parsers with `cargo-fuzz` or `afl` | Done: 03-14-2026
- [x] Replace `env_logger` with `tracing` crate for structured logging with request spans | Done: 03-14-2026
- [x] Implement `FromStr` for `GrowthModel` to eliminate string-based model selection in main.rs | Done: 03-14-2026
- [x] Single-pass metrics calculation — `compute_stand_metrics` calls `mean_tpa()`, `mean_basal_area()`, etc. separately, each iterating all plots | Done: 03-14-2026
- [x] Add property-based tests with `proptest` for statistical calculations (CI always positive, mean within bounds) | Done: 03-14-2026
- [x] Add `cargo-deny` to CI for license and duplicate dependency checking | Done: 03-14-2026
- [x] Add code coverage tracking with `cargo-llvm-cov` and badge in README | Done: 03-14-2026
- [x] Add clap value parser for confidence level (0.0-1.0 range validation at parse time, not runtime) | Done: 03-14-2026
- [x] Add request logging middleware to Actix Web (method, path, status, duration) | Done: 03-14-2026
- [x] Add criterion benchmark suite for hot paths: metrics computation, diameter distribution, BAF-to-TPA conversion | Done: 03-14-2026
- [x] Add MSRV (Minimum Supported Rust Version) check to CI matrix | Done: 03-14-2026
- [x] Add graceful shutdown handling to web server (SIGTERM/SIGINT with in-flight request drain) | Done: 03-14-2026
- [x] Add module-level `//!` doc comments to `analysis`, `io`, `visualization`, and `web` modules | Done: 03-14-2026
- [x] Extract file extension dispatch into shared helper — duplicated in `load_inventory()` and `Commands::Convert` | Done: 03-14-2026
- [x] Add snapshot tests with `insta` crate for visualization output (tables, histograms) | Done: 03-14-2026
- [x] Add integration test for `analyze-batch` CLI command with multi-file directory | Done: 03-14-2026
- [x] Replace `String` fields in `ValidationIssue` with `Cow<'static, str>` to reduce allocations | Done: 03-14-2026
- [x] Add streaming upload size enforcement — reject oversized payloads before buffering entire file | Done: 03-14-2026
- [x] Add per-stand summary grouping when cruise data contains multiple stands | Done: 03-14-2026
