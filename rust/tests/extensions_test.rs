//! Extension trees: core reads and writes them verbatim, preserving dtypes, so unknown or private
//! extensions survive a read-modify-write.

use machine_config::extension_contract::{Attr, AttrValue, Dataset, DatasetData, Dtype, Scalar};
use machine_config::{
    canonical_digest, ExtensionTree, MachineConfig, MachineConfigError, MachineConfigReader,
    MachineConfigWriter,
};
use tempfile::NamedTempFile;

const REFERENCE_V1_1: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_v1_1.h5");

/// Same tree (and digest) as nodejs/tests/extensions.test.ts and extension-contract's tests.
const SAMPLE_DIGEST: &str = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890";

fn sample_tree() -> ExtensionTree {
    let mut tree = ExtensionTree::default();
    tree.attrs.insert("Schema".into(), Attr::scalar("1.2"));
    tree.attrs.insert("Flag".into(), Attr::scalar(1u32));
    tree.attrs.insert("Float_Flag".into(), Attr::scalar(1.0f64));
    tree.attrs.insert("Small".into(), Attr::scalar(-5i8));
    tree.attrs.insert("Wide".into(), Attr::scalar(-9007199254740993i64));
    tree.attrs.insert("Huge".into(), Attr::scalar(u64::MAX));
    tree.attrs.insert("Ratio".into(), Attr::scalar(0.5f32));
    tree.attrs
        .insert("Pair".into(), Attr::array(Dtype::F64, vec![1.5f64.into(), (-2.5f64).into()]));
    tree.attrs
        .insert("Ids".into(), Attr::array(Dtype::U32, vec![1u32.into(), 2u32.into(), 3u32.into()]));

    let mut child = ExtensionTree::default();
    child.attrs.insert("Label".into(), Attr::scalar("Job Start"));
    let mut grandchild = ExtensionTree::default();
    grandchild.attrs.insert("Value".into(), Attr::scalar("running"));
    child.children.insert("Condition 1".into(), grandchild);
    tree.children.insert("Job Start".into(), child);

    tree.datasets.insert(
        "Grid".into(),
        Dataset {
            shape: vec![2, 3],
            data: DatasetData::F64(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.5]),
            attrs: [("Units".to_string(), Attr::scalar("mm"))].into_iter().collect(),
        },
    );
    tree
}

fn base() -> MachineConfig {
    let mut cfg = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    cfg.extensions.clear();
    cfg
}

fn with_extension(name: &str, tree: ExtensionTree) -> MachineConfig {
    let mut cfg = base();
    cfg.extensions.insert(name.into(), tree);
    cfg
}

fn write(cfg: &MachineConfig, version: &str) -> NamedTempFile {
    let tmp = NamedTempFile::new().unwrap();
    MachineConfigWriter::with_target_version(cfg, version).write(tmp.path()).unwrap();
    tmp
}

fn read(path: &std::path::Path) -> MachineConfig {
    MachineConfigReader::open(path).unwrap().parse().unwrap()
}

#[test]
fn round_trips_an_unknown_extension_verbatim() {
    let tmp = write(&with_extension("Private_Ext", sample_tree()), "1.1");
    let parsed = read(tmp.path());

    assert_eq!(parsed.extensions.keys().collect::<Vec<_>>(), ["Private_Ext"]);
    let got = &parsed.extensions["Private_Ext"];
    assert_eq!(canonical_digest(got).unwrap(), SAMPLE_DIGEST);
    assert_eq!(got.attrs["Wide"], Attr::scalar(-9007199254740993i64));
    assert_eq!(got.attrs["Flag"], Attr::scalar(1u32));
    assert_eq!(got.attrs["Float_Flag"], Attr::scalar(1.0f64));
    assert_eq!(got.datasets["Grid"].shape, vec![2, 3]);
    assert_eq!(got.datasets["Grid"].data, DatasetData::F64(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.5]));
}

#[test]
fn preserves_an_unknown_extension_across_read_write_read() {
    let first = write(&with_extension("Private_Ext", sample_tree()), "1.1");
    let parsed = read(first.path());
    let second = write(&parsed, "1.1");
    let again = read(second.path());
    assert_eq!(canonical_digest(&again.extensions["Private_Ext"]).unwrap(), SAMPLE_DIGEST);
}

#[test]
fn files_without_extensions_have_none() {
    assert!(base().extensions.is_empty());
}

#[test]
fn reads_the_tm_opcua_group_of_a_v1_1_file_like_any_other_extension() {
    let cfg = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    assert_eq!(cfg.extensions.keys().collect::<Vec<_>>(), ["TM_OPCUA"]);
}

#[test]
fn rejects_clearbox_as_a_tree() {
    let cfg = with_extension("ClearBox", ExtensionTree::default());
    let tmp = NamedTempFile::new().unwrap();
    let err = MachineConfigWriter::with_target_version(&cfg, "1.1").write(tmp.path()).unwrap_err();
    assert!(matches!(err, MachineConfigError::InvalidExtension(ref m) if m.contains("modelled by core")));
}

#[test]
fn rejects_an_invalid_tree_before_creating_the_file() {
    let mut bad = ExtensionTree::default();
    bad.attrs.insert(
        "X".into(),
        Attr { dtype: Dtype::U8, value: AttrValue::Scalar(Scalar::U32(1)) },
    );
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.h5");
    let err = MachineConfigWriter::with_target_version(&with_extension("Bad", bad), "1.1")
        .write(&out)
        .unwrap_err();
    assert!(matches!(err, MachineConfigError::InvalidExtension(_)));
    assert!(!out.exists());
}

#[test]
fn tm_opcua_tree_lives_at_extensions_tm_opcua_in_1_1_and_reads_back_untyped() {
    let mut tree = ExtensionTree::default();
    tree.attrs.insert("Opcua_Schema_Version".into(), Attr::scalar("1.2"));
    let mut client = ExtensionTree::default();
    client.attrs.insert("Auth_Mode".into(), Attr::scalar("Anonymous"));
    tree.children.insert("Client".into(), client);

    let tmp = write(&with_extension("TM_OPCUA", tree.clone()), "1.1");
    let f = hdf5::File::open(tmp.path()).unwrap();
    assert!(f.group("Extensions/TM_OPCUA/Client").is_ok());

    let parsed = read(tmp.path());
    assert_eq!(parsed.extensions["TM_OPCUA"], tree);
}

#[test]
fn tm_opcua_tree_lives_at_opcua_in_1_0_and_is_exposed_again_as_an_extension() {
    let mut tree = ExtensionTree::default();
    tree.attrs.insert("Opcua_Schema_Version".into(), Attr::scalar("1.2"));
    let tmp = write(&with_extension("TM_OPCUA", tree.clone()), "1.0");
    assert!(hdf5::File::open(tmp.path()).unwrap().group("OPCUA").is_ok());

    let parsed = read(tmp.path());
    assert_eq!(parsed.extensions["TM_OPCUA"], tree);
}

#[test]
fn version_1_0_rejects_other_extension_names() {
    let cfg = with_extension("Private_Ext", ExtensionTree::default());
    let tmp = NamedTempFile::new().unwrap();
    let err = MachineConfigWriter::with_target_version(&cfg, "1.0").write(tmp.path()).unwrap_err();
    assert!(matches!(err, MachineConfigError::InvalidExtension(ref m) if m.contains("no Extensions group")));
}

#[test]
fn hash_is_unchanged_without_extensions_and_changes_with_their_content() {
    let plain = base();
    let hash = machine_config::hash::compute_configuration_hash;
    assert_eq!(hash(&plain), hash(&base()));

    let a = with_extension("Private_Ext", sample_tree());
    let mut changed_tree = sample_tree();
    changed_tree.attrs.insert("Flag".into(), Attr::scalar(0u32));
    let b = with_extension("Private_Ext", changed_tree);
    assert_ne!(hash(&a), hash(&plain));
    assert_ne!(hash(&a), hash(&b));
}

/// Each language writes its own fixture (UPDATE_EXTENSION_FIXTURES=1) and reads both; the digest
/// proves Rust and Node agree on dtypes, values, structure and the canonical encoding.
#[test]
fn reads_the_node_and_rust_written_fixtures() {
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures");
    let node = format!("{fixtures}/extension_sample_node_v1_1.h5");
    let rust = format!("{fixtures}/extension_sample_rust_v1_1.h5");
    if std::env::var_os("UPDATE_EXTENSION_FIXTURES").is_some() {
        let mut small = machine_config::MockConfigBuilder::new(1).build();
        small.extensions.insert("Private_Ext".into(), sample_tree());
        MachineConfigWriter::with_target_version(&small, "1.1").write(&rust).unwrap();
    }
    for path in [node, rust] {
        let parsed = read(std::path::Path::new(&path));
        assert_eq!(
            canonical_digest(&parsed.extensions["Private_Ext"]).unwrap(),
            SAMPLE_DIGEST,
            "{path}"
        );
    }
}

#[test]
fn preserves_an_unknown_extension_through_an_open_save_session() {
    let first = write(&with_extension("Private_Ext", sample_tree()), "1.1");
    let mut session = machine_config::MachineConfigFileV1_1::open(first.path()).unwrap();

    let second = NamedTempFile::new().unwrap();
    session.save(Some(second.path())).unwrap();
    assert!(session.is_valid(), "hash must cover extension content and match after save");

    let again = read(second.path());
    assert_eq!(canonical_digest(&again.extensions["Private_Ext"]).unwrap(), SAMPLE_DIGEST);
    assert_eq!(again.meta.is_valid, Some(true));
}
