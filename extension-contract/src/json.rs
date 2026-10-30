use std::collections::BTreeMap;

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Number, Value};

use crate::tree::{
    Attr, AttrValue, ColumnData, Dataset, DatasetData, Dtype, ExtensionTree, Scalar, Table, TableField,
};
use crate::validate::{join, validate_tree, ContractError};

// Plain-JSON form of a tree, used by fixtures and `to_json()`. i64/u64 become decimal strings and
// non-finite floats become "NaN" / "Infinity" / "-Infinity", so the JSON is lossless in every
// language. Identical to the Node `extensionTreeToJson` output.

fn float_to_json(v: f64) -> Value {
    if v.is_nan() {
        Value::String("NaN".into())
    } else if v == f64::INFINITY {
        Value::String("Infinity".into())
    } else if v == f64::NEG_INFINITY {
        Value::String("-Infinity".into())
    } else {
        Value::Number(Number::from_f64(v).expect("finite"))
    }
}

fn scalar_to_json(v: &Scalar) -> Value {
    match v {
        Scalar::Utf8(s) => Value::String(s.clone()),
        Scalar::I8(x) => json!(x),
        Scalar::U8(x) => json!(x),
        Scalar::I16(x) => json!(x),
        Scalar::U16(x) => json!(x),
        Scalar::I32(x) => json!(x),
        Scalar::U32(x) => json!(x),
        Scalar::I64(x) => Value::String(x.to_string()),
        Scalar::U64(x) => Value::String(x.to_string()),
        Scalar::F32(x) => float_to_json(f64::from(*x)),
        Scalar::F64(x) => float_to_json(*x),
    }
}

fn attrs_to_json(attrs: &BTreeMap<String, Attr>) -> Value {
    let mut out = Map::new();
    for (name, attr) in attrs {
        let value = match &attr.value {
            AttrValue::Scalar(v) => scalar_to_json(v),
            AttrValue::Array(values) => Value::Array(values.iter().map(scalar_to_json).collect()),
        };
        out.insert(name.clone(), json!({ "dtype": attr.dtype.name(), "value": value }));
    }
    Value::Object(out)
}

fn data_to_json(data: &DatasetData) -> Value {
    macro_rules! each {
        ($values:expr, $variant:ident) => {
            $values.iter().map(|x| scalar_to_json(&Scalar::$variant(*x))).collect()
        };
    }
    Value::Array(match data {
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
    })
}

fn column_to_json(column: &ColumnData) -> Value {
    macro_rules! each {
        ($values:expr, $variant:ident) => {
            $values.iter().map(|x| scalar_to_json(&Scalar::$variant(*x))).collect()
        };
    }
    Value::Array(match column {
        ColumnData::Utf8(v) => v.iter().map(|s| Value::String(s.clone())).collect(),
        ColumnData::I8(v) => each!(v, I8),
        ColumnData::U8(v) => each!(v, U8),
        ColumnData::I16(v) => each!(v, I16),
        ColumnData::U16(v) => each!(v, U16),
        ColumnData::I32(v) => each!(v, I32),
        ColumnData::U32(v) => each!(v, U32),
        ColumnData::I64(v) => each!(v, I64),
        ColumnData::U64(v) => each!(v, U64),
        ColumnData::F32(v) => each!(v, F32),
        ColumnData::F64(v) => each!(v, F64),
    })
}

fn table_to_json(table: &Table) -> Value {
    let fields: Vec<Value> = table
        .fields
        .iter()
        .map(|f| match f.size {
            Some(size) => json!({ "name": f.name, "dtype": f.dtype.name(), "size": size }),
            None => json!({ "name": f.name, "dtype": f.dtype.name() }),
        })
        .collect();
    let columns: Map<String, Value> = table
        .fields
        .iter()
        .zip(&table.columns)
        .map(|(f, c)| (f.name.clone(), column_to_json(c)))
        .collect();
    json!({
        "fields": fields,
        "rows": table.rows,
        "columns": columns,
        "attrs": attrs_to_json(&table.attrs),
    })
}

pub fn tree_to_json(tree: &ExtensionTree) -> Value {
    let children: Map<String, Value> =
        tree.children.iter().map(|(n, c)| (n.clone(), tree_to_json(c))).collect();
    let datasets: Map<String, Value> = tree
        .datasets
        .iter()
        .map(|(n, ds)| {
            (
                n.clone(),
                json!({
                    "dtype": ds.dtype().name(),
                    "shape": ds.shape,
                    "data": data_to_json(&ds.data),
                    "attrs": attrs_to_json(&ds.attrs),
                }),
            )
        })
        .collect();
    let mut out =
        json!({ "attrs": attrs_to_json(&tree.attrs), "children": children, "datasets": datasets });
    if !tree.tables.is_empty() {
        let tables: Map<String, Value> =
            tree.tables.iter().map(|(n, t)| (n.clone(), table_to_json(t))).collect();
        out["tables"] = Value::Object(tables);
    }
    out
}

fn err(path: &str, message: impl Into<String>) -> ContractError {
    ContractError::new(path, message)
}

fn scalar_from_json(dtype: Dtype, v: &Value, path: &str) -> Result<Scalar, ContractError> {
    let int_range = |lo: i64, hi: i64| -> Result<i64, ContractError> {
        let n = v.as_i64().ok_or_else(|| err(path, format!("{} JSON value must be an integer", dtype.name())))?;
        if n < lo || n > hi {
            return Err(err(path, format!("{} value {n} is out of range", dtype.name())));
        }
        Ok(n)
    };
    Ok(match dtype {
        Dtype::Utf8 => Scalar::Utf8(
            v.as_str().ok_or_else(|| err(path, "utf8 JSON value must be a string"))?.to_string(),
        ),
        Dtype::I8 => Scalar::I8(int_range(i8::MIN.into(), i8::MAX.into())? as i8),
        Dtype::U8 => Scalar::U8(int_range(0, u8::MAX.into())? as u8),
        Dtype::I16 => Scalar::I16(int_range(i16::MIN.into(), i16::MAX.into())? as i16),
        Dtype::U16 => Scalar::U16(int_range(0, u16::MAX.into())? as u16),
        Dtype::I32 => Scalar::I32(int_range(i32::MIN.into(), i32::MAX.into())? as i32),
        Dtype::U32 => Scalar::U32(int_range(0, u32::MAX.into())? as u32),
        Dtype::I64 => Scalar::I64(
            v.as_str()
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| err(path, "i64 JSON value must be a decimal string"))?,
        ),
        Dtype::U64 => Scalar::U64(
            v.as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| err(path, "u64 JSON value must be a decimal string"))?,
        ),
        Dtype::F32 | Dtype::F64 => {
            let f = match v {
                Value::Number(n) => n.as_f64().ok_or_else(|| err(path, "invalid float JSON value"))?,
                Value::String(s) => match s.as_str() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    other => return Err(err(path, format!("invalid float JSON value \"{other}\""))),
                },
                _ => return Err(err(path, format!("{} JSON value must be a number", dtype.name()))),
            };
            if dtype == Dtype::F32 {
                Scalar::F32(f as f32)
            } else {
                Scalar::F64(f)
            }
        }
    })
}

fn dtype_from_json(v: Option<&Value>, path: &str) -> Result<Dtype, ContractError> {
    let name = v.and_then(Value::as_str).ok_or_else(|| err(path, "missing dtype"))?;
    Dtype::from_name(name).ok_or_else(|| err(path, format!("unknown dtype \"{name}\"")))
}

fn attrs_from_json(v: Option<&Value>, path: &str) -> Result<BTreeMap<String, Attr>, ContractError> {
    let mut out = BTreeMap::new();
    let Some(Value::Object(map)) = v else { return Ok(out) };
    for (name, a) in map {
        let attr_path = join(path, &format!("@{name}"));
        let dtype = dtype_from_json(a.get("dtype"), &attr_path)?;
        let raw = a.get("value").ok_or_else(|| err(&attr_path, "missing value"))?;
        let value = match raw {
            Value::Array(items) => AttrValue::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| scalar_from_json(dtype, item, &format!("{attr_path}[{i}]")))
                    .collect::<Result<_, _>>()?,
            ),
            other => AttrValue::Scalar(scalar_from_json(dtype, other, &attr_path)?),
        };
        out.insert(name.clone(), Attr { dtype, value });
    }
    Ok(out)
}

fn data_from_json(dtype: Dtype, items: &[Value], path: &str) -> Result<DatasetData, ContractError> {
    macro_rules! collect {
        ($variant:ident) => {{
            let mut out = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                match scalar_from_json(dtype, item, &format!("{path}[{i}]"))? {
                    Scalar::$variant(x) => out.push(x),
                    _ => unreachable!("scalar_from_json returns the requested dtype"),
                }
            }
            DatasetData::$variant(out)
        }};
    }
    Ok(match dtype {
        Dtype::I8 => collect!(I8),
        Dtype::U8 => collect!(U8),
        Dtype::I16 => collect!(I16),
        Dtype::U16 => collect!(U16),
        Dtype::I32 => collect!(I32),
        Dtype::U32 => collect!(U32),
        Dtype::I64 => collect!(I64),
        Dtype::U64 => collect!(U64),
        Dtype::F32 => collect!(F32),
        Dtype::F64 => collect!(F64),
        Dtype::Utf8 => return Err(err(path, "dataset dtype must be numeric, got \"utf8\"")),
    })
}

fn tree_from_json_at(json: &Value, path: &str) -> Result<ExtensionTree, ContractError> {
    let mut tree = ExtensionTree { attrs: attrs_from_json(json.get("attrs"), path)?, ..Default::default() };
    if let Some(Value::Object(children)) = json.get("children") {
        for (name, child) in children {
            tree.children.insert(name.clone(), tree_from_json_at(child, &join(path, name))?);
        }
    }
    if let Some(Value::Object(datasets)) = json.get("datasets") {
        for (name, ds) in datasets {
            let ds_path = join(path, name);
            let dtype = dtype_from_json(ds.get("dtype"), &ds_path)?;
            let shape = ds
                .get("shape")
                .and_then(Value::as_array)
                .ok_or_else(|| err(&ds_path, "missing shape"))?
                .iter()
                .map(|d| d.as_u64().map(|n| n as usize).ok_or_else(|| err(&ds_path, "invalid shape")))
                .collect::<Result<Vec<_>, _>>()?;
            let items = ds
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| err(&ds_path, "missing data"))?;
            tree.datasets.insert(
                name.clone(),
                Dataset {
                    shape,
                    data: data_from_json(dtype, items, &ds_path)?,
                    attrs: attrs_from_json(ds.get("attrs"), &ds_path)?,
                },
            );
        }
    }
    if let Some(Value::Object(tables)) = json.get("tables") {
        for (name, table) in tables {
            tree.tables.insert(name.clone(), table_from_json(table, &join(path, name))?);
        }
    }
    Ok(tree)
}

fn column_from_json(dtype: Dtype, items: &[Value], path: &str) -> Result<ColumnData, ContractError> {
    if dtype == Dtype::Utf8 {
        return items
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| err(&format!("{path}[{i}]"), "utf8 JSON value must be a string"))
            })
            .collect::<Result<_, _>>()
            .map(ColumnData::Utf8);
    }
    Ok(match data_from_json(dtype, items, path)? {
        DatasetData::I8(v) => ColumnData::I8(v),
        DatasetData::U8(v) => ColumnData::U8(v),
        DatasetData::I16(v) => ColumnData::I16(v),
        DatasetData::U16(v) => ColumnData::U16(v),
        DatasetData::I32(v) => ColumnData::I32(v),
        DatasetData::U32(v) => ColumnData::U32(v),
        DatasetData::I64(v) => ColumnData::I64(v),
        DatasetData::U64(v) => ColumnData::U64(v),
        DatasetData::F32(v) => ColumnData::F32(v),
        DatasetData::F64(v) => ColumnData::F64(v),
    })
}

fn table_from_json(json: &Value, path: &str) -> Result<Table, ContractError> {
    let field_values =
        json.get("fields").and_then(Value::as_array).ok_or_else(|| err(path, "missing fields"))?;
    let rows =
        json.get("rows").and_then(Value::as_u64).ok_or_else(|| err(path, "missing rows"))? as usize;
    let mut fields = Vec::with_capacity(field_values.len());
    let mut columns = Vec::with_capacity(field_values.len());
    for f in field_values {
        let name =
            f.get("name").and_then(Value::as_str).ok_or_else(|| err(path, "field is missing a name"))?;
        let field_path = join(path, name);
        let dtype = dtype_from_json(f.get("dtype"), &field_path)?;
        let size = f.get("size").and_then(Value::as_u64).map(|n| n as usize);
        let items = json
            .get("columns")
            .and_then(|c| c.get(name))
            .and_then(Value::as_array)
            .ok_or_else(|| err(&field_path, "missing column"))?;
        columns.push(column_from_json(dtype, items, &field_path)?);
        fields.push(TableField { name: name.to_string(), dtype, size });
    }
    Ok(Table { fields, rows, columns, attrs: attrs_from_json(json.get("attrs"), path)? })
}

/// Inverse of [`tree_to_json`]; the result is validated.
pub fn tree_from_json(json: &Value) -> Result<ExtensionTree, ContractError> {
    let tree = tree_from_json_at(json, "")?;
    validate_tree(&tree, "")?;
    Ok(tree)
}

impl Serialize for ExtensionTree {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        tree_to_json(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExtensionTree {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        tree_from_json(&value).map_err(D::Error::custom)
    }
}
