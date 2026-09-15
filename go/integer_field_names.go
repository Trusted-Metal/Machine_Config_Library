package machineconfig

// Derives, via reflection, the set of JSON leaf key names that
// models.MachineConfig's own struct definitions declare as an integer kind
// (int/int8/.../uint64, or a pointer to one) anywhere in its type graph —
// as opposed to a float64 (or *float64) leaf, which is everything else
// numeric in this schema. See hash.go's module docs for why this is
// reflection-derived rather than a hardcoded list.

import (
	"reflect"
	"strings"

	"machine-config-go/internal/models"
)

var integerFieldNameSet = deriveIntegerFieldNames()

// IntegerFieldNames returns the set of JSON leaf key names classified as
// integer-typed for configuration-hash rendering purposes. Exported for
// tests (and for diagnosing a cross-language hash mismatch); computed once
// at package init.
func IntegerFieldNames() map[string]struct{} {
	return integerFieldNameSet
}

func deriveIntegerFieldNames() map[string]struct{} {
	out := map[string]struct{}{}
	collectIntegerFieldNames(reflect.TypeOf(models.MachineConfig{}), map[reflect.Type]bool{}, out)
	return out
}

// stripContainers unwraps pointer/slice/array/map layers down to the
// underlying element type (e.g. *[]*float64 -> float64). A []byte-shaped
// slice is left as-is (not unwrapped to uint8) — encoding/json always
// marshals []byte as a base64 string, never as a numeric array, so treating
// its element kind as "integer" would misclassify a binary field like
// ScanFieldCorrectionFile.RawBytes.
func stripContainers(t reflect.Type) reflect.Type {
	for {
		switch t.Kind() {
		case reflect.Ptr, reflect.Array, reflect.Map:
			t = t.Elem()
		case reflect.Slice:
			if t.Elem().Kind() == reflect.Uint8 {
				return t
			}
			t = t.Elem()
		default:
			return t
		}
	}
}

func collectIntegerFieldNames(t reflect.Type, seen map[reflect.Type]bool, out map[string]struct{}) {
	t = stripContainers(t)
	if t.Kind() != reflect.Struct || seen[t] {
		return
	}
	seen[t] = true
	for i := 0; i < t.NumField(); i++ {
		f := t.Field(i)
		if f.PkgPath != "" {
			continue // unexported
		}
		tag := f.Tag.Get("json")
		if tag == "-" {
			continue
		}
		name, _, _ := strings.Cut(tag, ",")
		if name == "" {
			name = f.Name
		}
		leaf := stripContainers(f.Type)
		switch leaf.Kind() {
		case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64,
			reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
			out[name] = struct{}{}
		case reflect.Struct:
			collectIntegerFieldNames(leaf, seen, out)
		}
	}
}
