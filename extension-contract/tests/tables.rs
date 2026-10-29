use mcf_extension_contract::*;

// Pinned; the Node implementation asserts the same value (nodejs/tests/extension_tables.test.ts).
const TABLE_SAMPLE_DIGEST: &str = "600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858";
// A tree without tables keeps its contract 0.1.0 digest.
const LEGACY_SAMPLE_DIGEST: &str = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890";

fn points() -> Table {
    Table {
        fields: vec![TableField::numeric("input_value", Dtype::F64), TableField::numeric("output_value", Dtype::F64)],
        rows: 2,
        columns: vec![ColumnData::F64(vec![4.0, 20.0]), ColumnData::F64(vec![-1.0, 6.0])],
        attrs: Default::default(),
    }
}

/// Same tree as `tableTree()` in the Node test.
pub fn table_tree() -> ExtensionTree {
    let mut tree = ExtensionTree::default();
    tree.attrs.insert("Schema".into(), Attr::scalar("1.0"));
    tree.tables.insert(
        "Constants".into(),
        Table {
            fields: vec![TableField::utf8("name", 64), TableField::numeric("value", Dtype::F64)],
            rows: 2,
            columns: vec![
                ColumnData::Utf8(vec!["a".into(), "b".into()]),
                ColumnData::F64(vec![0.4375, -2.75]),
            ],
            attrs: [("Units".to_string(), Attr::scalar("V"))].into_iter().collect(),
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

#[test]
fn stays_on_the_0_1_contract_line() {
    assert!(EXTENSION_CONTRACT_VERSION.starts_with("0.1."));
}

#[test]
fn digest_matches_the_node_implementation() {
    assert_eq!(canonical_digest(&table_tree()).unwrap(), TABLE_SAMPLE_DIGEST);
}

#[test]
fn an_empty_tables_map_does_not_change_the_digest() {
    let mut tree = ExtensionTree::default();
    tree.attrs.insert("A".into(), Attr::scalar(1u32));
    let plain = canonical_digest(&tree).unwrap();
    tree.tables.clear();
    assert_eq!(canonical_digest(&tree).unwrap(), plain);
}

#[test]
fn golden_json_fixtures_keep_their_digests() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures");
    let read = |name: &str| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join(name)).unwrap()).unwrap()
    };
    let legacy = tree_from_json(&read("extension_sample_tree.json")).unwrap();
    assert_eq!(canonical_digest(&legacy).unwrap(), LEGACY_SAMPLE_DIGEST);
    let tables = tree_from_json(&read("extension_table_sample_tree.json")).unwrap();
    assert_eq!(tables, table_tree());
    assert_eq!(canonical_digest(&tables).unwrap(), TABLE_SAMPLE_DIGEST);
}

#[test]
fn digest_is_sensitive_to_field_order_size_values_and_rows_but_not_table_order() {
    let digest = |t: &ExtensionTree| canonical_digest(t).unwrap();
    let base = digest(&table_tree());

    let mut swapped = table_tree();
    let t = swapped.tables.get_mut("Constants").unwrap();
    t.fields.reverse();
    t.columns.reverse();
    assert_ne!(digest(&swapped), base);

    let mut wider = table_tree();
    wider.tables.get_mut("Constants").unwrap().fields[0].size = Some(32);
    assert_ne!(digest(&wider), base);

    let mut edited = table_tree();
    edited.tables.get_mut("Constants").unwrap().columns[1] = ColumnData::F64(vec![0.4375, -2.5]);
    assert_ne!(digest(&edited), base);

    let mut shorter = table_tree();
    let p = shorter.children.get_mut("Train_1").unwrap().tables.get_mut("Points").unwrap();
    p.rows = 1;
    p.columns = vec![ColumnData::F64(vec![4.0]), ColumnData::F64(vec![-1.0])];
    assert_ne!(digest(&shorter), base);
}

#[test]
fn json_round_trips_and_omits_tables_when_absent() {
    let json = tree_to_json(&table_tree());
    assert_eq!(json["tables"]["Mixed"]["columns"]["count"], serde_json::json!(["-9007199254740993", "0", "7"]));
    assert_eq!(json["tables"]["Constants"]["fields"][0], serde_json::json!({"name": "name", "dtype": "utf8", "size": 64}));
    assert_eq!(json["tables"]["Constants"]["fields"][1], serde_json::json!({"name": "value", "dtype": "f64"}));
    assert_eq!(tree_from_json(&json).unwrap(), table_tree());

    assert!(tree_to_json(&ExtensionTree::default()).get("tables").is_none());
}

fn invalid(mutate: impl FnOnce(&mut Table)) -> ContractError {
    let mut tree = ExtensionTree::default();
    let mut t = points();
    mutate(&mut t);
    tree.tables.insert("T".into(), t);
    validate_tree(&tree, "Ext").unwrap_err()
}

#[test]
fn validation_rejects_malformed_tables_with_a_path() {
    assert!(invalid(|t| t.fields.clear()).message.contains("at least one field"));
    assert!(invalid(|t| t.fields[1].name = "input_value".into()).message.contains("duplicate field"));
    assert!(invalid(|t| t.rows = 3).message.contains("does not match 3 rows"));
    assert!(invalid(|t| t.columns[0] = ColumnData::F32(vec![0.0; 2])).message.contains("field dtype is f64"));
    assert!(invalid(|t| t.fields[0].size = Some(8)).message.contains("only valid for utf8"));
    assert!(invalid(|t| {
        t.fields[0] = TableField { name: "input_value".into(), dtype: Dtype::Utf8, size: None };
        t.columns[0] = ColumnData::Utf8(vec!["a".into(), "b".into()]);
    })
    .message
    .contains("positive integer size"));
    assert!(invalid(|t| {
        t.fields[0] = TableField::utf8("input_value", 2);
        t.columns[0] = ColumnData::Utf8(vec!["abc".into(), "b".into()]);
    })
    .message
    .contains("do not fit the 2-byte field"));
    assert_eq!(invalid(|t| t.columns[1] = ColumnData::F64(vec![0.0])).path, "Ext/T/output_value");

    let mut clash = ExtensionTree::default();
    clash.children.insert("T".into(), ExtensionTree::default());
    clash.tables.insert("T".into(), points());
    assert!(validate_tree(&clash, "Ext").unwrap_err().message.contains("cannot share a name"));
}
