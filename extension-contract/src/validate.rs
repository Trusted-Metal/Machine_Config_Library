use std::collections::BTreeMap;

use crate::tree::{Attr, AttrValue, Dataset, ExtensionTree, Scalar};

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
    Ok(())
}
