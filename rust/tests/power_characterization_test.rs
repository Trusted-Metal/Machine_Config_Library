//! Unit tests for machine_config::power_characterization — the shared,
//! version-agnostic shape-conversion functions Phase 2
//! (V1_1_IMPLEMENTATION_PLAN.md) introduced to replace
//! migrate_v1_to_v1_1/migrate_v1_1_to_v1. Pure functions, no HDF5 I/O —
//! integration with the real writers is covered separately in
//! v1_1_adapter_test.rs. Mirrors Python's test_power_characterization.py.

use machine_config::power_characterization::{
    backward_flat_fields_coefficients, backward_flat_fields_points,
    forward_power_characterization_coefficients, forward_power_characterization_points,
};
use machine_config::{CalibrationPoint, EquationConstant, PowerCharacterization};

fn names(constants: &[EquationConstant]) -> Vec<&str> {
    constants.iter().map(|c| c.name.as_str()).collect()
}

// ---------------------------------------------------------------------------
// forward_power_characterization_coefficients (Change 3 / ClearBox shape)
// ---------------------------------------------------------------------------

#[test]
fn forward_coefficients_both_none_returns_none() {
    assert!(forward_power_characterization_coefficients(None, None).unwrap().is_none());
}

#[test]
fn forward_coefficients_linear() {
    let pc = forward_power_characterization_coefficients(Some("LINEAR"), Some("50.0,100.0"))
        .unwrap()
        .unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("LINEAR"));
    assert_eq!(pc.algorithm_equation.as_deref(), Some("W = a*V + b"));
    assert_eq!(names(&pc.derivation_equation_constants), vec!["b", "a"]);
    let values: Vec<f64> = pc.derivation_equation_constants.iter().map(|c| c.value).collect();
    assert_eq!(values, vec![50.0, 100.0]);
    assert!(pc.characterization_points.is_empty());
}

#[test]
fn forward_coefficients_polynomial() {
    let pc = forward_power_characterization_coefficients(Some("POLYNOMIAL"), Some("1.0,2.0,3.0"))
        .unwrap()
        .unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("POLYNOMIAL"));
    assert_eq!(pc.algorithm_equation.as_deref(), Some("W = c0 + c1*V + c2*V^2"));
    assert_eq!(names(&pc.derivation_equation_constants), vec!["c0", "c1", "c2"]);
}

#[test]
fn forward_coefficients_blank_input_type_and_units() {
    // Confirmed 2026-09-01: blank, not hardcoded — see module docs.
    let pc = forward_power_characterization_coefficients(Some("LINEAR"), Some("50.0,100.0"))
        .unwrap()
        .unwrap();
    assert!(pc.input_type.is_none());
    assert!(pc.units_derived_quantity.is_none());
}

#[test]
fn forward_coefficients_unrecognized_algorithm_never_raises() {
    // Never a hard error — see module docs' "Never raises" section.
    let pc = forward_power_characterization_coefficients(Some("EXPONENTIAL"), Some("1.5,2.5,3.5"))
        .unwrap()
        .unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("EXPONENTIAL"));
    assert!(pc.algorithm_equation.is_none());
    assert_eq!(names(&pc.derivation_equation_constants), vec!["0", "1", "2"]);
    let values: Vec<f64> = pc.derivation_equation_constants.iter().map(|c| c.value).collect();
    assert_eq!(values, vec![1.5, 2.5, 3.5]);
}

// ---------------------------------------------------------------------------
// forward_power_characterization_points (Change 4 / Light_Source shape)
// ---------------------------------------------------------------------------

#[test]
fn forward_points_both_none_returns_none() {
    assert!(forward_power_characterization_points(None, None).unwrap().is_none());
}

#[test]
fn forward_points_linear() {
    let pc = forward_power_characterization_points(Some("LINEAR"), Some("1,100,10,1000"))
        .unwrap()
        .unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("LINEAR"));
    assert_eq!(pc.algorithm_equation.as_deref(), Some("W = a*V + b"));
    let points: Vec<(f64, f64)> =
        pc.characterization_points.iter().map(|p| (p.input_value, p.output_value)).collect();
    assert_eq!(points, vec![(1.0, 100.0), (10.0, 1000.0)]);
    assert!(pc.derivation_equation_constants.is_empty());
}

#[test]
fn forward_points_polynomial_leaves_equation_blank() {
    // Point count doesn't reliably indicate polynomial degree — never
    // generated for POLYNOMIAL, unlike the coefficients shape.
    let pc = forward_power_characterization_points(Some("POLYNOMIAL"), Some("1,100,10,1000"))
        .unwrap()
        .unwrap();
    assert!(pc.algorithm_equation.is_none());
}

#[test]
fn forward_points_unrecognized_algorithm_never_raises() {
    let pc = forward_power_characterization_points(Some("QUADRATIC"), Some("1,100,10,1000"))
        .unwrap()
        .unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("QUADRATIC"));
    assert!(pc.algorithm_equation.is_none());
    assert_eq!(pc.characterization_points.len(), 2);
}

#[test]
fn forward_points_strips_brackets() {
    // Real Watts_To_Volts_Params fixtures wrap the CSV in brackets, unlike
    // Volts_To_Watts_Params's plain form.
    let pc = forward_power_characterization_points(Some("LINEAR"), Some("[1,100,10,1000]"))
        .unwrap()
        .unwrap();
    let points: Vec<(f64, f64)> =
        pc.characterization_points.iter().map(|p| (p.input_value, p.output_value)).collect();
    assert_eq!(points, vec![(1.0, 100.0), (10.0, 1000.0)]);
}

#[test]
fn forward_points_blank_input_type_and_units() {
    let pc = forward_power_characterization_points(Some("LINEAR"), Some("1,100")).unwrap().unwrap();
    assert!(pc.input_type.is_none());
    assert!(pc.units_derived_quantity.is_none());
}

// ---------------------------------------------------------------------------
// backward_flat_fields_coefficients (Change 3 backward)
// ---------------------------------------------------------------------------

#[test]
fn backward_coefficients_none_pc_returns_none_none() {
    assert_eq!(backward_flat_fields_coefficients(None), (None, None));
}

#[test]
fn backward_coefficients_empty_constants_writes_blank_not_error() {
    let pc = PowerCharacterization {
        algorithm_type: Some("LINEAR".to_string()),
        algorithm_equation: Some("W = a*V + b".to_string()),
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: vec![],
        characterization_points: vec![],
    };
    assert_eq!(
        backward_flat_fields_coefficients(Some(&pc)),
        (Some("LINEAR".to_string()), Some(String::new()))
    );
}

#[test]
fn backward_coefficients_reorders_by_name() {
    // Rows aren't positionally guaranteed on disk — must re-sort before
    // joining as CSV: b before a for LINEAR.
    let pc = PowerCharacterization {
        algorithm_type: Some("LINEAR".to_string()),
        algorithm_equation: None,
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: vec![
            EquationConstant { name: "a".to_string(), value: 2.0 },
            EquationConstant { name: "b".to_string(), value: 1.0 },
        ],
        characterization_points: vec![],
    };
    let (algorithm, params) = backward_flat_fields_coefficients(Some(&pc));
    assert_eq!(algorithm.as_deref(), Some("LINEAR"));
    assert_eq!(params.as_deref(), Some("1.0,2.0"));
}

#[test]
fn backward_coefficients_reorders_polynomial_by_numeric_suffix() {
    let pc = PowerCharacterization {
        algorithm_type: Some("POLYNOMIAL".to_string()),
        algorithm_equation: None,
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: vec![
            EquationConstant { name: "c2".to_string(), value: 3.0 },
            EquationConstant { name: "c0".to_string(), value: 1.0 },
            EquationConstant { name: "c1".to_string(), value: 2.0 },
        ],
        characterization_points: vec![],
    };
    let (_, params) = backward_flat_fields_coefficients(Some(&pc));
    assert_eq!(params.as_deref(), Some("1.0,2.0,3.0"));
}

#[test]
fn backward_coefficients_reorders_positional_fallback_numerically() {
    // Unrecognized-algorithm-type fallback names ('0','1',...) must sort
    // numerically, not lexically (else '10' would sort before '2').
    let pc = PowerCharacterization {
        algorithm_type: Some("EXPONENTIAL".to_string()),
        algorithm_equation: None,
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: vec![
            EquationConstant { name: "1".to_string(), value: 2.5 },
            EquationConstant { name: "0".to_string(), value: 1.5 },
            EquationConstant { name: "2".to_string(), value: 3.5 },
        ],
        characterization_points: vec![],
    };
    let (algorithm, params) = backward_flat_fields_coefficients(Some(&pc));
    assert_eq!(algorithm.as_deref(), Some("EXPONENTIAL"));
    assert_eq!(params.as_deref(), Some("1.5,2.5,3.5"));
}

#[test]
fn forward_then_backward_coefficients_round_trips_unrecognized_type() {
    let (original_algorithm, original_params) = ("EXPONENTIAL", "1.5,2.5,3.5");
    let pc =
        forward_power_characterization_coefficients(Some(original_algorithm), Some(original_params))
            .unwrap();
    let (algorithm, params) = backward_flat_fields_coefficients(pc.as_ref());
    assert_eq!(algorithm.as_deref(), Some(original_algorithm));
    assert_eq!(params.as_deref(), Some(original_params));
}

// ---------------------------------------------------------------------------
// backward_flat_fields_points (Change 4 backward)
// ---------------------------------------------------------------------------

#[test]
fn backward_points_none_pc_returns_none_none() {
    assert_eq!(backward_flat_fields_points(None), (None, None));
}

#[test]
fn backward_points_empty_writes_blank_not_error() {
    let pc = PowerCharacterization {
        algorithm_type: Some("LINEAR".to_string()),
        algorithm_equation: Some("W = a*V + b".to_string()),
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: vec![],
        characterization_points: vec![],
    };
    assert_eq!(
        backward_flat_fields_points(Some(&pc)),
        (Some("LINEAR".to_string()), Some(String::new()))
    );
}

#[test]
fn backward_points_no_resort_needed() {
    // Characterization_Points rows aren't named — on-disk order is already
    // correct, unlike the coefficients shape.
    let pc = PowerCharacterization {
        algorithm_type: Some("LINEAR".to_string()),
        algorithm_equation: None,
        input_type: None,
        units_derived_quantity: None,
        derivation_equation_constants: vec![],
        characterization_points: vec![
            CalibrationPoint { input_value: 1.0, output_value: 100.0 },
            CalibrationPoint { input_value: 10.0, output_value: 1000.0 },
        ],
    };
    let (algorithm, params) = backward_flat_fields_points(Some(&pc));
    assert_eq!(algorithm.as_deref(), Some("LINEAR"));
    assert_eq!(params.as_deref(), Some("1.0,100.0,10.0,1000.0"));
}

#[test]
fn forward_then_backward_points_round_trips() {
    let (original_algorithm, original_params) = ("LINEAR", "1.0,100.0,10.0,1000.0");
    let pc = forward_power_characterization_points(Some(original_algorithm), Some(original_params))
        .unwrap();
    let (algorithm, params) = backward_flat_fields_points(pc.as_ref());
    assert_eq!(algorithm.as_deref(), Some(original_algorithm));
    assert_eq!(params.as_deref(), Some(original_params));
}
