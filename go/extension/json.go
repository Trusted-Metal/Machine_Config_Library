package extension

import (
	"bytes"
	"encoding/json"
	"fmt"
	"math"
	"strconv"
)

// JSON form (used by fixtures and exports): i64/u64 as decimal strings, non-finite floats as
// "NaN" / "Infinity" / "-Infinity", so the JSON is lossless in every language:
//
//	{"attrs": {"Name": {"dtype": "u32", "value": 1}}, "children": {...},
//	 "datasets": {"Grid": {"dtype": "f64", "shape": [2,3], "data": [...], "attrs": {...}}}}

func scalarToJSON(dtype string, v any) any {
	switch n := v.(type) {
	case int64:
		if dtype == I64 {
			return strconv.FormatInt(n, 10)
		}
		return n
	case uint64:
		if dtype == U64 {
			return strconv.FormatUint(n, 10)
		}
		return n
	case float64:
		switch {
		case math.IsNaN(n):
			return "NaN"
		case math.IsInf(n, 1):
			return "Infinity"
		case math.IsInf(n, -1):
			return "-Infinity"
		}
		return n
	}
	return v
}

func valueToJSON(dtype string, value any) any {
	switch v := value.(type) {
	case []string:
		return append([]string{}, v...)
	case []int64:
		out := make([]any, len(v))
		for i, n := range v {
			out[i] = scalarToJSON(dtype, n)
		}
		return out
	case []uint64:
		out := make([]any, len(v))
		for i, n := range v {
			out[i] = scalarToJSON(dtype, n)
		}
		return out
	case []float64:
		out := make([]any, len(v))
		for i, n := range v {
			out[i] = scalarToJSON(dtype, n)
		}
		return out
	}
	return scalarToJSON(dtype, value)
}

func attrsToJSON(attrs map[string]Attr) map[string]any {
	out := make(map[string]any, len(attrs))
	for name, a := range attrs {
		out[name] = map[string]any{"dtype": a.Dtype, "value": valueToJSON(a.Dtype, a.Value)}
	}
	return out
}

// ToJSON returns the tree's lossless JSON form as a generic value ready for encoding/json.
func (t *Tree) ToJSON() map[string]any {
	children := make(map[string]any, len(t.Children))
	for name, c := range t.Children {
		children[name] = c.ToJSON()
	}
	datasets := make(map[string]any, len(t.Datasets))
	for name, ds := range t.Datasets {
		shape := make([]any, len(ds.Shape))
		for i, d := range ds.Shape {
			shape[i] = d
		}
		datasets[name] = map[string]any{
			"dtype": ds.Dtype,
			"shape": shape,
			"data":  valueToJSON(ds.Dtype, ds.Data),
			"attrs": attrsToJSON(ds.Attrs),
		}
	}
	out := map[string]any{"attrs": attrsToJSON(t.Attrs), "children": children, "datasets": datasets}
	if len(t.Tables) > 0 {
		tables := make(map[string]any, len(t.Tables))
		for name, table := range t.Tables {
			fields := make([]any, len(table.Fields))
			columns := make(map[string]any, len(table.Fields))
			for i, f := range table.Fields {
				entry := map[string]any{"name": f.Name, "dtype": f.Dtype}
				if f.Size > 0 {
					entry["size"] = f.Size
				}
				fields[i] = entry
				columns[f.Name] = valueToJSON(f.Dtype, table.Columns[f.Name])
			}
			tables[name] = map[string]any{
				"fields":  fields,
				"rows":    table.Rows,
				"columns": columns,
				"attrs":   attrsToJSON(table.Attrs),
			}
		}
		out["tables"] = tables
	}
	return out
}

// MarshalJSON encodes the lossless JSON form.
func (t Tree) MarshalJSON() ([]byte, error) { return json.Marshal(t.ToJSON()) }

// UnmarshalJSON decodes the lossless JSON form; the result is validated.
func (t *Tree) UnmarshalJSON(data []byte) error {
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.UseNumber()
	var raw map[string]any
	if err := dec.Decode(&raw); err != nil {
		return err
	}
	parsed, err := treeFromJSON(raw, "")
	if err != nil {
		return err
	}
	if err := Validate(parsed, ""); err != nil {
		return err
	}
	*t = *parsed
	return nil
}

// FromJSON is the inverse of ToJSON for a decoded generic value; the result is validated.
func FromJSON(data []byte) (*Tree, error) {
	var t Tree
	if err := t.UnmarshalJSON(data); err != nil {
		return nil, err
	}
	return &t, nil
}

func jsonNumber(v any) (json.Number, bool) {
	n, ok := v.(json.Number)
	return n, ok
}

func scalarFromJSON(dtype string, v any, path string) (any, error) {
	switch {
	case dtype == Utf8:
		s, ok := v.(string)
		if !ok {
			return nil, errAt(path, "utf8 JSON value must be a string")
		}
		return s, nil
	case dtype == I64:
		s, ok := v.(string)
		if !ok {
			return nil, errAt(path, "i64 JSON value must be a decimal string")
		}
		n, err := strconv.ParseInt(s, 10, 64)
		if err != nil {
			return nil, errAt(path, "i64 JSON value %q is not a valid int64", s)
		}
		return n, nil
	case dtype == U64:
		s, ok := v.(string)
		if !ok {
			return nil, errAt(path, "u64 JSON value must be a decimal string")
		}
		n, err := strconv.ParseUint(s, 10, 64)
		if err != nil {
			return nil, errAt(path, "u64 JSON value %q is not a valid uint64", s)
		}
		return n, nil
	case isFloat(dtype):
		if s, ok := v.(string); ok {
			switch s {
			case "NaN":
				return math.NaN(), nil
			case "Infinity":
				return math.Inf(1), nil
			case "-Infinity":
				return math.Inf(-1), nil
			}
			return nil, errAt(path, "invalid float JSON value %q", s)
		}
		n, ok := jsonNumber(v)
		if !ok {
			return nil, errAt(path, "%s JSON value must be a number", dtype)
		}
		f, err := n.Float64()
		if err != nil {
			return nil, errAt(path, "%s JSON value %s is not a valid number", dtype, n)
		}
		return f, nil
	default: // i8..i32, u8..u32
		n, ok := jsonNumber(v)
		if !ok {
			return nil, errAt(path, "%s JSON value must be a number", dtype)
		}
		if isSigned(dtype) {
			i, err := strconv.ParseInt(n.String(), 10, 64)
			if err != nil {
				f, ferr := n.Float64()
				if ferr != nil || f != math.Trunc(f) || math.Abs(f) > 1<<53 {
					return nil, errAt(path, "%s JSON value %s must be an integer", dtype, n)
				}
				i = int64(f)
			}
			if err := checkSigned(dtype, i, path); err != nil {
				return nil, err
			}
			return i, nil
		}
		u, err := strconv.ParseUint(n.String(), 10, 64)
		if err != nil {
			f, ferr := n.Float64()
			if ferr != nil || f != math.Trunc(f) || f < 0 || f > 1<<53 {
				return nil, errAt(path, "%s JSON value %s must be a non-negative integer", dtype, n)
			}
			u = uint64(f)
		}
		if err := checkUnsigned(dtype, u, path); err != nil {
			return nil, err
		}
		return u, nil
	}
}

// valueFromJSON decodes one JSON value (scalar or array) into the dtype's Go shape.
func valueFromJSON(dtype string, raw any, path string) (any, error) {
	list, isList := raw.([]any)
	if !isList {
		return scalarFromJSON(dtype, raw, path)
	}
	return sliceFromJSON(dtype, list, path)
}

func sliceFromJSON(dtype string, list []any, path string) (any, error) {
	switch {
	case dtype == Utf8:
		out := make([]string, len(list))
		for i, e := range list {
			v, err := scalarFromJSON(dtype, e, fmt.Sprintf("%s[%d]", path, i))
			if err != nil {
				return nil, err
			}
			out[i] = v.(string)
		}
		return out, nil
	case isSigned(dtype):
		out := make([]int64, len(list))
		for i, e := range list {
			v, err := scalarFromJSON(dtype, e, fmt.Sprintf("%s[%d]", path, i))
			if err != nil {
				return nil, err
			}
			out[i] = v.(int64)
		}
		return out, nil
	case isUnsigned(dtype):
		out := make([]uint64, len(list))
		for i, e := range list {
			v, err := scalarFromJSON(dtype, e, fmt.Sprintf("%s[%d]", path, i))
			if err != nil {
				return nil, err
			}
			out[i] = v.(uint64)
		}
		return out, nil
	default:
		out := make([]float64, len(list))
		for i, e := range list {
			v, err := scalarFromJSON(dtype, e, fmt.Sprintf("%s[%d]", path, i))
			if err != nil {
				return nil, err
			}
			out[i] = v.(float64)
		}
		return out, nil
	}
}

func attrsFromJSON(raw any, path string) (map[string]Attr, error) {
	out := map[string]Attr{}
	m, _ := raw.(map[string]any)
	for name, v := range m {
		attrPath := joinPath(path, "@"+name)
		entry, ok := v.(map[string]any)
		if !ok {
			return nil, errAt(attrPath, "attribute must be an object")
		}
		dtype, _ := entry["dtype"].(string)
		if !IsKnownDtype(dtype) {
			return nil, errAt(attrPath, "unknown dtype %q", dtype)
		}
		value, err := valueFromJSON(dtype, entry["value"], attrPath)
		if err != nil {
			return nil, err
		}
		out[name] = Attr{Dtype: dtype, Value: value}
	}
	return out, nil
}

func treeFromJSON(raw map[string]any, path string) (*Tree, error) {
	t := NewTree()
	attrs, err := attrsFromJSON(raw["attrs"], path)
	if err != nil {
		return nil, err
	}
	t.Attrs = attrs
	children, _ := raw["children"].(map[string]any)
	for name, c := range children {
		cm, ok := c.(map[string]any)
		if !ok {
			return nil, errAt(joinPath(path, name), "child must be an object")
		}
		child, err := treeFromJSON(cm, joinPath(path, name))
		if err != nil {
			return nil, err
		}
		t.Children[name] = child
	}
	datasets, _ := raw["datasets"].(map[string]any)
	for name, d := range datasets {
		dsPath := joinPath(path, name)
		dm, ok := d.(map[string]any)
		if !ok {
			return nil, errAt(dsPath, "dataset must be an object")
		}
		dtype, _ := dm["dtype"].(string)
		if !IsKnownDtype(dtype) || dtype == Utf8 {
			return nil, errAt(dsPath, "dataset dtype must be numeric, got %q", dtype)
		}
		var shape []uint64
		shapeRaw, _ := dm["shape"].([]any)
		for _, e := range shapeRaw {
			n, ok := jsonNumber(e)
			if !ok {
				return nil, errAt(dsPath, "shape entries must be numbers")
			}
			u, err := strconv.ParseUint(n.String(), 10, 64)
			if err != nil {
				return nil, errAt(dsPath, "invalid shape entry %s", n)
			}
			shape = append(shape, u)
		}
		dataRaw, _ := dm["data"].([]any)
		data, err := sliceFromJSON(dtype, dataRaw, dsPath)
		if err != nil {
			return nil, err
		}
		dsAttrs, err := attrsFromJSON(dm["attrs"], dsPath)
		if err != nil {
			return nil, err
		}
		t.Datasets[name] = &Dataset{Dtype: dtype, Shape: shape, Data: data, Attrs: dsAttrs}
	}
	tables, _ := raw["tables"].(map[string]any)
	for name, tm := range tables {
		tableRaw, ok := tm.(map[string]any)
		if !ok {
			return nil, errAt(joinPath(path, name), "table must be an object")
		}
		table, err := tableFromJSON(tableRaw, joinPath(path, name))
		if err != nil {
			return nil, err
		}
		t.Tables[name] = table
	}
	return t, nil
}

func tableFromJSON(raw map[string]any, path string) (*Table, error) {
	table := &Table{Columns: map[string]any{}}
	rowsNum, ok := jsonNumber(raw["rows"])
	if !ok {
		return nil, errAt(path, "missing rows")
	}
	rows, err := strconv.ParseUint(rowsNum.String(), 10, 64)
	if err != nil {
		return nil, errAt(path, "invalid rows %s", rowsNum)
	}
	table.Rows = rows
	fieldsRaw, _ := raw["fields"].([]any)
	columnsRaw, _ := raw["columns"].(map[string]any)
	for _, fr := range fieldsRaw {
		fm, ok := fr.(map[string]any)
		if !ok {
			return nil, errAt(path, "field must be an object")
		}
		name, _ := fm["name"].(string)
		fieldPath := joinPath(path, name)
		dtype, _ := fm["dtype"].(string)
		if !IsKnownDtype(dtype) {
			return nil, errAt(fieldPath, "unknown dtype %q", dtype)
		}
		field := TableField{Name: name, Dtype: dtype}
		if sizeNum, ok := jsonNumber(fm["size"]); ok {
			size, err := strconv.Atoi(sizeNum.String())
			if err != nil {
				return nil, errAt(fieldPath, "invalid size %s", sizeNum)
			}
			field.Size = size
		}
		list, ok := columnsRaw[name].([]any)
		if !ok {
			return nil, errAt(fieldPath, "missing column")
		}
		column, err := sliceFromJSON(dtype, list, fieldPath)
		if err != nil {
			return nil, err
		}
		table.Fields = append(table.Fields, field)
		table.Columns[name] = column
	}
	attrs, err := attrsFromJSON(raw["attrs"], path)
	if err != nil {
		return nil, err
	}
	table.Attrs = attrs
	return table, nil
}
