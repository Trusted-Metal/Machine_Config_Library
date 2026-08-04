// Machine Config Library — Rust Full Workflow: Calibration Adjustment
//
// Source lives in rust/examples/full_workflow.rs (Cargo example).
// Run from the repo root:
//
//   cargo run --example full_workflow --manifest-path rust/Cargo.toml
//
// Scenario: a field calibration measured new scanner-head positions for both
// optical trains.  Load the current machine config, apply the updated offsets,
// write the modified config to a new file, and verify the changes persisted
// alongside the binary correction data.
//
// Steps demonstrated:
//   1. Print pre-calibration summary (machine name, train count, offsets, correction grid shapes)
//   2. Apply new scanner offsets via direct struct mutation
//   3. Write the modified config to a temporary HDF5 file
//   4. Read back and assert offset changes persisted and correction data is intact
