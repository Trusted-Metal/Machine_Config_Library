//! Read and write [`ExtensionTree`]s as HDF5 groups.
//!
//! Version-independent: the adapters decide *where* an extension lives (`Extensions/<name>/` in
//! 1.1, the legacy `OPCUA/` group in 1.0); this module only maps the contract's tree to and from
//! the HDF5 group at that location. Everything is read and written verbatim, with dtypes
//! preserved, so extensions core has no model for survive a read-modify-write.

use std::collections::BTreeMap;

use hdf5::types::{FloatSize, IntSize, TypeDescriptor, VarLenAscii, VarLenUnicode};
use hdf5::{Extents, Group, Location};
use mcf_extension_contract::{
    validate_tree, Attr, AttrValue, Dataset, DatasetData, Dtype, ExtensionTree, Scalar,
};

use crate::error::{MachineConfigError, Result};

mod table;

/// Test helper: drops the `data` array of every extension dataset in a serialized
/// `MachineConfig`, keeping `dtype`, `shape` and attributes. The golden file does this so it stays
/// small; the grids themselves are covered by the bit-exact round-trip and `correction-hash`
/// checks.
#[cfg(test)]
pub(crate) fn strip_dataset_data(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Object(datasets)) = map.get_mut("datasets") {
                for ds in datasets.values_mut() {
                    if let Some(obj) = ds.as_object_mut() {
                        obj.remove("data");
                    }
                }
            }
            for v in map.values_mut() {
                strip_dataset_data(v);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(strip_dataset_data),
        _ => {}
    }
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}/{name}")
    }
}

fn unsupported(path: &str, what: impl std::fmt::Display) -> MachineConfigError {
    MachineConfigError::Parse(format!("extension node '{path}': {what}"))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Checks every tree against the contract, naming the extension in any error.
pub fn validate_extension_trees(extensions: &BTreeMap<String, ExtensionTree>) -> Result<()> {
    for (name, tree) in extensions {
        validate_tree(tree, name).map_err(|e| MachineConfigError::InvalidExtension(e.to_string()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

fn read_attr(attr: &hdf5::Attribute, path: &str) -> Result<Attr> {
    let shape = attr.shape();
    if shape.len() > 1 {
        return Err(unsupported(path, format!("attributes must be scalar or 1-D, got rank {}", shape.len())));
    }
    let scalar = shape.is_empty();
    let td = attr.dtype()?.to_descriptor()?;

    macro_rules! numeric {
        ($t:ty, $variant:ident, $dtype:expr) => {{
            if scalar {
                Attr { dtype: $dtype, value: AttrValue::Scalar(Scalar::$variant(attr.read_scalar::<$t>()?)) }
            } else {
                Attr::array($dtype, attr.read_raw::<$t>()?.into_iter().map(Scalar::$variant).collect())
            }
        }};
    }
    macro_rules! string {
        ($t:ty) => {{
            if scalar {
                Attr::scalar(attr.read_scalar::<$t>()?.as_str())
            } else {
                Attr::array(
                    Dtype::Utf8,
                    attr.read_raw::<$t>()?.iter().map(|s| Scalar::Utf8(s.as_str().to_string())).collect(),
                )
            }
        }};
    }

    Ok(match td {
        TypeDescriptor::VarLenUnicode => string!(VarLenUnicode),
        TypeDescriptor::VarLenAscii => string!(VarLenAscii),
        TypeDescriptor::Integer(IntSize::U1) => numeric!(i8, I8, Dtype::I8),
        TypeDescriptor::Integer(IntSize::U2) => numeric!(i16, I16, Dtype::I16),
        TypeDescriptor::Integer(IntSize::U4) => numeric!(i32, I32, Dtype::I32),
        TypeDescriptor::Integer(IntSize::U8) => numeric!(i64, I64, Dtype::I64),
        TypeDescriptor::Unsigned(IntSize::U1) => numeric!(u8, U8, Dtype::U8),
        TypeDescriptor::Unsigned(IntSize::U2) => numeric!(u16, U16, Dtype::U16),
        TypeDescriptor::Unsigned(IntSize::U4) => numeric!(u32, U32, Dtype::U32),
        TypeDescriptor::Unsigned(IntSize::U8) => numeric!(u64, U64, Dtype::U64),
        TypeDescriptor::Float(FloatSize::U4) => numeric!(f32, F32, Dtype::F32),
        TypeDescriptor::Float(FloatSize::U8) => numeric!(f64, F64, Dtype::F64),
        other => return Err(unsupported(path, format!("unsupported HDF5 attribute type {other}"))),
    })
}

fn read_attrs(loc: &Location, path: &str) -> Result<BTreeMap<String, Attr>> {
    let mut out = BTreeMap::new();
    for name in loc.attr_names()? {
        let attr = loc.attr(&name)?;
        out.insert(name.clone(), read_attr(&attr, &join(path, &format!("@{name}")))?);
    }
    Ok(out)
}

fn read_dataset(ds: &hdf5::Dataset, path: &str) -> Result<Dataset> {
    macro_rules! data {
        ($t:ty, $variant:ident) => {
            DatasetData::$variant(ds.read_raw::<$t>()?)
        };
    }
    let data = match ds.dtype()?.to_descriptor()? {
        TypeDescriptor::Integer(IntSize::U1) => data!(i8, I8),
        TypeDescriptor::Integer(IntSize::U2) => data!(i16, I16),
        TypeDescriptor::Integer(IntSize::U4) => data!(i32, I32),
        TypeDescriptor::Integer(IntSize::U8) => data!(i64, I64),
        TypeDescriptor::Unsigned(IntSize::U1) => data!(u8, U8),
        TypeDescriptor::Unsigned(IntSize::U2) => data!(u16, U16),
        TypeDescriptor::Unsigned(IntSize::U4) => data!(u32, U32),
        TypeDescriptor::Unsigned(IntSize::U8) => data!(u64, U64),
        TypeDescriptor::Float(FloatSize::U4) => data!(f32, F32),
        TypeDescriptor::Float(FloatSize::U8) => data!(f64, F64),
        other => return Err(unsupported(path, format!("unsupported HDF5 dataset type {other}"))),
    };
    Ok(Dataset { shape: ds.shape(), data, attrs: read_attrs(ds, path)? })
}

/// Reads `group` and everything below it verbatim. Unsupported HDF5 types fail with their path.
pub fn read_extension_tree(group: &Group, path: &str) -> Result<ExtensionTree> {
    let mut tree = ExtensionTree { attrs: read_attrs(group, path)?, ..Default::default() };
    for name in group.member_names()? {
        let child_path = join(path, &name);
        if let Ok(child) = group.group(&name) {
            tree.children.insert(name, read_extension_tree(&child, &child_path)?);
        } else if let Ok(ds) = group.dataset(&name) {
            match ds.dtype()?.to_descriptor()? {
                TypeDescriptor::Compound(compound) => {
                    tree.tables.insert(name, table::read_table(&ds, &compound, &child_path)?);
                }
                _ => {
                    tree.datasets.insert(name, read_dataset(&ds, &child_path)?);
                }
            }
        } else {
            return Err(unsupported(&child_path, "only groups and datasets are supported"));
        }
    }
    Ok(tree)
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

fn write_attrs(loc: &Location, attrs: &BTreeMap<String, Attr>) -> Result<()> {
    macro_rules! numeric {
        ($name:expr, $attr:expr, $t:ty, $variant:ident) => {{
            match &$attr.value {
                AttrValue::Scalar(Scalar::$variant(v)) => {
                    loc.new_attr::<$t>().create($name.as_str())?.write_scalar(v)?;
                }
                AttrValue::Array(items) => {
                    let values: Vec<$t> = items
                        .iter()
                        .map(|s| match s {
                            Scalar::$variant(v) => *v,
                            _ => unreachable!("validated: array elements match the attribute dtype"),
                        })
                        .collect();
                    loc.new_attr::<$t>().shape(values.len()).create($name.as_str())?.write_raw(&values)?;
                }
                _ => unreachable!("validated: scalar matches the attribute dtype"),
            }
        }};
    }
    let to_vlu = |s: &str| -> Result<VarLenUnicode> {
        s.parse().map_err(|e| MachineConfigError::InvalidExtension(format!("utf8 value: {e:?}")))
    };

    for (name, attr) in attrs {
        match attr.dtype {
            Dtype::Utf8 => match &attr.value {
                AttrValue::Scalar(Scalar::Utf8(s)) => {
                    loc.new_attr::<VarLenUnicode>().create(name.as_str())?.write_scalar(&to_vlu(s)?)?;
                }
                AttrValue::Array(items) => {
                    let values = items
                        .iter()
                        .map(|s| match s {
                            Scalar::Utf8(s) => to_vlu(s),
                            _ => unreachable!("validated: array elements match the attribute dtype"),
                        })
                        .collect::<Result<Vec<_>>>()?;
                    loc.new_attr::<VarLenUnicode>()
                        .shape(values.len())
                        .create(name.as_str())?
                        .write_raw(&values)?;
                }
                _ => unreachable!("validated: scalar matches the attribute dtype"),
            },
            Dtype::I8 => numeric!(name, attr, i8, I8),
            Dtype::U8 => numeric!(name, attr, u8, U8),
            Dtype::I16 => numeric!(name, attr, i16, I16),
            Dtype::U16 => numeric!(name, attr, u16, U16),
            Dtype::I32 => numeric!(name, attr, i32, I32),
            Dtype::U32 => numeric!(name, attr, u32, U32),
            Dtype::I64 => numeric!(name, attr, i64, I64),
            Dtype::U64 => numeric!(name, attr, u64, U64),
            Dtype::F32 => numeric!(name, attr, f32, F32),
            Dtype::F64 => numeric!(name, attr, f64, F64),
        }
    }
    Ok(())
}

fn write_dataset(parent: &Group, name: &str, ds: &Dataset) -> Result<()> {
    macro_rules! create {
        ($t:ty, $values:expr) => {{
            let extents =
                if ds.shape.is_empty() { Extents::Scalar } else { Extents::from(ds.shape.clone()) };
            let dataset = parent.new_dataset::<$t>().shape(extents).create(name)?;
            if !$values.is_empty() {
                dataset.write_raw($values)?;
            }
            dataset
        }};
    }
    let dataset = match &ds.data {
        DatasetData::I8(v) => create!(i8, v),
        DatasetData::U8(v) => create!(u8, v),
        DatasetData::I16(v) => create!(i16, v),
        DatasetData::U16(v) => create!(u16, v),
        DatasetData::I32(v) => create!(i32, v),
        DatasetData::U32(v) => create!(u32, v),
        DatasetData::I64(v) => create!(i64, v),
        DatasetData::U64(v) => create!(u64, v),
        DatasetData::F32(v) => create!(f32, v),
        DatasetData::F64(v) => create!(f64, v),
    };
    write_attrs(&dataset, &ds.attrs)
}

fn write_group(group: &Group, tree: &ExtensionTree) -> Result<()> {
    write_attrs(group, &tree.attrs)?;
    for (name, ds) in &tree.datasets {
        write_dataset(group, name, ds)?;
    }
    for (name, t) in &tree.tables {
        table::write_table(group, name, t)?;
    }
    for (name, child) in &tree.children {
        write_group(&group.create_group(name)?, child)?;
    }
    Ok(())
}

/// Writes `tree` as a new group `name` under `parent`; the caller guarantees `name` is unused.
pub fn write_extension_tree(parent: &Group, name: &str, tree: &ExtensionTree) -> Result<()> {
    validate_tree(tree, name).map_err(|e| MachineConfigError::InvalidExtension(e.to_string()))?;
    write_group(&parent.create_group(name)?, tree)
}
