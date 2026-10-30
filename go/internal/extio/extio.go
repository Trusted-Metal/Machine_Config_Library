// Package extio maps extension.Tree to and from HDF5 groups.
//
// Version-independent: the adapters decide where an extension lives (Extensions/<name>/ in 1.1, the
// legacy OPCUA/ group in 1.0); this package only maps the contract's tree to and from the HDF5 group
// at that location. Everything is read and written verbatim with dtypes preserved, so extensions
// core has no model for survive a read-modify-write.
package extio

import (
	"errors"
	"fmt"

	"machine-config-go/extension"
	"machine-config-go/internal/h5c"
)

func joinPath(path, name string) string {
	if path == "" {
		return name
	}
	return path + "/" + name
}

type attrHost interface {
	TypedAttrNames() []string
	ReadTypedAttr(name string) (h5c.TypedValue, error)
	WriteTypedAttr(name string, v h5c.TypedValue) error
}

func attrFromTyped(v h5c.TypedValue) extension.Attr {
	a := extension.Attr{Dtype: v.Dtype}
	switch {
	case v.Dtype == extension.Utf8:
		if v.Scalar {
			a.Value = v.Strs[0]
		} else {
			a.Value = v.Strs
		}
	case isSignedDtype(v.Dtype):
		if v.Scalar {
			a.Value = v.Ints[0]
		} else {
			a.Value = emptyIfNil(v.Ints)
		}
	case isUnsignedDtype(v.Dtype):
		if v.Scalar {
			a.Value = v.Uints[0]
		} else {
			a.Value = emptyIfNilU(v.Uints)
		}
	default:
		if v.Scalar {
			a.Value = v.Floats[0]
		} else {
			a.Value = emptyIfNilF(v.Floats)
		}
	}
	return a
}

func isSignedDtype(d string) bool   { return d == "i8" || d == "i16" || d == "i32" || d == "i64" }
func isUnsignedDtype(d string) bool { return d == "u8" || d == "u16" || d == "u32" || d == "u64" }

func emptyIfNil(s []int64) []int64 {
	if s == nil {
		return []int64{}
	}
	return s
}

func emptyIfNilU(s []uint64) []uint64 {
	if s == nil {
		return []uint64{}
	}
	return s
}

func emptyIfNilF(s []float64) []float64 {
	if s == nil {
		return []float64{}
	}
	return s
}

func typedFromAttr(a extension.Attr) h5c.TypedValue {
	v := h5c.TypedValue{Dtype: a.Dtype}
	switch x := a.Value.(type) {
	case string:
		v.Scalar, v.Strs = true, []string{x}
	case []string:
		v.Strs = x
	case int64:
		v.Scalar, v.Ints = true, []int64{x}
	case []int64:
		v.Ints = x
	case uint64:
		v.Scalar, v.Uints = true, []uint64{x}
	case []uint64:
		v.Uints = x
	case float64:
		v.Scalar, v.Floats = true, []float64{x}
	case []float64:
		v.Floats = x
	}
	return v
}

func readAttrs(h attrHost, path string) (map[string]extension.Attr, error) {
	out := map[string]extension.Attr{}
	for _, name := range h.TypedAttrNames() {
		v, err := h.ReadTypedAttr(name)
		if err != nil {
			return nil, &extension.Error{Path: joinPath(path, "@"+name), Message: err.Error()}
		}
		out[name] = attrFromTyped(v)
	}
	return out, nil
}

// ReadTree reads a group and everything below it verbatim; unsupported HDF5 types fail with their
// path. path prefixes every reported location (use the extension's name).
func ReadTree(g *h5c.Group, path string) (*extension.Tree, error) {
	attrs, err := readAttrs(g, path)
	if err != nil {
		return nil, err
	}
	tree := extension.NewTree()
	tree.Attrs = attrs

	members, err := g.Members()
	if err != nil {
		return nil, &extension.Error{Path: path, Message: err.Error()}
	}
	for _, m := range members {
		childPath := joinPath(path, m.Name)
		switch {
		case m.IsGroup:
			cg, err := g.OpenGroup(m.Name)
			if err != nil {
				return nil, &extension.Error{Path: childPath, Message: err.Error()}
			}
			child, err := ReadTree(cg, childPath)
			cg.Close()
			if err != nil {
				return nil, err
			}
			tree.Children[m.Name] = child
		case m.IsDataset:
			if g.IsCompoundDataset(m.Name) {
				table, err := readTable(g, m.Name, childPath)
				if err != nil {
					return nil, err
				}
				tree.Tables[m.Name] = table
				continue
			}
			ds, err := readDataset(g, m.Name, childPath)
			if err != nil {
				return nil, err
			}
			tree.Datasets[m.Name] = ds
		default:
			return nil, &extension.Error{Path: childPath, Message: "only groups and datasets are supported"}
		}
	}
	return tree, nil
}

func readDataset(g *h5c.Group, name, path string) (*extension.Dataset, error) {
	v, dims, err := g.ReadTypedDataset(name)
	if err != nil {
		return nil, &extension.Error{Path: path, Message: err.Error()}
	}
	h5ds, err := g.OpenDataset(name)
	if err != nil {
		return nil, &extension.Error{Path: path, Message: err.Error()}
	}
	defer h5ds.Close()
	attrs, err := readAttrs(h5ds, path)
	if err != nil {
		return nil, err
	}
	ds := &extension.Dataset{Dtype: v.Dtype, Shape: dims, Attrs: attrs}
	switch {
	case isSignedDtype(v.Dtype):
		ds.Data = emptyIfNil(v.Ints)
	case isUnsignedDtype(v.Dtype):
		ds.Data = emptyIfNilU(v.Uints)
	default:
		ds.Data = emptyIfNilF(v.Floats)
	}
	return ds, nil
}

func readTable(g *h5c.Group, name, path string) (*extension.Table, error) {
	cols, rows, err := g.ReadCompoundDataset(name)
	if err != nil {
		var member *h5c.MemberError
		if errors.As(err, &member) {
			return nil, &extension.Error{Path: joinPath(path, member.Member), Message: member.Message}
		}
		return nil, &extension.Error{Path: path, Message: err.Error()}
	}
	h5ds, err := g.OpenDataset(name)
	if err != nil {
		return nil, &extension.Error{Path: path, Message: err.Error()}
	}
	defer h5ds.Close()
	attrs, err := readAttrs(h5ds, path)
	if err != nil {
		return nil, err
	}
	table := &extension.Table{Rows: rows, Columns: map[string]any{}, Attrs: attrs}
	for _, c := range cols {
		table.Fields = append(table.Fields, extension.TableField{Name: c.Name, Dtype: c.Dtype, Size: c.Size})
		switch {
		case c.Dtype == extension.Utf8:
			if c.Strs == nil {
				c.Strs = []string{}
			}
			table.Columns[c.Name] = c.Strs
		case isSignedDtype(c.Dtype):
			table.Columns[c.Name] = emptyIfNil(c.Ints)
		case isUnsignedDtype(c.Dtype):
			table.Columns[c.Name] = emptyIfNilU(c.Uints)
		default:
			table.Columns[c.Name] = emptyIfNilF(c.Floats)
		}
	}
	return table, nil
}

func writeTable(g *h5c.Group, name string, t *extension.Table, path string) error {
	cols := make([]h5c.CompoundColumn, len(t.Fields))
	for i, f := range t.Fields {
		col := h5c.CompoundColumn{Name: f.Name, Dtype: f.Dtype, Size: f.Size}
		switch v := t.Columns[f.Name].(type) {
		case []string:
			col.Strs = v
		case []int64:
			col.Ints = v
		case []uint64:
			col.Uints = v
		case []float64:
			col.Floats = v
		}
		cols[i] = col
	}
	h5ds, err := g.CreateCompoundDataset(name, t.Rows, cols)
	if err != nil {
		return &extension.Error{Path: path, Message: err.Error()}
	}
	err = writeAttrs(h5ds, t.Attrs, path)
	h5ds.Close()
	return err
}

func writeAttrs(h attrHost, attrs map[string]extension.Attr, path string) error {
	for name, a := range attrs {
		if err := h.WriteTypedAttr(name, typedFromAttr(a)); err != nil {
			return &extension.Error{Path: joinPath(path, "@"+name), Message: err.Error()}
		}
	}
	return nil
}

func writeGroup(g *h5c.Group, t *extension.Tree, path string) error {
	if err := writeAttrs(g, t.Attrs, path); err != nil {
		return err
	}
	for name, ds := range t.Datasets {
		dsPath := joinPath(path, name)
		var v h5c.TypedValue
		v.Dtype = ds.Dtype
		switch d := ds.Data.(type) {
		case []int64:
			v.Ints = d
		case []uint64:
			v.Uints = d
		case []float64:
			v.Floats = d
		}
		h5ds, err := g.CreateTypedDataset(name, ds.Shape, v)
		if err != nil {
			return &extension.Error{Path: dsPath, Message: err.Error()}
		}
		err = writeAttrs(h5ds, ds.Attrs, dsPath)
		h5ds.Close()
		if err != nil {
			return err
		}
	}
	for name, table := range t.Tables {
		if err := writeTable(g, name, table, joinPath(path, name)); err != nil {
			return err
		}
	}
	for name, child := range t.Children {
		childPath := joinPath(path, name)
		cg, err := g.CreateGroup(name)
		if err != nil {
			return &extension.Error{Path: childPath, Message: err.Error()}
		}
		err = writeGroup(cg, child, childPath)
		cg.Close()
		if err != nil {
			return err
		}
	}
	return nil
}

// WriteTree validates t and writes it as a new group name under parent; the caller guarantees name
// is unused.
func WriteTree(parent *h5c.Group, name string, t *extension.Tree) error {
	if err := extension.Validate(t, name); err != nil {
		return err
	}
	g, err := parent.CreateGroup(name)
	if err != nil {
		return fmt.Errorf("create extension group %q: %w", name, err)
	}
	defer g.Close()
	return writeGroup(g, t, name)
}
