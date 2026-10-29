//! File_Version 1.0 keeps ClearBox per optical train under `Optional_Components/ClearBox/`; core
//! exposes it as the opaque `ClearBox` extension tree, always in the File_Version 1.1 layout, so an
//! extension library never has to know which file version a tree came from:
//!
//! ```text
//! ClearBox/                      Output_Path, Software_Trigger_Delay   (one shared value each)
//!   <train_id>/                  per-train attributes, correction datasets, Synchronous_Sensors/
//!     Power_Characterization/    structured replacement for Volts_To_Watts_Algorithm/Params
//! ```
//!
//! Everything else in a v1.0 ClearBox group is carried verbatim, including the attributes v1.1
//! dropped (`Selected_Camera`, `Show_Console`, ...), so a v1.0 file round-trips without losing them.
//! The two shared values are taken from the first train that has a ClearBox.

use std::collections::BTreeMap;

use mcf_extension_contract::{Attr, ColumnData, Dtype, ExtensionTree, Table, TableField};

use crate::error::{MachineConfigError, Result};
use crate::models::{CalibrationPoint, EquationConstant, PowerCharacterization};
use crate::power_characterization::{
    backward_flat_fields_coefficients, forward_power_characterization_coefficients,
};

pub(crate) const EXTENSION_NAME: &str = "ClearBox";

const POWER_CHARACTERIZATION: &str = "Power_Characterization";
const SHARED_ATTRS: [&str; 2] = ["Output_Path", "Software_Trigger_Delay"];
const FLAT_ALGORITHM: &str = "Volts_To_Watts_Algorithm";
const FLAT_PARAMS: &str = "Volts_To_Watts_Params";
const V1_1_ONLY_ATTRS: [&str; 1] = ["Firmware_Version"];
const EQUATION_CONSTANT_NAME_BYTES: usize = 64;

fn text(tree: &ExtensionTree, name: &str) -> Option<String> {
    let s = tree.attrs.get(name)?.as_str()?.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn str_attr(value: Option<&str>) -> Attr {
    Attr::scalar(value.unwrap_or(""))
}

fn power_characterization_to_tree(pc: Option<&PowerCharacterization>) -> ExtensionTree {
    let mut tree = ExtensionTree::default();
    for (name, value) in [
        ("Algorithm_Type", pc.and_then(|p| p.algorithm_type.as_deref())),
        ("Algorithm_Equation", pc.and_then(|p| p.algorithm_equation.as_deref())),
        ("Input_Type", pc.and_then(|p| p.input_type.as_deref())),
        ("Units_Derived_Quantity", pc.and_then(|p| p.units_derived_quantity.as_deref())),
    ] {
        tree.attrs.insert(name.into(), str_attr(value));
    }
    let constants: &[EquationConstant] = pc.map_or(&[], |p| &p.derivation_equation_constants);
    tree.tables.insert(
        "Derivation_Equation_Constants".into(),
        Table {
            fields: vec![
                TableField::utf8("name", EQUATION_CONSTANT_NAME_BYTES),
                TableField::numeric("value", Dtype::F64),
            ],
            rows: constants.len(),
            columns: vec![
                ColumnData::Utf8(constants.iter().map(|c| c.name.clone()).collect()),
                ColumnData::F64(constants.iter().map(|c| c.value).collect()),
            ],
            attrs: BTreeMap::new(),
        },
    );
    let points: &[CalibrationPoint] = pc.map_or(&[], |p| &p.characterization_points);
    tree.tables.insert(
        "Characterization_Points".into(),
        Table {
            fields: vec![
                TableField::numeric("input_value", Dtype::F64),
                TableField::numeric("output_value", Dtype::F64),
            ],
            rows: points.len(),
            columns: vec![
                ColumnData::F64(points.iter().map(|p| p.input_value).collect()),
                ColumnData::F64(points.iter().map(|p| p.output_value).collect()),
            ],
            attrs: BTreeMap::new(),
        },
    );
    tree
}

fn column<'a>(table: &'a Table, name: &str) -> Option<&'a ColumnData> {
    let i = table.fields.iter().position(|f| f.name == name)?;
    table.columns.get(i)
}

fn power_characterization_from_tree(tree: &ExtensionTree) -> Option<PowerCharacterization> {
    let mut constants = Vec::new();
    if let Some(t) = tree.tables.get("Derivation_Equation_Constants") {
        if let (Some(ColumnData::Utf8(names)), Some(ColumnData::F64(values))) =
            (column(t, "name"), column(t, "value"))
        {
            constants = names
                .iter()
                .zip(values)
                .map(|(name, value)| EquationConstant { name: name.clone(), value: *value })
                .collect();
        }
    }
    let mut points = Vec::new();
    if let Some(t) = tree.tables.get("Characterization_Points") {
        if let (Some(ColumnData::F64(inputs)), Some(ColumnData::F64(outputs))) =
            (column(t, "input_value"), column(t, "output_value"))
        {
            points = inputs
                .iter()
                .zip(outputs)
                .map(|(i, o)| CalibrationPoint { input_value: *i, output_value: *o })
                .collect();
        }
    }
    let pc = PowerCharacterization {
        algorithm_type: text(tree, "Algorithm_Type"),
        algorithm_equation: text(tree, "Algorithm_Equation"),
        input_type: text(tree, "Input_Type"),
        units_derived_quantity: text(tree, "Units_Derived_Quantity"),
        derivation_equation_constants: constants,
        characterization_points: points,
    };
    Some(pc)
}

/// Builds the `ClearBox` tree from each train's verbatim v1.0 group, in train order.
/// `None` when no train has a ClearBox.
pub(crate) fn from_v1_0_trains(trains: Vec<(String, ExtensionTree)>) -> Result<Option<ExtensionTree>> {
    let Some((_, first)) = trains.first() else { return Ok(None) };
    let mut root = ExtensionTree::default();
    for name in SHARED_ATTRS {
        if let Some(a) = first.attrs.get(name) {
            root.attrs.insert(name.into(), a.clone());
        }
    }
    for (train_id, mut tree) in trains {
        for name in SHARED_ATTRS {
            tree.attrs.remove(name);
        }
        let algorithm = text(&tree, FLAT_ALGORITHM);
        let params = text(&tree, FLAT_PARAMS);
        tree.attrs.remove(FLAT_ALGORITHM);
        tree.attrs.remove(FLAT_PARAMS);
        let pc = forward_power_characterization_coefficients(algorithm.as_deref(), params.as_deref())?;
        if tree.children.contains_key(POWER_CHARACTERIZATION) {
            return Err(MachineConfigError::Parse(format!(
                "{train_id}: a File_Version 1.0 ClearBox group cannot contain {POWER_CHARACTERIZATION}"
            )));
        }
        tree.children.insert(POWER_CHARACTERIZATION.into(), power_characterization_to_tree(pc.as_ref()));
        root.children.insert(train_id, tree);
    }
    Ok(Some(root))
}

/// Splits the `ClearBox` tree back into one verbatim v1.0 group per train id.
///
/// Fails rather than dropping anything v1.0 has no place for: attributes or datasets directly on
/// the extension root other than the two shared values.
pub(crate) fn to_v1_0_trains(tree: &ExtensionTree) -> Result<BTreeMap<String, ExtensionTree>> {
    let lossy = |what: String| {
        MachineConfigError::InvalidExtension(format!(
            "ClearBox: {what} has no File_Version 1.0 location"
        ))
    };
    if let Some(name) = tree.attrs.keys().find(|k| !SHARED_ATTRS.contains(&k.as_str())) {
        return Err(lossy(format!("attribute \"{name}\"")));
    }
    if let Some(name) = tree.datasets.keys().chain(tree.tables.keys()).next() {
        return Err(lossy(format!("dataset \"{name}\"")));
    }
    let mut out = BTreeMap::new();
    for (train_id, child) in &tree.children {
        let mut group = child.clone();
        for name in SHARED_ATTRS {
            if !group.attrs.contains_key(name) {
                if let Some(a) = tree.attrs.get(name) {
                    group.attrs.insert(name.into(), a.clone());
                }
            }
        }
        for name in V1_1_ONLY_ATTRS {
            group.attrs.remove(name);
        }
        let pc = group.children.remove(POWER_CHARACTERIZATION).as_ref().and_then(power_characterization_from_tree);
        let (algorithm, params) = backward_flat_fields_coefficients(pc.as_ref());
        group.attrs.insert(FLAT_ALGORITHM.into(), str_attr(algorithm.as_deref()));
        group.attrs.insert(FLAT_PARAMS.into(), str_attr(params.as_deref()));
        out.insert(train_id.clone(), group);
    }
    Ok(out)
}
