//! Shared, version-agnostic conversion between the two on-disk shapes this
//! concept has ever used: the flat shape (an opaque `Algorithm_Type` string
//! plus a CSV `Params` string — File_Version 1.0's `ClearBox.Volts_To_Watts_*`
//! and `LightSource.Watts_To_Volts_*`) and the structured shape
//! ([`PowerCharacterization`] — File_Version 1.1's `Power_Characterization`
//! group).
//!
//! Deliberately independent of both `capabilities::v1_0` and
//! `capabilities::v1_1` — this module only transforms StableModel types
//! ([`PowerCharacterization`], [`EquationConstant`], [`CalibrationPoint`]),
//! has no HDF5 I/O, and has no knowledge of "1.0"/"1.1" as version strings.
//! It lives at the same neutral level as `models.rs` so that either version's
//! adapter can import it without either version becoming a runtime
//! dependency of the other (see `V1_1_IMPLEMENTATION_PLAN.md`'s Phase 2
//! "Structural requirement" note) — and so a future version reusing either
//! shape needs zero new code here, only a new pair of functions if a third
//! shape is ever introduced.
//!
//! `power_characterization` is the StableModel's **only** representation of
//! this concept, for files of either version
//! (`POWER_CHARACTERIZATION_UNIFICATION_PLAN.md`) — `ClearBox`/`LightSource`
//! no longer carry the flat `volts_to_watts_*`/`watts_to_volts_*` fields at
//! all. `Hdf5AdapterV1_0`'s reader unconditionally forward-derives
//! `power_characterization` from the flat attrs it reads off disk;
//! `Hdf5WriterV1_0` unconditionally backward-derives the flat attrs from
//! `power_characterization` to write. `Hdf5AdapterV1_1`/`Hdf5WriterV1_1` read
//! and write `power_characterization` natively — no conversion needed, since
//! v1.1's on-disk shape already *is* this shape.
//!
//! (Phase 2, since superseded by the unification above, briefly had both a
//! flat field and `power_characterization` on the StableModel simultaneously,
//! with each writer preferring its own native field and falling back to
//! deriving from the other only when absent. That fallback-based design is
//! what originally motivated keeping this conversion out of the readers —
//! see `V1_1_IMPLEMENTATION_PLAN.md` Phase 2's "Design decision" section for
//! the historical comparison. The reasoning no longer applies now that there
//! is only one StableModel field: v1.0's reader calling this module
//! unconditionally is not "the readers deriving as a fallback," it is simply
//! how v1.0's on-disk shape becomes the StableModel's only shape.)
//!
//! Two shapes exist today:
//!
//! - **Coefficients** (Change 3 / ClearBox): `Params` is a positional,
//!   comma-separated coefficient list. Named to match the generated
//!   equation's own variable names: `b`/`a` for LINEAR, `c0`/`c1`/... for
//!   POLYNOMIAL, or a plain numeric fallback (`"0"`, `"1"`, ...) for any
//!   other value — never an error, see "Never raises" below.
//! - **Points** (Change 4 / Light_Source): `Params` is an alternating
//!   `input_value, output_value` list — direct, mechanical parse, no
//!   coefficient-ordering convention to assume.
//!
//! **Never raises** on an unrecognized `Algorithm_Type`. Earlier design
//! (File_Version 1.1's now-removed `migrate_v1_to_v1_1`) treated this as a
//! hard error (`MachineConfigError::UnrecognizedAlgorithmType`), which made
//! sense when this conversion only ran inside an explicit, opt-in migration
//! call. Once it runs implicitly as part of an ordinary write (this module's
//! actual job now), that would mean an unrelated value elsewhere on the model
//! could fail a write the caller never expected to fail. This is an
//! *incompleteness* problem (less information than the richer shape can
//! hold), not a *conflict* (two present, disagreeing values) — Change 1's
//! Consolidate check is the latter and correctly keeps its hard error in
//! `Hdf5WriterV1_1`, untouched by this module. `algorithm_type` is always
//! copied verbatim (open string, per `docs/migrations/v1_0_to_v1_1.md`'s
//! resolved enum-vs-string decision), coefficients/points are always parsed
//! mechanically, and only `algorithm_equation` is left blank when it can't be
//! generated — round trip stays lossless even for unrecognized types, since
//! writing back re-sorts by the numeric fallback name and reproduces the
//! original CSV order exactly. (A genuinely malformed CSV float — not an
//! unrecognized algorithm type, actual corrupt data — still returns
//! `Err(MachineConfigError::Parse)`, same as before; that failure mode is
//! orthogonal to this rule and unchanged.)
//!
//! `input_type`/`units_derived_quantity` are always left blank (`None`) on
//! forward derivation, not hardcoded — confirmed 2026-09-01: a hardcoded
//! guess is a special case that would need re-litigating for every future
//! version; blank is honest about "not derivable from what's on disk."

use crate::error::{MachineConfigError, Result};
use crate::models::{CalibrationPoint, EquationConstant, PowerCharacterization};

const RECOGNIZED_ALGORITHM_TYPES: [&str; 2] = ["LINEAR", "POLYNOMIAL"];

/// Parses a comma-separated float list. Tolerates the bracketed `"[1,2,3]"`
/// form real `Watts_To_Volts_Params` fixtures use, as well as the plain
/// `"1,2,3"` form real `Volts_To_Watts_Params` fixtures use — brackets are
/// stripped unconditionally (a no-op for the unbracketed case).
fn parse_csv_floats(s: Option<&str>) -> Result<Vec<f64>> {
    let Some(s) = s else { return Ok(Vec::new()) };
    let trimmed = s.trim().trim_matches(|c| c == '[' || c == ']');
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    trimmed
        .split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(|x| {
            x.parse::<f64>()
                .map_err(|_| MachineConfigError::Parse(format!("invalid float {x:?} in CSV params")))
        })
        .collect()
}

fn generate_power_equation(algorithm_type: &str, n_constants: usize) -> String {
    if algorithm_type == "LINEAR" {
        return "W = a*V + b".to_string();
    }
    let mut terms = vec!["c0".to_string()];
    for i in 1..n_constants {
        if i == 1 {
            terms.push("c1*V".to_string());
        } else {
            terms.push(format!("c{i}*V^{i}"));
        }
    }
    format!("W = {}", terms.join(" + "))
}

/// Named rows aren't positionally guaranteed on disk — re-sort by name before
/// joining as CSV: `b` before `a` for LINEAR, `c0 < c1 < c2 < ...` by numeric
/// suffix for POLYNOMIAL, `"0" < "1" < "2" < ...` numerically for the
/// unrecognized-algorithm-type fallback naming (see module docs' "Never
/// raises"), anything else falls back to alphabetical by name
/// (natively-authored v1.1 data can use arbitrary names, since Algorithm_Type
/// is an open string).
fn sort_key_for_constant_name(name: &str) -> (u8, i64, String) {
    if name == "b" {
        return (0, 0, String::new());
    }
    if name == "a" {
        return (0, 1, String::new());
    }
    if let Some(rest) = name.strip_prefix('c') {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = rest.parse::<i64>() {
                return (1, n, String::new());
            }
        }
    }
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = name.parse::<i64>() {
            return (2, n, String::new());
        }
    }
    (3, 0, name.to_string())
}

fn sorted_constants_by_name(constants: &[EquationConstant]) -> Vec<EquationConstant> {
    let mut sorted = constants.to_vec();
    sorted.sort_by(|a, b| sort_key_for_constant_name(&a.name).cmp(&sort_key_for_constant_name(&b.name)));
    sorted
}

/// Formats an `f64` the way Python's `str(float)` would — always with a
/// decimal point or exponent, never a bare integer — so CSV strings produced
/// here match what the Python reference implementation produces byte for
/// byte.
fn format_f64_like_python(v: f64) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".to_string() } else { "-inf".to_string() };
    }
    let s = v.to_string();
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{s}.0")
    }
}

/// Coefficients-shape forward derivation (Change 3 / ClearBox):
/// `Volts_To_Watts_Algorithm`/`Volts_To_Watts_Params` -> `PowerCharacterization`.
///
/// `None` if both inputs are `None` (nothing to derive). Never fails on an
/// unrecognized `algorithm_type` — see module docs. Can still return
/// `Err(MachineConfigError::Parse)` for a genuinely malformed CSV float,
/// an orthogonal, pre-existing failure mode.
pub fn forward_power_characterization_coefficients(
    algorithm_type: Option<&str>,
    params: Option<&str>,
) -> Result<Option<PowerCharacterization>> {
    if algorithm_type.is_none() && params.is_none() {
        return Ok(None);
    }
    let at = algorithm_type.unwrap_or("");
    let values = parse_csv_floats(params)?;
    let names: Vec<String> = if at == "LINEAR" {
        vec!["b".to_string(), "a".to_string()]
    } else if at == "POLYNOMIAL" {
        (0..values.len()).map(|i| format!("c{i}")).collect()
    } else {
        (0..values.len()).map(|i| i.to_string()).collect()
    };
    let constants = names
        .into_iter()
        .zip(values.iter().copied())
        .map(|(name, value)| EquationConstant { name, value })
        .collect();
    let equation = if RECOGNIZED_ALGORITHM_TYPES.contains(&at) {
        Some(generate_power_equation(at, values.len()))
    } else {
        None
    };
    Ok(Some(PowerCharacterization {
        algorithm_type: Some(at.to_string()),
        algorithm_equation: equation,
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: constants,
        characterization_points: Vec::new(),
    }))
}

/// Points-shape forward derivation (Change 4 / Light_Source):
/// `Watts_To_Volts_Algorithm`/`Watts_To_Volts_Params` -> `PowerCharacterization`.
///
/// Inverse data-availability from the coefficients shape — direct,
/// mechanical parse, no coefficient-ordering convention to assume. `None` if
/// both inputs are `None`. Never fails on an unrecognized `algorithm_type` —
/// see module docs.
pub fn forward_power_characterization_points(
    algorithm_type: Option<&str>,
    params: Option<&str>,
) -> Result<Option<PowerCharacterization>> {
    if algorithm_type.is_none() && params.is_none() {
        return Ok(None);
    }
    let at = algorithm_type.unwrap_or("");
    let values = parse_csv_floats(params)?;
    let mut points = Vec::new();
    let mut i = 0;
    while i + 1 < values.len() {
        points.push(CalibrationPoint { input_value: values[i], output_value: values[i + 1] });
        i += 2;
    }
    // POLYNOMIAL (and the unrecognized-type fallback): left blank, not
    // generated — point count doesn't reliably indicate polynomial degree.
    let equation = if at == "LINEAR" { Some("W = a*V + b".to_string()) } else { None };
    Ok(Some(PowerCharacterization {
        algorithm_type: Some(at.to_string()),
        algorithm_equation: equation,
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: Vec::new(),
        characterization_points: points,
    }))
}

/// Coefficients-shape backward derivation (Change 3 / ClearBox):
/// `PowerCharacterization` -> (`Algorithm_Type`, `Volts_To_Watts_Params`).
///
/// `(None, None)` if `pc` is `None`. Blank params (`Some("")`, not an error)
/// if `derivation_equation_constants` is empty — absence, not ambiguity, see
/// `docs/migrations/v1_0_to_v1_1.md` Change 3's "Backward migration" section.
pub fn backward_flat_fields_coefficients(
    pc: Option<&PowerCharacterization>,
) -> (Option<String>, Option<String>) {
    match pc {
        None => (None, None),
        Some(p) if p.derivation_equation_constants.is_empty() => {
            (p.algorithm_type.clone(), Some(String::new()))
        }
        Some(p) => {
            let ordered = sorted_constants_by_name(&p.derivation_equation_constants);
            let params = ordered
                .iter()
                .map(|c| format_f64_like_python(c.value))
                .collect::<Vec<_>>()
                .join(",");
            (p.algorithm_type.clone(), Some(params))
        }
    }
}

/// Points-shape backward derivation (Change 4 / Light_Source):
/// `PowerCharacterization` -> (`Algorithm_Type`, `Watts_To_Volts_Params`).
///
/// `(None, None)` if `pc` is `None`. No re-sort needed — `Characterization_Points`
/// rows aren't named, and on-disk order is already correct. Blank params if
/// empty, same absence-not-ambiguity reasoning as the coefficients shape.
pub fn backward_flat_fields_points(
    pc: Option<&PowerCharacterization>,
) -> (Option<String>, Option<String>) {
    match pc {
        None => (None, None),
        Some(p) if p.characterization_points.is_empty() => {
            (p.algorithm_type.clone(), Some(String::new()))
        }
        Some(p) => {
            let mut flat = Vec::with_capacity(p.characterization_points.len() * 2);
            for pt in &p.characterization_points {
                flat.push(format_f64_like_python(pt.input_value));
                flat.push(format_f64_like_python(pt.output_value));
            }
            (p.algorithm_type.clone(), Some(flat.join(",")))
        }
    }
}
