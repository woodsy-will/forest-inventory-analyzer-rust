#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Feed arbitrary bytes to CSV parser. It should never panic,
    // only return Ok or Err.
    let _ = forest_inventory_analyzer::io::read_csv_from_bytes(data, "fuzz");
});
