mod scenarios;

use std::path::Path;
use std::process::ExitCode;

type RunFn = fn(&Path, &Path) -> (bool, String);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: mcl_rust_validation <fixtures_dir> <real_dir>");
        return ExitCode::FAILURE;
    }
    let fixtures_dir = Path::new(&args[1]);
    let real_dir = Path::new(&args[2]);

    let scenario_list: Vec<(&str, RunFn)> = vec![
        ("S-01", scenarios::s01_read_scalars::run),
        ("S-02", scenarios::s02_read_binary::run),
        ("S-03", scenarios::s03_read_real::run),
        ("S-04", scenarios::s04_write_modify::run),
        ("S-05", scenarios::s05_binary_roundtrip::run),
        ("S-06", scenarios::s06_builder::run),
        ("S-07", scenarios::s07_opcua::run),
        ("S-08", scenarios::s08_drastic_change::run),
        ("S-09", scenarios::s09_type_exports::run),
        ("AV-01", scenarios::av01_unknown_version::run),
        ("AV-02", scenarios::av02_missing_version::run),
        ("AV-03", scenarios::av03_future_version::run),
        ("AV-04", scenarios::av04_missing_group::run),
        ("AV-05", scenarios::av05_corrupt_scalar::run),
        ("AV-06", scenarios::av06_whitespace_version::run),
        ("AV-07", scenarios::av07_empty_version::run),
        ("AV-08", scenarios::av08_version_fidelity::run),
    ];

    let mut passed = 0;
    let mut failed = 0;
    for (id, run) in scenario_list {
        let (ok, detail) = run(fixtures_dir, real_dir);
        if ok {
            println!("[PASS] {id}: {detail}");
            passed += 1;
        } else {
            println!("[FAIL] {id}: {detail}");
            failed += 1;
        }
    }

    println!("\n{} scenarios: {passed} passed, {failed} failed", passed + failed);
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
