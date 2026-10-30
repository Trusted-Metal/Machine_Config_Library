// Package extension is the extension contract: the opaque, language-neutral tree core uses to carry
// data that belongs to an extension (a group under Extensions/<name>/), so a separate library can
// pack and unpack it without core knowing its schema.
//
// Dependency-free on purpose (no HDF5, no cgo): extension libraries import only this package. Core
// reads and writes every tree verbatim, so unknown or private extensions round-trip untouched.
//
// Pre-stable (0.x) until a second extension has been ported against it. Every language produces
// the identical canonical digest and JSON form; see docs/EXTENSION_CONTRACT.md.
package extension

import (
	"fmt"
	"math"
	"strings"
)

// ContractVersion is the version of this contract.
const ContractVersion = "0.1.1"

// Fixed-width storage types; each maps 1:1 to an HDF5 type.
const (
	Utf8 = "utf8"
	I8   = "i8"
	U8   = "u8"
	I16  = "i16"
	U16  = "u16"
	I32  = "i32"
	U32  = "u32"
	I64  = "i64"
	U64  = "u64"
	F32  = "f32"
	F64  = "f64"
)

// Value shapes by dtype: utf8 -> string/[]string, i8..i64 -> int64/[]int64,
// u8..u64 -> uint64/[]uint64, f32/f64 -> float64/[]float64.

var dtypeTag = map[string]byte{
	Utf8: 1, I8: 2, U8: 3, I16: 4, U16: 5, I32: 6, U32: 7, I64: 8, U64: 9, F32: 10, F64: 11,
}

type intRange struct {
	lo, hi int64
}

var signedRanges = map[string]intRange{
	I8:  {math.MinInt8, math.MaxInt8},
	I16: {math.MinInt16, math.MaxInt16},
	I32: {math.MinInt32, math.MaxInt32},
	I64: {math.MinInt64, math.MaxInt64},
}

var unsignedMax = map[string]uint64{
	U8: math.MaxUint8, U16: math.MaxUint16, U32: math.MaxUint32, U64: math.MaxUint64,
}

// IsKnownDtype reports whether dtype is one of the contract's eleven storage types.
func IsKnownDtype(dtype string) bool {
	_, ok := dtypeTag[dtype]
	return ok
}

func isSigned(dtype string) bool   { _, ok := signedRanges[dtype]; return ok }
func isUnsigned(dtype string) bool { _, ok := unsignedMax[dtype]; return ok }
func isFloat(dtype string) bool    { return dtype == F32 || dtype == F64 }

// Error names the first invalid node. Path is the slash-joined location; attributes are prefixed @.
type Error struct {
	Path    string
	Message string
}

func (e *Error) Error() string {
	p := e.Path
	if p == "" {
		p = "<root>"
	}
	return p + ": " + e.Message
}

func errAt(path, format string, a ...any) *Error {
	return &Error{Path: path, Message: fmt.Sprintf(format, a...)}
}

// Attr is a group or dataset attribute: a scalar or a 1-D slice of one dtype.
type Attr struct {
	Dtype string
	Value any
}

// Dataset is an N-d numeric array stored row-major in Data (a flat slice); len(Data) equals the
// product of Shape (an empty Shape is a scalar).
type Dataset struct {
	Dtype string
	Shape []uint64
	Data  any
	Attrs map[string]Attr
}

// TableField is one column of a Table. Size is the fixed byte length of a utf8 column and 0 for
// numeric ones.
type TableField struct {
	Name  string
	Dtype string
	Size  int
}

// Table is a 1-D compound dataset (HDF5 compound type): one row per record, one named column per
// field. Fields order is the compound member order and is significant. Columns[name] holds Rows
// values in the dtype's Go shape ([]string, []int64, []uint64 or []float64).
type Table struct {
	Fields  []TableField
	Rows    uint64
	Columns map[string]any
	Attrs   map[string]Attr
}

// Tree is one group: its attributes, child groups, datasets and tables. Order is not significant;
// the canonical form sorts by UTF-8 bytes. Tables is empty unless used, so a tree without compound
// data is encoded and hashed exactly as in contract 0.1.0.
type Tree struct {
	Attrs    map[string]Attr
	Children map[string]*Tree
	Datasets map[string]*Dataset
	Tables   map[string]*Table
}

// NewTree returns an empty tree with initialised maps.
func NewTree() *Tree {
	return &Tree{
		Attrs:    map[string]Attr{},
		Children: map[string]*Tree{},
		Datasets: map[string]*Dataset{},
		Tables:   map[string]*Table{},
	}
}

func joinPath(path, name string) string {
	if path == "" {
		return name
	}
	return path + "/" + name
}

// ValidateNodeName checks a name that becomes an HDF5 link name: non-empty, no "/", no NUL.
func ValidateNodeName(name, path string) error {
	switch {
	case name == "":
		return errAt(path, "name must not be empty")
	case strings.Contains(name, "/"):
		return errAt(path, "name %q must not contain \"/\"", name)
	case strings.ContainsRune(name, 0):
		return errAt(path, "name must not contain NUL")
	}
	return nil
}

func checkSigned(dtype string, v int64, path string) error {
	r := signedRanges[dtype]
	if v < r.lo || v > r.hi {
		return errAt(path, "%s value %d is out of range", dtype, v)
	}
	return nil
}

func checkUnsigned(dtype string, v uint64, path string) error {
	if v > unsignedMax[dtype] {
		return errAt(path, "%s value %d is out of range", dtype, v)
	}
	return nil
}

// checkValue validates a scalar (when slice is false) or a slice of the dtype's Go shape.
func checkValue(dtype string, value any, path string) error {
	switch {
	case dtype == Utf8:
		switch v := value.(type) {
		case string:
			if strings.ContainsRune(v, 0) {
				return errAt(path, "utf8 value must not contain NUL")
			}
		case []string:
			for i, s := range v {
				if strings.ContainsRune(s, 0) {
					return errAt(fmt.Sprintf("%s[%d]", path, i), "utf8 value must not contain NUL")
				}
			}
		default:
			return errAt(path, "utf8 value must be a string or []string, got %T", value)
		}
	case isSigned(dtype):
		switch v := value.(type) {
		case int64:
			return checkSigned(dtype, v, path)
		case []int64:
			for i, n := range v {
				if err := checkSigned(dtype, n, fmt.Sprintf("%s[%d]", path, i)); err != nil {
					return err
				}
			}
		default:
			return errAt(path, "%s value must be int64 or []int64, got %T", dtype, value)
		}
	case isUnsigned(dtype):
		switch v := value.(type) {
		case uint64:
			return checkUnsigned(dtype, v, path)
		case []uint64:
			for i, n := range v {
				if err := checkUnsigned(dtype, n, fmt.Sprintf("%s[%d]", path, i)); err != nil {
					return err
				}
			}
		default:
			return errAt(path, "%s value must be uint64 or []uint64, got %T", dtype, value)
		}
	default: // f32, f64
		switch value.(type) {
		case float64, []float64:
		default:
			return errAt(path, "%s value must be float64 or []float64, got %T", dtype, value)
		}
	}
	return nil
}

func validateAttrs(attrs map[string]Attr, path string) error {
	for name, a := range attrs {
		attrPath := joinPath(path, "@"+name)
		if err := ValidateNodeName(name, attrPath); err != nil {
			return err
		}
		if !IsKnownDtype(a.Dtype) {
			return errAt(attrPath, "unknown dtype %q", a.Dtype)
		}
		if err := checkValue(a.Dtype, a.Value, attrPath); err != nil {
			return err
		}
	}
	return nil
}

func dataLen(data any) (int, bool) {
	switch d := data.(type) {
	case []int64:
		return len(d), true
	case []uint64:
		return len(d), true
	case []float64:
		return len(d), true
	}
	return 0, false
}

// Validate returns an *Error naming the first invalid node, or nil. path prefixes every reported
// location (use the extension's name, or "" for none).
func Validate(t *Tree, path string) error {
	if t == nil {
		return errAt(path, "tree must not be nil")
	}
	if err := validateAttrs(t.Attrs, path); err != nil {
		return err
	}
	for name, child := range t.Children {
		childPath := joinPath(path, name)
		if err := ValidateNodeName(name, childPath); err != nil {
			return err
		}
		if err := Validate(child, childPath); err != nil {
			return err
		}
	}
	for name, ds := range t.Datasets {
		dsPath := joinPath(path, name)
		if err := ValidateNodeName(name, dsPath); err != nil {
			return err
		}
		if _, clash := t.Children[name]; clash {
			return errAt(dsPath, "a dataset and a child group cannot share a name")
		}
		if ds == nil {
			return errAt(dsPath, "dataset must not be nil")
		}
		if !IsKnownDtype(ds.Dtype) || ds.Dtype == Utf8 {
			return errAt(dsPath, "dataset dtype must be numeric, got %q", ds.Dtype)
		}
		expected := uint64(1)
		for _, d := range ds.Shape {
			if d != 0 && expected > math.MaxInt32/d {
				return errAt(dsPath, "shape %v is too large", ds.Shape)
			}
			expected *= d
		}
		n, ok := dataLen(ds.Data)
		if !ok {
			return errAt(dsPath, "dataset data must be []int64, []uint64 or []float64, got %T", ds.Data)
		}
		if uint64(n) != expected {
			return errAt(dsPath, "data length %d does not match shape %v", n, ds.Shape)
		}
		if err := checkValue(ds.Dtype, ds.Data, dsPath); err != nil {
			return err
		}
		if err := validateAttrs(ds.Attrs, dsPath); err != nil {
			return err
		}
	}
	for name, table := range t.Tables {
		tablePath := joinPath(path, name)
		if err := ValidateNodeName(name, tablePath); err != nil {
			return err
		}
		_, isChild := t.Children[name]
		_, isDataset := t.Datasets[name]
		if isChild || isDataset {
			return errAt(tablePath, "a table cannot share a name with a dataset or child group")
		}
		if err := validateTable(table, tablePath); err != nil {
			return err
		}
	}
	return nil
}

func validateTable(table *Table, path string) error {
	if table == nil {
		return errAt(path, "table must not be nil")
	}
	if len(table.Fields) == 0 {
		return errAt(path, "a table needs at least one field")
	}
	names := map[string]bool{}
	for _, f := range table.Fields {
		fieldPath := joinPath(path, f.Name)
		if err := ValidateNodeName(f.Name, fieldPath); err != nil {
			return err
		}
		if names[f.Name] {
			return errAt(fieldPath, "duplicate field name")
		}
		names[f.Name] = true
		if !IsKnownDtype(f.Dtype) {
			return errAt(fieldPath, "unknown dtype %q", f.Dtype)
		}
		column, ok := table.Columns[f.Name]
		if !ok {
			return errAt(fieldPath, "missing column")
		}
		if f.Dtype == Utf8 {
			if f.Size < 1 {
				return errAt(fieldPath, "a utf8 field needs a positive integer size")
			}
		} else if f.Size != 0 {
			return errAt(fieldPath, "size is only valid for utf8 fields")
		}
		n, ok := columnLen(column)
		if !ok {
			return errAt(fieldPath, "column must be []string, []int64, []uint64 or []float64, got %T", column)
		}
		if uint64(n) != table.Rows {
			return errAt(fieldPath, "column length %d does not match %d rows", n, table.Rows)
		}
		if _, isStrings := column.([]string); isStrings != (f.Dtype == Utf8) {
			return errAt(fieldPath, "column is %T but the field dtype is %s", column, f.Dtype)
		}
		if err := checkValue(f.Dtype, column, fieldPath); err != nil {
			return err
		}
		if strs, isStrings := column.([]string); isStrings {
			for i, s := range strs {
				if len(s) > f.Size {
					return errAt(fmt.Sprintf("%s[%d]", fieldPath, i), "%d UTF-8 bytes do not fit the %d-byte field", len(s), f.Size)
				}
			}
		}
	}
	for name := range table.Columns {
		if !names[name] {
			return errAt(joinPath(path, name), "column has no matching field")
		}
	}
	return validateAttrs(table.Attrs, path)
}

func columnLen(column any) (int, bool) {
	if s, ok := column.([]string); ok {
		return len(s), true
	}
	return dataLen(column)
}
