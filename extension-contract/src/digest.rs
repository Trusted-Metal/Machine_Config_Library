use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::tree::{Attr, AttrValue, DatasetData, ExtensionTree, Scalar};
use crate::validate::{validate_tree, ContractError};

// Canonical encoding, identical in every language (all little-endian):
//
//   tree    = u32 attrCount, attr*, u32 childCount, (name tree)*, u32 dsCount, (name dataset)*
//   name    = u32 byteLength, UTF-8 bytes
//   attr    = name, u8 dtypeTag, u8 kind (0 scalar, 1 array), [u32 count], value*
//   dataset = u8 dtypeTag, u32 rank, u64 dim*, u64 count, value*, attr map (as in tree)
//   value   = utf8: u32 byteLength + bytes; ints: fixed width; f32/f64: IEEE bits
//
// Entries are sorted by the UTF-8 bytes of their names (BTreeMap<String, _> order). NaN is
// encoded as the quiet NaN and -0 as +0, so equal values always hash equal.

fn put_u32(h: &mut Sha256, n: u32) {
    h.update(n.to_le_bytes());
}

fn put_u64(h: &mut Sha256, n: u64) {
    h.update(n.to_le_bytes());
}

fn put_name(h: &mut Sha256, s: &str) {
    put_u32(h, s.len() as u32);
    h.update(s.as_bytes());
}

fn put_scalar(h: &mut Sha256, v: &Scalar) {
    match v {
        Scalar::Utf8(s) => put_name(h, s),
        Scalar::I8(x) => h.update(x.to_le_bytes()),
        Scalar::U8(x) => h.update(x.to_le_bytes()),
        Scalar::I16(x) => h.update(x.to_le_bytes()),
        Scalar::U16(x) => h.update(x.to_le_bytes()),
        Scalar::I32(x) => h.update(x.to_le_bytes()),
        Scalar::U32(x) => h.update(x.to_le_bytes()),
        Scalar::I64(x) => h.update(x.to_le_bytes()),
        Scalar::U64(x) => h.update(x.to_le_bytes()),
        Scalar::F32(x) => {
            let bits = if x.is_nan() { 0x7fc0_0000 } else if *x == 0.0 { 0 } else { x.to_bits() };
            put_u32(h, bits);
        }
        Scalar::F64(x) => {
            let bits = if x.is_nan() { 0x7ff8_0000_0000_0000 } else if *x == 0.0 { 0 } else { x.to_bits() };
            put_u64(h, bits);
        }
    }
}

fn put_attrs(h: &mut Sha256, attrs: &BTreeMap<String, Attr>) {
    put_u32(h, attrs.len() as u32);
    for (name, attr) in attrs {
        put_name(h, name);
        h.update([attr.dtype.tag()]);
        match &attr.value {
            AttrValue::Scalar(v) => {
                h.update([0u8]);
                put_scalar(h, v);
            }
            AttrValue::Array(values) => {
                h.update([1u8]);
                put_u32(h, values.len() as u32);
                for v in values {
                    put_scalar(h, v);
                }
            }
        }
    }
}

fn put_data(h: &mut Sha256, data: &DatasetData) {
    macro_rules! each {
        ($values:expr, $variant:ident) => {
            for x in $values {
                put_scalar(h, &Scalar::$variant(*x));
            }
        };
    }
    match data {
        DatasetData::I8(v) => each!(v, I8),
        DatasetData::U8(v) => each!(v, U8),
        DatasetData::I16(v) => each!(v, I16),
        DatasetData::U16(v) => each!(v, U16),
        DatasetData::I32(v) => each!(v, I32),
        DatasetData::U32(v) => each!(v, U32),
        DatasetData::I64(v) => each!(v, I64),
        DatasetData::U64(v) => each!(v, U64),
        DatasetData::F32(v) => each!(v, F32),
        DatasetData::F64(v) => each!(v, F64),
    }
}

fn put_tree(h: &mut Sha256, tree: &ExtensionTree) {
    put_attrs(h, &tree.attrs);
    put_u32(h, tree.children.len() as u32);
    for (name, child) in &tree.children {
        put_name(h, name);
        put_tree(h, child);
    }
    put_u32(h, tree.datasets.len() as u32);
    for (name, ds) in &tree.datasets {
        put_name(h, name);
        h.update([ds.dtype().tag()]);
        put_u32(h, ds.shape.len() as u32);
        for d in &ds.shape {
            put_u64(h, *d as u64);
        }
        put_u64(h, ds.data.len() as u64);
        put_data(h, &ds.data);
        put_attrs(h, &ds.attrs);
    }
}

/// Hex SHA-256 of the canonical encoding. Errors if the tree violates the contract.
pub fn canonical_digest(tree: &ExtensionTree) -> Result<String, ContractError> {
    validate_tree(tree, "")?;
    let mut h = Sha256::new();
    put_tree(&mut h, tree);
    Ok(format!("{:x}", h.finalize()))
}
