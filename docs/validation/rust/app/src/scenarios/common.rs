/// Bitwise (NaN-aware) equality for correction-grid data — mirrors Python's
/// `np.array_equal(a, b, equal_nan=True)`. Plain `==` on `&[f64]` would treat
/// any NaN as unequal to itself, which is wrong for correction grids where
/// NaN marks an intentional out-of-field cell.
pub fn bitwise_equal(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// The shared, language-agnostic AV fixtures live at
/// `docs/validation/fixtures/<name>` relative to the repo root, one level up
/// from `fixtures_dir` — same convention already used by Python's and Node's
/// AV scenario files.
pub fn av_fixture(fixtures_dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    fixtures_dir
        .parent()
        .expect("fixtures_dir has a parent")
        .join("docs")
        .join("validation")
        .join("fixtures")
        .join(name)
}
