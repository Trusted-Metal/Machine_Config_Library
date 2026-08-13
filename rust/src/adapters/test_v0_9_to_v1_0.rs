// AUTO-GENERATED from schema/adapters/test_v0_9_to_v1_0.yaml
// DO NOT EDIT MANUALLY — regenerate with: python tools/generate_adapters.py
use serde_json::{Map, Value};
use crate::adapters::Adapter;

pub const FROM_VERSION: &str = "0.9";
pub const TO_VERSION:   &str = "1.0";

pub struct V0_9ToV1_0;

impl Adapter for V0_9ToV1_0 {
    fn from_version(&self) -> &'static str { FROM_VERSION }
    fn to_version(&self) -> &'static str { TO_VERSION }

    fn adapt(&self, mut config: Map<String, Value>) -> Map<String, Value> {
        if let Some(Value::Array(trains)) = config.get_mut("optical_trains") {
            for train in trains.iter_mut() {
                if let Value::Object(ref mut t) = train {
                    // field_add: test_added_field (path: optical_trains[*])
                    t.entry("test_added_field".to_owned()).or_insert(Value::Null);
                    // field_rename: test_old_name -> test_new_name (path: optical_trains[*])
                    if let Some(val) = t.remove("test_old_name") {
                        t.insert("test_new_name".to_owned(), val);
                    }
                    // field_remove: test_removed_field dropped (path: optical_trains[*])
                    t.remove("test_removed_field");
                    // field_move: test_move_field (optical_trains[*] -> optical_trains[*].test_nested)
                    if let Some(val) = t.remove("test_move_field") {
                        let dest = t.entry("test_nested".to_owned())
                            .or_insert_with(|| Value::Object(Map::new()));
                        if let Value::Object(ref mut d) = dest {
                            d.insert("test_move_field".to_owned(), val);
                        }
                    }
                    // field_move_rename: test_move_rename_old -> test_nested.test_move_rename_new (optical_trains[*] -> optical_trains[*].test_nested)
                    if let Some(val) = t.remove("test_move_rename_old") {
                        let dest = t.entry("test_nested".to_owned())
                            .or_insert_with(|| Value::Object(Map::new()));
                        if let Value::Object(ref mut d) = dest {
                            d.insert("test_move_rename_new".to_owned(), val);
                        }
                    }
                }
            }
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Map, Value};
    use super::{V0_9ToV1_0, FROM_VERSION, TO_VERSION};
    use crate::adapters::{Adapter, registry};

    fn make_config(train: Value) -> Map<String, Value> {
        match json!({ "optical_trains": [train] }) {
            Value::Object(m) => m,
            _ => unreachable!(),
        }
    }

    fn first_train(mut config: Map<String, Value>) -> Map<String, Value> {
        match config.remove("optical_trains").unwrap() {
            Value::Array(mut arr) => match arr.remove(0) {
                Value::Object(t) => t,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_field_add() {
        let cfg = make_config(json!({}));
        let result = first_train(V0_9ToV1_0.adapt(cfg));
        assert_eq!(result["test_added_field"], Value::Null);
    }

    #[test]
    fn test_field_rename() {
        let cfg = make_config(json!({ "test_old_name": "val" }));
        let result = first_train(V0_9ToV1_0.adapt(cfg));
        assert_eq!(result["test_new_name"], "val");
        assert!(!result.contains_key("test_old_name"));
    }

    #[test]
    fn test_field_remove() {
        let cfg = make_config(json!({ "test_removed_field": "val" }));
        let result = first_train(V0_9ToV1_0.adapt(cfg));
        assert!(!result.contains_key("test_removed_field"));
    }

    #[test]
    fn test_field_move() {
        let cfg = make_config(json!({ "test_move_field": "val" }));
        let result = first_train(V0_9ToV1_0.adapt(cfg));
        assert!(!result.contains_key("test_move_field"));
        assert_eq!(result["test_nested"]["test_move_field"], "val");
    }

    #[test]
    fn test_field_move_rename() {
        let cfg = make_config(json!({ "test_move_rename_old": "val" }));
        let result = first_train(V0_9ToV1_0.adapt(cfg));
        assert!(!result.contains_key("test_move_rename_old"));
        assert_eq!(result["test_nested"]["test_move_rename_new"], "val");
    }

    #[test]
    fn test_version_constants() {
        assert_eq!(FROM_VERSION, "0.9");
        assert_eq!(TO_VERSION, "1.0");
    }

    #[test]
    fn test_registry_chain() {
        assert_eq!(registry::get_chain("0.9", "1.0").len(), 1);
        assert_eq!(registry::get_chain("1.0", "1.0").len(), 0);
    }
}
