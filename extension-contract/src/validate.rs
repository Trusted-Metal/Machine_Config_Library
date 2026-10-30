use std::collections::BTreeMap;

use crate::tree::{Attr, AttrValue, ColumnData, Dataset, Dtype, ExtensionTree, Scalar, Table};

/// `path` is the slash-joined location of the offending node; attributes are prefixed with `@`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {message}", if path.is_empty() { "<root>" } else { path.as_str() })]
pub struct ContractError {
    pub path: String,
    pub message: String,
}

impl ContractError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        ContractError { path: path.into(), message: message.into() }
    }
}

pub(crate) fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}/{name}")
    }
}

/// Names become HDF5 link names: non-empty, no `/`, no NUL.
pub fn validate_node_name(name: &str, path: &str) -> Result<(), ContractError> {
    if name.is_empty() {
        return Err(ContractError::new(path, "name must not be empty"));
    }
    if name.contains('/') {
        return Err(ContractError::new(path, format!("name \"{name}\" must not contain \"/\"")));
    }
    if name.contains('\0') {
        return Err(ContractError::new(path, "name must not contain NUL"));
    }
    Ok(())
}

fn validate_scalar(attr: &Attr, v: &Scalar, path: &str) -> Result<(), ContractError> {
    if v.dtype() != attr.dtype {
        return Err(ContractError::new(
            path,
            format!("value is {} but the attribute dtype is {}", v.dtype().name(), attr.dtype.name()),
        ));
    }
    if let Scalar::Utf8(s) = v {
        if s.contains('\0') {
            return Err(ContractError::new(path, "utf8 value must not contain NUL"));
        }
    }
    Ok(())
}

fn validate_attrs(attrs: &BTreeMap<String, Attr>, path: &str) -> Result<(), ContractError> {
    for (name, attr) in attrs {
        let attr_path = join(path, &format!("@{name}"));
        validate_node_name(name, &attr_path)?;
        match &attr.value {
            AttrValue::Scalar(v) => validate_scalar(attr, v, &attr_path)?,
            AttrValue::Array(values) => {
                for (i, v) in values.iter().enumerate() {
                    validate_scalar(attr, v, &format!("{attr_path}[{i}]"))?;
                }
            }
        }
    }
    Ok(())
}

fn validate_dataset(ds: &Dataset, path: &str) -> Result<(), ContractError> {
    let expected: usize = ds.shape.iter().product();
    if ds.data.len() != expected {
        return Err(ContractError::new(
            path,
            format!(
                "data length {} does not match shape [{}]",
                ds.data.len(),
                ds.shape.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
            ),
        ));
    }
    validate_attrs(&ds.attrs, path)
}

/// Validates `tree` rooted at `path` (use `""` for an anonymous root, or the extension name).
pub fn validate_tree(tree: &ExtensionTree, path: &str) -> Result<(), ContractError> {
    validate_attrs(&tree.attrs, path)?;
    for (name, child) in &tree.children {
        let child_path = join(path, name);
        validate_node_name(name, &child_path)?;
        validate_tree(child, &child_path)?;
    }
    for (name, ds) in &tree.datasets {
        let ds_path = join(path, name);
        validate_node_name(name, &ds_path)?;
        if tree.children.contains_key(name) {
            return Err(ContractError::new(ds_path, "a dataset and a child group cannot share a name"));
        }
        validate_dataset(ds, &ds_path)?;
    }
    for (name, table) in &tree.tables {
        let table_path = join(path, name);
        validate_node_name(name, &table_path)?;
        if tree.children.contains_key(name) || tree.datasets.contains_key(name) {
            return Err(ContractError::new(
                table_path,
                "a table cannot share a name with a dataset or child group",
            ));
        }
        validate_table(table, &table_path)?;
    }
    Ok(())
}

fn validate_table(table: &Table, path: &str) -> Result<(), ContractError> {
    if table.fields.is_empty() {
        return Err(ContractError::new(path, "a table needs at least one field"));
    }
    if table.columns.len() != table.fields.len() {
        return Err(ContractError::new(
            path,
            format!("{} columns for {} fields", table.columns.len(), table.fields.len()),
        ));
    }
    for (i, field) in table.fields.iter().enumerate() {
        let field_path = join(path, &field.name);
        validate_node_name(&field.name, &field_path)?;
        if table.fields[..i].iter().any(|f| f.name == field.name) {
            return Err(ContractError::new(field_path, "duplicate field name"));
        }
        let column = &table.columns[i];
        if column.dtype() != field.dtype {
            return Err(ContractError::new(
                field_path,
                format!("column is {} but the field dtype is {}", column.dtype().name(), field.dtype.name()),
            ));
        }
        if column.len() != table.rows {
            return Err(ContractError::new(
                field_path,
                format!("column length {} does not match {} rows", column.len(), table.rows),
            ));
        }
        match (field.dtype, field.size, column) {
            (Dtype::Utf8, Some(size), ColumnData::Utf8(values)) if size >= 1 => {
                for (row, v) in values.iter().enumerate() {
                    let row_path = format!("{field_path}[{row}]");
                    if v.contains('\0') {
                        return Err(ContractError::new(row_path, "utf8 value must not contain NUL"));
                    }
                    if v.len() > size {
                        return Err(ContractError::new(
                            row_path,
                            format!("{} UTF-8 bytes do not fit the {size}-byte field", v.len()),
                        ));
                    }
                }
            }
            (Dtype::Utf8, _, _) => {
                return Err(ContractError::new(field_path, "a utf8 field needs a positive integer size"));
            }
            (_, Some(_), _) => {
                return Err(ContractError::new(field_path, "size is only valid for utf8 fields"));
            }
            _ => {}
        }
    }
    validate_attrs(&table.attrs, path)
}
