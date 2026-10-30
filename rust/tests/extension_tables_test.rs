//! Compound datasets in extension trees (contract 0.1.1): core reads and writes them verbatim as
//! tables, and a tree without tables is unaffected.

use machine_config::extension_contract::{ColumnData, Dtype, Table, TableField};
use machine_config::{
    canonical_digest, ExtensionTree, MachineConfig, MachineConfigReader, MachineConfigWriter,
};
use tempfile::NamedTempFile;

const REFERENCE_V1_1: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_v1_1.h5");

/// Same tree (and digest) as nodejs/tests/extension_tables.test.ts and extension-contract's tests.
const TABLE_SAMPLE_DIGEST: &str = "600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858";

fn points() -> Table {
    Table {
        fields: vec![TableField::numeric("input_value", Dtype::F64), TableField::numeric("output_value", Dtype::F64)],
        rows: 2,
        columns: vec![ColumnData::F64(vec![4.0, 20.0]), ColumnData::F64(vec![-1.0, 6.0])],
        attrs: Default::default(),
    }
}

fn table_tree() -> ExtensionTree {
    let mut tree = ExtensionTree::default();
    tree.attrs.insert("Schema".into(), machine_config::extension_contract::Attr::scalar("1.0"));
    tree.tables.insert(
        "Constants".into(),
        Table {
            fields: vec![TableField::utf8("name", 64), TableField::numeric("value", Dtype::F64)],
            rows: 2,
            columns: vec![ColumnData::Utf8(vec!["a".into(), "b".into()]), ColumnData::F64(vec![0.4375, -2.75])],
            attrs: [("Units".to_string(), machine_config::extension_contract::Attr::scalar("V"))]
                .into_iter()
                .collect(),
        },
    );
    tree.tables.insert(
        "Mixed".into(),
        Table {
            fields: vec![
                TableField::numeric("id", Dtype::U32),
                TableField::numeric("count", Dtype::I64),
                TableField::numeric("ratio", Dtype::F32),
                TableField::numeric("flag", Dtype::U8),
                TableField::utf8("label", 8),
            ],
            rows: 3,
            columns: vec![
                ColumnData::U32(vec![1, 2, u32::MAX]),
                ColumnData::I64(vec![-9007199254740993, 0, 7]),
                ColumnData::F32(vec![0.5, -1.5, 2.0]),
                ColumnData::U8(vec![0, 1, 1]),
                ColumnData::Utf8(vec!["x".into(), "\u{b5}m".into(), String::new()]),
            ],
            attrs: Default::default(),
        },
    );
    tree.tables.insert(
        "Empty".into(),
        Table {
            fields: vec![TableField::numeric("input_value", Dtype::F64), TableField::numeric("output_value", Dtype::F64)],
            rows: 0,
            columns: vec![ColumnData::F64(vec![]), ColumnData::F64(vec![])],
            attrs: Default::default(),
        },
    );
    let mut child = ExtensionTree::default();
    child.tables.insert("Points".into(), points());
    tree.children.insert("Train_1".into(), child);
    tree
}

fn with_extension(name: &str, tree: ExtensionTree) -> MachineConfig {
    let mut cfg = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    cfg.extensions.clear();
    cfg.extensions.insert(name.into(), tree);
    cfg
}

fn write(cfg: &MachineConfig) -> NamedTempFile {
    let tmp = NamedTempFile::new().unwrap();
    MachineConfigWriter::with_target_version(cfg, "1.1").write(tmp.path()).unwrap();
    tmp
}

fn read(path: &std::path::Path) -> MachineConfig {
    MachineConfigReader::open(path).unwrap().parse().unwrap()
}

#[test]
fn round_trips_every_table_verbatim() {
    let tmp = write(&with_extension("Private_Ext", table_tree()));
    let parsed = read(tmp.path());
    let got = &parsed.extensions["Private_Ext"];
    assert_eq!(got, &table_tree());
    assert_eq!(canonical_digest(got).unwrap(), TABLE_SAMPLE_DIGEST);
    assert_eq!(got.tables["Empty"].rows, 0);
}

#[test]
fn preserves_tables_across_read_write_read() {
    let first = write(&with_extension("Private_Ext", table_tree()));
    let second = write(&read(first.path()));
    assert_eq!(canonical_digest(&read(second.path()).extensions["Private_Ext"]).unwrap(), TABLE_SAMPLE_DIGEST);
}

#[test]
fn a_tree_without_tables_has_none_after_a_round_trip() {
    let tmp = write(&with_extension("Private_Ext", ExtensionTree::default()));
    assert!(read(tmp.path()).extensions["Private_Ext"].tables.is_empty());
}

#[test]
fn rejects_an_invalid_table_before_creating_the_file() {
    let mut bad = table_tree();
    bad.tables.get_mut("Constants").unwrap().columns[0] = ColumnData::Utf8(vec!["only-one".into()]);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.h5");
    assert!(MachineConfigWriter::with_target_version(&with_extension("Bad", bad), "1.1").write(&out).is_err());
    assert!(!out.exists());
}

/// Each language writes its own fixture (UPDATE_EXTENSION_FIXTURES=1) and reads both; the digest
/// proves Node and Rust agree on the compound layout, strings and 64-bit values.
#[test]
fn reads_the_node_and_rust_written_fixtures() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures");
    let node = format!("{fixtures}/extension_table_sample_node_v1_1.h5");
    let rust = format!("{fixtures}/extension_table_sample_rust_v1_1.h5");
    if std::env::var_os("UPDATE_EXTENSION_FIXTURES").is_some() {
        MachineConfigWriter::with_target_version(&with_extension("Private_Ext", table_tree()), "1.1")
            .write(std::path::Path::new(&rust))
            .unwrap();
    }
    for path in [node, rust] {
        let cfg = read(std::path::Path::new(&path));
        assert_eq!(
            canonical_digest(&cfg.extensions["Private_Ext"]).unwrap(),
            TABLE_SAMPLE_DIGEST,
            "{path}"
        );
    }
}

mod unsupported {
    use super::*;

    #[derive(hdf5::H5Type, Clone, Copy)]
    #[repr(C)]
    struct WithArray {
        a: [f64; 2],
    }

    #[derive(hdf5::H5Type, Clone, Copy)]
    #[repr(C)]
    struct Plain {
        a: f64,
    }

    fn file_with(add: impl FnOnce(&hdf5::Group)) -> NamedTempFile {
        let tmp = write(&with_extension("Private_Ext", ExtensionTree::default()));
        let file = hdf5::File::open_rw(tmp.path()).unwrap();
        add(&file.group("Extensions/Private_Ext").unwrap());
        drop(file);
        tmp
    }

    fn read_error(tmp: &NamedTempFile) -> String {
        MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap_err().to_string()
    }

    #[test]
    fn rejects_a_compound_dataset_that_is_not_1_d() {
        let tmp = file_with(|g| {
            g.new_dataset::<Plain>().shape((2, 2)).create("Grid").unwrap();
        });
        let message = read_error(&tmp);
        assert!(message.contains("Private_Ext/Grid") && message.contains("1-D"), "{message}");
    }

    #[test]
    fn rejects_an_array_member() {
        let tmp = file_with(|g| {
            g.new_dataset::<WithArray>().shape(1).create("Arr").unwrap();
        });
        let message = read_error(&tmp);
        assert!(message.contains("Private_Ext/Arr/a"), "{message}");
    }
}
