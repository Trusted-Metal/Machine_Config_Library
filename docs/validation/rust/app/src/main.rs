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
        ("S-01", scenarios::run_s01),
        ("S-02", scenarios::run_s02),
        ("S-03", scenarios::run_s03),
        ("S-04", scenarios::run_s04),
        ("S-05", scenarios::run_s05),
        ("S-06", scenarios::run_s06),
        ("S-07", scenarios::run_s07),
        ("S-08", scenarios::run_s08),
        ("S-09", scenarios::run_s09),
        ("AV-01", scenarios::run_av01),
        ("AV-02", scenarios::run_av02),
        ("AV-03", scenarios::run_av03),
        ("AV-04", scenarios::run_av04),
        ("AV-05", scenarios::run_av05),
        ("AV-06", scenarios::run_av06),
        ("AV-07", scenarios::run_av07),
        ("AV-08", scenarios::run_av08),
        ("AV-12", scenarios::run_av12),
        ("AV-13", scenarios::run_av13),
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
