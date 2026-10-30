//! Extension contract: the opaque, language-neutral tree core uses to carry data that belongs to
//! an extension (a group under `Extensions/<name>/`), so a separate library can pack and unpack it
//! without core knowing its schema.
//!
//! Dependency-light on purpose (no HDF5): extension libraries depend on this crate only. Core reads
//! and writes every tree verbatim, so unknown or private extensions round-trip untouched.
//!
//! Pre-stable (0.x) until a second extension has been ported against it. The Node implementation
//! lives in `nodejs/src/extensions/`; the canonical digest and JSON form are byte-for-byte
//! identical in both.

mod digest;
mod json;
mod tree;
mod validate;

pub use digest::canonical_digest;
pub use json::{tree_from_json, tree_to_json};
pub use tree::{
    Attr, AttrValue, ColumnData, Dataset, DatasetData, Dtype, ExtensionTree, Scalar, Table, TableField,
    EXTENSION_CONTRACT_VERSION,
};
pub use validate::{validate_node_name, validate_tree, ContractError};
