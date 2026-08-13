package hdf5

import (
	"fmt"
	"strconv"
	"strings"

	"machine-config-go/internal/h5c"
)

// readStrAttr returns nil for absent or empty-string attributes (Rule 3).
func readStrAttr(g *h5c.Group, key string) *string {
	if !g.HasAttr(key) {
		return nil
	}
	s, err := g.ReadStringAttr(key)
	if err != nil {
		return nil
	}
	s = strings.TrimRight(s, "\x00")
	if s == "" {
		return nil
	}
	return &s
}

// readRequiredStr returns "" when absent (for required identity fields).
func readRequiredStr(g *h5c.Group, key string) string {
	p := readStrAttr(g, key)
	if p == nil {
		return ""
	}
	return *p
}

// readFloatAttr returns nil for absent or empty-string attributes.
func readFloatAttr(g *h5c.Group, key string) (*float64, error) {
	kind, err := g.PeekAttr(key)
	if err != nil {
		return nil, err
	}
	switch kind {
	case h5c.AttrMissing:
		return nil, nil
	case h5c.AttrFloat:
		v, err := g.ReadFloat64Attr(key)
		if err != nil {
			return nil, err
		}
		return &v, nil
	case h5c.AttrInt:
		iv, err := g.ReadInt64Attr(key)
		if err != nil {
			return nil, err
		}
		f := float64(iv)
		return &f, nil
	case h5c.AttrString:
		s, err := g.ReadStringAttr(key)
		if err != nil {
			return nil, err
		}
		s = strings.TrimSpace(strings.TrimRight(s, "\x00"))
		if s == "" {
			return nil, nil
		}
		f, err := strconv.ParseFloat(s, 64)
		if err != nil {
			return nil, fmt.Errorf("attribute %q: non-numeric string %q", key, s)
		}
		return &f, nil
	default:
		return nil, fmt.Errorf("attribute %q: unsupported type", key)
	}
}

// readIntAttr returns nil for absent or empty-string attributes.
func readIntAttr(g *h5c.Group, key string) (*int, error) {
	kind, err := g.PeekAttr(key)
	if err != nil {
		return nil, err
	}
	switch kind {
	case h5c.AttrMissing:
		return nil, nil
	case h5c.AttrInt:
		iv, err := g.ReadInt64Attr(key)
		if err != nil {
			return nil, err
		}
		i := int(iv)
		return &i, nil
	case h5c.AttrFloat:
		fv, err := g.ReadFloat64Attr(key)
		if err != nil {
			return nil, err
		}
		i := int(fv)
		return &i, nil
	case h5c.AttrString:
		s, err := g.ReadStringAttr(key)
		if err != nil {
			return nil, err
		}
		s = strings.TrimSpace(strings.TrimRight(s, "\x00"))
		if s == "" {
			return nil, nil
		}
		iv, err := strconv.ParseInt(s, 10, 64)
		if err != nil {
			return nil, fmt.Errorf("attribute %q: non-integer string %q", key, s)
		}
		if strconv.IntSize == 32 && (iv < -(1<<31) || iv > (1<<31)-1) {
			return nil, fmt.Errorf("attribute %q: integer %q out of range for int", key, s)
		}
		i := int(iv)
		return &i, nil
	default:
		return nil, fmt.Errorf("attribute %q: unsupported type", key)
	}
}

// readBoolFromIntAttr implements Rule 4: HDF5 int 0/1 → *bool.
func readBoolFromIntAttr(g *h5c.Group, key string) (*bool, error) {
	kind, err := g.PeekAttr(key)
	if err != nil {
		return nil, err
	}
	if kind == h5c.AttrMissing {
		return nil, nil
	}
	var iv int64
	switch kind {
	case h5c.AttrInt:
		iv, err = g.ReadInt64Attr(key)
	case h5c.AttrFloat:
		var fv float64
		fv, err = g.ReadFloat64Attr(key)
		iv = int64(fv)
	case h5c.AttrString:
		var s string
		s, err = g.ReadStringAttr(key)
		if err == nil {
			s = strings.TrimSpace(s)
			if s == "" {
				return nil, nil
			}
			iv, err = strconv.ParseInt(s, 10, 64)
		}
	default:
		return nil, fmt.Errorf("attribute %q: expected int 0/1", key)
	}
	if err != nil {
		return nil, err
	}
	if iv != 0 && iv != 1 {
		return nil, fmt.Errorf("attribute %q: expected 0 or 1, got %d", key, iv)
	}
	b := iv == 1
	return &b, nil
}

// readStrLocked reads a string unit attr and optionally checks expected value.
func readStrLocked(g *h5c.Group, key, expected string) (*string, error) {
	p := readStrAttr(g, key)
	if p == nil {
		return nil, nil
	}
	if expected != "" && *p != expected {
		return p, fmt.Errorf("attribute %q: expected unit %q, got %q", key, expected, *p)
	}
	return p, nil
}
