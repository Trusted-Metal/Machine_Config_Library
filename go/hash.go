package machineconfig

// SHA-256 configuration-integrity hash for MachineConfig.
//
// Computed over the canonical, include-binary=false JSON shape of the whole
// config (the same shape export-json produces), minus
// meta.configuration_hash (can't hash itself) and meta.export_date (changes
// on every re-export even when nothing configuration-wise changed). Object
// keys are recursively sorted; arrays keep their existing element order.
//
// This hash is computed for MCF's own self-consistency across MCF's own
// five language implementations — it is NOT designed to match any other
// producer's own hashing scheme. A file MCF's own writer produced will read
// back valid; a file authored by anything else will very likely read back
// invalid — that's expected, not a bug: it answers "was this MCF-touched
// file tampered with since MCF itself last touched it," not "does this
// match some other system's proprietary algorithm."
//
// Binary correction-grid data (CorrectionData/InverseCorrectionData/
// RawBytes) is always excluded regardless of whether the *MachineConfig
// passed in happens to carry it (e.g. a config obtained via
// ParseWithOptions{IncludeBinary: true}) — a separate, dedicated mechanism
// (correction-hash) already exists for binary-grid integrity.
//
// # Float-vs-integer rendering
//
// Go's encoding/json renders a whole-valued float64 the same way it renders
// an int (250.0 and 250 both marshal to "250") — confirmed empirically, not
// assumed; this is the opposite of what a first guess based on Rust's
// serde_json might suggest (serde_json already forces "250.0", see hash.rs).
// So Go needs the same kind of float-rendering fixup Node.js needs.
//
// Unlike Node.js, though, this model's Go struct fields are already
// concretely typed as int/*int vs float64/*float64 — the distinction Node
// can't recover at all is fully present here, just not preserved by a
// naive json.Marshal → json.Unmarshal round trip into a generic map (the
// first Marshal already collapses "250.0" to "250" as plain JSON text,
// before any Unmarshal happens). Rather than hand-copy a leaf-key-name
// allowlist the way hash.ts does (and then needing a separate test to keep
// that copy from drifting out of sync with something else), this package
// derives the set of integer-typed leaf key names once, via reflection over
// models.MachineConfig itself (see integerFieldNames.go) — the model's own
// struct tags and field types are the single source of truth, so there is
// no second copy that can drift.
//
// This does not cover dynamically-named passthrough Extra attributes
// (map[string]any — no fixed key name to classify, and by the time a value
// reaches that map its concrete Go type is already just whatever
// encoding/json's own decoder produced) — a pre-existing limitation shared
// with every other language's hash, not newly introduced here.

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"sort"
	"strconv"
	"strings"

	"machine-config-go/internal/models"
)

// ComputeConfigurationHash returns the SHA-256 hex digest of cfg's
// canonical, hash-relevant content.
func ComputeConfigurationHash(cfg *MachineConfig) (string, error) {
	canonical, err := CanonicalizeForHash(cfg)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(canonical)
	return fmt.Sprintf("%x", sum), nil
}

// CanonicalizeForHash returns the exact canonical JSON bytes that
// ComputeConfigurationHash hashes — exposed (not just an internal detail)
// so this can be diffed directly against the other languages' equivalents
// when tracking down a cross-language hash mismatch.
func CanonicalizeForHash(cfg *MachineConfig) ([]byte, error) {
	sanitized := sanitizeForHash(cfg)

	// json.Marshal on the typed struct applies every existing omitempty /
	// json:"-" / field-renaming rule automatically and correctly (including
	// Scanner's four Invert_* flags, which already omit themselves via
	// `json:"invert_actual_x,omitempty"` directly on the struct — no
	// separate replacer step needed here, unlike Node.js's toJson()).
	raw, err := json.Marshal(sanitized)
	if err != nil {
		return nil, err
	}

	var generic map[string]any
	if err := json.Unmarshal(raw, &generic); err != nil {
		return nil, err
	}
	if meta, ok := generic["meta"].(map[string]any); ok {
		delete(meta, "configuration_hash")
		delete(meta, "export_date")
	}

	var buf bytes.Buffer
	renderValue(&buf, generic, "")
	return buf.Bytes(), nil
}

// sanitizeForHash returns a copy of cfg with the binary correction-grid
// fields removed from every optical train, regardless of whether they were
// populated — mirrors Python's _config_to_dict(config, include_binary=False),
// which always builds the non-binary shape irrespective of what the input
// carries. Does not mutate cfg.
func sanitizeForHash(cfg *MachineConfig) *MachineConfig {
	sanitized := *cfg
	trains := make([]models.OpticalTrain, len(cfg.OpticalTrains))
	for i, train := range cfg.OpticalTrains {
		trains[i] = train
		if train.OptionalComponents.Clearbox != nil {
			cb := *train.OptionalComponents.Clearbox
			cb.CorrectionData = nil
			cb.InverseCorrectionData = nil
			trains[i].OptionalComponents.Clearbox = &cb
		}
		if train.ScanFieldCorrectionFile != nil {
			sfcf := *train.ScanFieldCorrectionFile
			sfcf.RawBytes = nil
			trains[i].ScanFieldCorrectionFile = &sfcf
		}
	}
	sanitized.OpticalTrains = trains
	return &sanitized
}

// renderValue writes v's canonical compact JSON encoding to buf: object
// keys sorted recursively, arrays left in their existing order, numbers
// rendered via renderNumber. key is the enclosing object key v was read
// from (used only to classify a numeric v); pass "" when there is none.
func renderValue(buf *bytes.Buffer, v any, key string) {
	switch val := v.(type) {
	case nil:
		buf.WriteString("null")
	case bool:
		if val {
			buf.WriteString("true")
		} else {
			buf.WriteString("false")
		}
	case string:
		renderJSONString(buf, val)
	case float64:
		renderNumber(buf, val, key)
	case []any:
		buf.WriteByte('[')
		for i, item := range val {
			if i > 0 {
				buf.WriteByte(',')
			}
			renderValue(buf, item, "")
		}
		buf.WriteByte(']')
	case map[string]any:
		keys := make([]string, 0, len(val))
		for k := range val {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		buf.WriteByte('{')
		for i, k := range keys {
			if i > 0 {
				buf.WriteByte(',')
			}
			renderJSONString(buf, k)
			buf.WriteByte(':')
			renderValue(buf, val[k], k)
		}
		buf.WriteByte('}')
	default:
		// json.Unmarshal into `any` only ever produces the types above.
		panic(fmt.Sprintf("unreachable: unexpected type %T in configuration hash", v))
	}
}

func renderJSONString(buf *bytes.Buffer, s string) {
	// Reuses encoding/json's own string-escaping (quotes, unicode, control
	// chars) rather than reimplementing it.
	b, _ := json.Marshal(s)
	buf.Write(b)
}

// renderNumber renders n as a JSON number token: a plain integer literal
// when key is a known integer-typed field (see integerFieldNames.go),
// otherwise a float literal with a forced trailing ".0" when n is whole —
// matching Python/Rust/C++'s native int/float distinction, which Go's own
// float64 formatter (like JS's) does not preserve by itself.
func renderNumber(buf *bytes.Buffer, n float64, key string) {
	if _, isInt := integerFieldNameSet[key]; isInt {
		buf.WriteString(strconv.FormatFloat(n, 'f', 0, 64))
		return
	}
	s := strconv.FormatFloat(n, 'g', -1, 64)
	if !strings.ContainsAny(s, ".eE") {
		s += ".0"
	}
	buf.WriteString(s)
}
