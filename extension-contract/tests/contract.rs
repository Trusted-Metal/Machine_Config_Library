use mcf_extension_contract::*;
use serde_json::json;

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

#[test]
fn digest_matches_the_node_implementation() {
    // Same tree and expected value as nodejs/tests/extensions.test.ts.
    assert_eq!(
        canonical_digest(&sample_tree()).unwrap(),
        "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890"
    );
}

#[test]
fn digest_distinguishes_dtype_value_and_structure() {
    let mk = |a: Attr| {
        let mut t = ExtensionTree::default();
        t.attrs.insert("A".into(), a);
        canonical_digest(&t).unwrap()
    };
    assert_ne!(mk(Attr::scalar(1u32)), mk(Attr::scalar(1.0f64)));
    assert_ne!(mk(Attr::scalar(1u32)), mk(Attr::scalar(2u32)));

    let mut with_child = ExtensionTree::default();
    with_child.children.insert("A".into(), ExtensionTree::default());
    let mut with_attr = ExtensionTree::default();
    with_attr.attrs.insert("A".into(), Attr::scalar(0u32));
    assert_ne!(canonical_digest(&with_child).unwrap(), canonical_digest(&with_attr).unwrap());
}

#[test]
fn digest_treats_negative_zero_as_zero_and_all_nan_as_equal() {
    let mk = |v: f64| {
        let mut t = ExtensionTree::default();
        t.attrs.insert("A".into(), Attr::scalar(v));
        canonical_digest(&t).unwrap()
    };
    assert_eq!(mk(-0.0), mk(0.0));
    assert_eq!(mk(f64::NAN), mk(-f64::NAN));
}

#[test]
fn validation_rejects_bad_names_and_shapes_with_a_path() {
    for name in ["", "a/b", "a\0b"] {
        let mut t = ExtensionTree::default();
        t.children.insert(name.into(), ExtensionTree::default());
        assert!(validate_tree(&t, "Ext").is_err(), "{name:?}");
    }

    let mut t = ExtensionTree::default();
    t.datasets.insert(
        "D".into(),
        Dataset { shape: vec![2, 2], data: DatasetData::F64(vec![0.0; 3]), attrs: Default::default() },
    );
    assert!(validate_tree(&t, "").unwrap_err().message.contains("does not match shape"));

    let mut inner = ExtensionTree::default();
    inner.attrs.insert(
        "Bad".into(),
        Attr { dtype: Dtype::U8, value: AttrValue::Scalar(Scalar::U32(1)) },
    );
    let mut t = ExtensionTree::default();
    t.children.insert("Rule".into(), inner);
    assert_eq!(validate_tree(&t, "Ext").unwrap_err().path, "Ext/Rule/@Bad");

    let mut t = ExtensionTree::default();
    t.children.insert("X".into(), ExtensionTree::default());
    t.datasets.insert(
        "X".into(),
        Dataset { shape: vec![], data: DatasetData::U8(vec![1]), attrs: Default::default() },
    );
    assert!(validate_tree(&t, "").unwrap_err().message.contains("cannot share a name"));
}

#[test]
fn json_round_trips_losslessly() {
    let mut tree = sample_tree();
    tree.attrs.insert(
        "Weird".into(),
        Attr::array(Dtype::F64, vec![f64::NAN.into(), f64::INFINITY.into(), f64::NEG_INFINITY.into()]),
    );
    let json = tree_to_json(&tree);
    assert_eq!(json["attrs"]["Wide"], json!({ "dtype": "i64", "value": "-9007199254740993" }));
    let back = tree_from_json(&json).unwrap();
    assert_eq!(canonical_digest(&back).unwrap(), canonical_digest(&tree).unwrap());
}

#[test]
fn json_rejects_out_of_range_and_unknown_dtypes() {
    let bad = json!({ "attrs": { "X": { "dtype": "u8", "value": 256 } }, "children": {}, "datasets": {} });
    assert!(tree_from_json(&bad).unwrap_err().message.contains("out of range"));
    let bad = json!({ "attrs": { "X": { "dtype": "u128", "value": 1 } } });
    assert!(tree_from_json(&bad).unwrap_err().message.contains("unknown dtype"));
}

#[test]
fn serde_uses_the_json_form() {
    let tree = sample_tree();
    let text = serde_json::to_string(&tree).unwrap();
    let back: ExtensionTree = serde_json::from_str(&text).unwrap();
    assert_eq!(canonical_digest(&back).unwrap(), canonical_digest(&tree).unwrap());
}

#[test]
fn contract_version_is_pre_stable() {
    assert!(EXTENSION_CONTRACT_VERSION.starts_with("0."));
}

#[test]
fn golden_json_fixture_has_the_pinned_digest() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/extension_sample_tree.json");
    let tree = tree_from_json(&serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()).unwrap();
    assert_eq!(
        canonical_digest(&tree).unwrap(),
        "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890"
    );
}
