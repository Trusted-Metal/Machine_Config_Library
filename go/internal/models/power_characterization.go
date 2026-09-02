// Package models: shared, version-agnostic conversion between the two
// on-disk shapes this concept has ever used: the flat shape (an opaque
// Algorithm_Type string plus a CSV Params string — File_Version 1.0's
// ClearBox.Volts_To_Watts_* and LightSource.Watts_To_Volts_*) and the
// structured shape (PowerCharacterization — File_Version 1.1's
// Power_Characterization group).
//
// Deliberately placed in internal/models rather than capabilities/v1_0 or
// capabilities/v1_1 — this file only transforms StableModel types
// (PowerCharacterization, EquationConstant, CalibrationPoint), has no HDF5
// I/O, and has no knowledge of "1.0"/"1.1" as version strings. Both
// capabilities/v1_0/hdf5 and capabilities/v1_1/hdf5 already dot-import this
// package, so no new import is needed to reach these functions from either
// writer — and go/version_adapter_isolation_test.go only scans
// capabilities/v1_0, capabilities/v1_1, and internal/mockv1_1, so this file
// (living outside all three) can never be flagged as a cross-version
// reference regardless of what calls it. See V1_1_IMPLEMENTATION_PLAN.md's
// Phase 2 "Structural requirement" note for the full reasoning.
//
// PowerCharacterization is the only StableModel representation of this
// concept, for files of either version — there is no separate
// Volts_To_Watts_*/Watts_To_Volts_* field to fall back to. v1_0/hdf5.Parse
// calls the forward function unconditionally as part of ordinary parsing,
// to populate PowerCharacterization from the on-disk flat attrs;
// v1_0/hdf5.Write calls the backward function unconditionally to derive the
// flat attrs it writes from PowerCharacterization. v1_1/hdf5 reads and
// writes PowerCharacterization natively, with no conversion at all.
//
// (An earlier design — V1_1_IMPLEMENTATION_PLAN.md's Phase 2 — kept both
// representations on the StableModel side by side, with each writer
// deriving from the other shape only as a fallback when its own native
// field was absent, and kept this conversion out of the readers entirely
// because it only worked for models that came from an actual Parse call.
// That asymmetry — silently empty for any consumer that only ever read the
// legacy field — is exactly what motivated collapsing to a single field;
// see POWER_CHARACTERIZATION_UNIFICATION_PLAN.md's "Context" section.)
//
// Two shapes exist today:
//   - Coefficients (Change 3 / ClearBox): Params is a positional,
//     comma-separated coefficient list. Named to match the generated
//     equation's own variable names: b/a for LINEAR, c0/c1/... for
//     POLYNOMIAL, or a plain numeric fallback ("0", "1", ...) for any other
//     value — never an error, see "Never raises" below.
//   - Points (Change 4 / Light_Source): Params is an alternating
//     input_value, output_value list — direct, mechanical parse, no
//     coefficient-ordering convention to assume.
//
// Never raises on an unrecognized Algorithm_Type. Earlier design
// (File_Version 1.1's now-removed MigrateV1ToV1_1) treated this as a hard
// error, which made sense when this conversion only ran inside an explicit,
// opt-in migration call. Once it runs implicitly as part of an ordinary
// write (this file's actual job now), that would mean an unrelated value
// elsewhere on the model could fail a write the caller never expected to
// fail. This is an *incompleteness* problem (less information than the
// richer shape can hold), not a *conflict* (two present, disagreeing
// values) — Change 1's Consolidate check is the latter and correctly keeps
// its hard error in v1_1/hdf5's Write, untouched by this file.
// AlgorithmType is always copied verbatim (open string, per
// docs/migrations/v1_0_to_v1_1.md's resolved enum-vs-string decision),
// coefficients/points are always parsed mechanically, and only
// AlgorithmEquation is left blank when it can't be generated — round trip
// stays lossless even for unrecognized types, since writing back re-sorts
// by the numeric fallback name and reproduces the original CSV order
// exactly. (A genuinely malformed CSV float — not an unrecognized algorithm
// type, actual corrupt data — still returns an error, same as before; that
// failure mode is orthogonal to this rule and unchanged.)
//
// InputType/UnitsDerivedQuantity are always left blank (nil) on forward
// derivation, not hardcoded — confirmed 2026-09-01: a hardcoded guess is a
// special case that would need re-litigating for every future version;
// blank is honest about "not derivable from what's on disk."
package models

import (
	"fmt"
	"sort"
	"strconv"
	"strings"
)

var pcRecognizedAlgorithmTypes = map[string]bool{"LINEAR": true, "POLYNOMIAL": true}

// parsePCCSVFloats parses a comma-separated float list, tolerating the
// bracket-wrapped "[1,2,3]" form real Watts_To_Volts_Params fixtures use as
// well as the plain "1,2,3" form real Volts_To_Watts_Params fixtures use —
// stripping brackets unconditionally is a no-op for the unbracketed case.
func parsePCCSVFloats(s *string) ([]float64, error) {
	if s == nil {
		return nil, nil
	}
	str := strings.TrimSpace(*s)
	str = strings.Trim(str, "[]")
	str = strings.TrimSpace(str)
	if str == "" {
		return nil, nil
	}
	parts := strings.Split(str, ",")
	out := make([]float64, 0, len(parts))
	for _, p := range parts {
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		v, err := strconv.ParseFloat(p, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid float %q in params string", p)
		}
		out = append(out, v)
	}
	return out, nil
}

// generatePCPowerEquation mechanically derives Algorithm_Equation from
// Algorithm_Type + constant count — never stored/derived per-record.
func generatePCPowerEquation(algorithmType string, nConstants int) string {
	if algorithmType == "LINEAR" {
		return "W = a*V + b"
	}
	terms := []string{"c0"}
	for i := 1; i < nConstants; i++ {
		if i == 1 {
			terms = append(terms, "c1*V")
		} else {
			terms = append(terms, fmt.Sprintf("c%d*V^%d", i, i))
		}
	}
	return "W = " + strings.Join(terms, " + ")
}

// ForwardPowerCharacterizationCoefficients implements the coefficients-shape
// forward derivation (Change 3 / ClearBox):
// Volts_To_Watts_Algorithm/Volts_To_Watts_Params -> PowerCharacterization.
//
// Returns (nil, nil) if both inputs are nil (nothing to derive). Never
// returns an error for an unrecognized algorithmType — see package docs.
// Can still return an error for a genuinely malformed CSV float, an
// orthogonal, pre-existing failure mode.
func ForwardPowerCharacterizationCoefficients(algorithmType, params *string) (*PowerCharacterization, error) {
	if algorithmType == nil && params == nil {
		return nil, nil
	}
	at := strOrEmptyPC(algorithmType)
	values, err := parsePCCSVFloats(params)
	if err != nil {
		return nil, err
	}
	var names []string
	switch {
	case at == "LINEAR":
		names = []string{"b", "a"}
	case at == "POLYNOMIAL":
		names = make([]string, len(values))
		for i := range values {
			names[i] = fmt.Sprintf("c%d", i)
		}
	default:
		names = make([]string, len(values))
		for i := range values {
			names[i] = strconv.Itoa(i)
		}
	}
	n := len(names)
	if len(values) < n {
		n = len(values)
	}
	constants := make([]EquationConstant, 0, n)
	for i := 0; i < n; i++ {
		constants = append(constants, EquationConstant{Name: names[i], Value: values[i]})
	}
	var eq *string
	if pcRecognizedAlgorithmTypes[at] {
		eq = StrPtr(generatePCPowerEquation(at, len(values)))
	}
	return &PowerCharacterization{
		AlgorithmType:               StrPtr(at),
		AlgorithmEquation:           eq,
		InputType:                   nil,
		UnitsDerivedQuantity:        nil,
		DerivationEquationConstants: constants,
		CharacterizationPoints:      []CalibrationPoint{},
	}, nil
}

// ForwardPowerCharacterizationPoints implements the points-shape forward
// derivation (Change 4 / Light_Source):
// Watts_To_Volts_Algorithm/Watts_To_Volts_Params -> PowerCharacterization.
//
// Inverse data-availability from the coefficients shape — direct,
// mechanical parse, no coefficient-ordering convention to assume. Returns
// (nil, nil) if both inputs are nil. Never returns an error for an
// unrecognized algorithmType — see package docs.
func ForwardPowerCharacterizationPoints(algorithmType, params *string) (*PowerCharacterization, error) {
	if algorithmType == nil && params == nil {
		return nil, nil
	}
	at := strOrEmptyPC(algorithmType)
	values, err := parsePCCSVFloats(params)
	if err != nil {
		return nil, err
	}
	points := make([]CalibrationPoint, 0, len(values)/2)
	for i := 0; i+1 < len(values); i += 2 {
		points = append(points, CalibrationPoint{InputValue: values[i], OutputValue: values[i+1]})
	}
	// POLYNOMIAL (and the unrecognized-type fallback): left blank, not
	// generated — point count doesn't reliably indicate polynomial degree.
	var eq *string
	if at == "LINEAR" {
		eq = StrPtr("W = a*V + b")
	}
	return &PowerCharacterization{
		AlgorithmType:               StrPtr(at),
		AlgorithmEquation:           eq,
		InputType:                   nil,
		UnitsDerivedQuantity:        nil,
		DerivationEquationConstants: []EquationConstant{},
		CharacterizationPoints:      points,
	}, nil
}

// pcSortedConstantsByName re-sorts Derivation_Equation_Constants rows by
// name before joining as CSV — HDF5 compound dataset rows are named, not
// positionally guaranteed, so on-disk order cannot be trusted. Order: b
// before a (LINEAR); c0 < c1 < c2 < ... by numeric suffix (POLYNOMIAL);
// "0" < "1" < "2" < ... numerically for the unrecognized-algorithm-type
// fallback naming; anything else falls back to alphabetical by name
// (natively-authored v1.1 data can use arbitrary names, since Algorithm_Type
// is an open string).
func pcSortedConstantsByName(constants []EquationConstant) []EquationConstant {
	out := make([]EquationConstant, len(constants))
	copy(out, constants)
	sort.SliceStable(out, func(i, j int) bool {
		gi, ni := pcConstantSortKey(out[i].Name)
		gj, nj := pcConstantSortKey(out[j].Name)
		if gi != gj {
			return gi < gj
		}
		if gi == 3 {
			return out[i].Name < out[j].Name
		}
		return ni < nj
	})
	return out
}

func pcConstantSortKey(name string) (group int, num int) {
	switch name {
	case "b":
		return 0, 0
	case "a":
		return 0, 1
	}
	if len(name) > 1 && name[0] == 'c' {
		if n, err := strconv.Atoi(name[1:]); err == nil {
			return 1, n
		}
	}
	if n, err := strconv.Atoi(name); err == nil {
		return 2, n
	}
	return 3, 0
}

// formatPCFloat renders a float64 the way the on-disk CSV params strings
// expect — always with a decimal point (e.g. "1.0", not "1").
func formatPCFloat(v float64) string {
	s := strconv.FormatFloat(v, 'f', -1, 64)
	if !strings.Contains(s, ".") {
		s += ".0"
	}
	return s
}

// BackwardFlatFieldsCoefficients implements the coefficients-shape backward
// derivation (Change 3 / ClearBox): PowerCharacterization ->
// (Algorithm_Type, Volts_To_Watts_Params).
//
// Returns (nil, nil) if pc is nil. Blank params (a pointer to "", not an
// error) if DerivationEquationConstants is empty — absence, not ambiguity,
// see docs/migrations/v1_0_to_v1_1.md Change 3's "Backward migration" section.
func BackwardFlatFieldsCoefficients(pc *PowerCharacterization) (algorithmType, params *string) {
	if pc == nil {
		return nil, nil
	}
	if len(pc.DerivationEquationConstants) == 0 {
		return pc.AlgorithmType, StrPtr("")
	}
	ordered := pcSortedConstantsByName(pc.DerivationEquationConstants)
	parts := make([]string, len(ordered))
	for i, c := range ordered {
		parts[i] = formatPCFloat(c.Value)
	}
	return pc.AlgorithmType, StrPtr(strings.Join(parts, ","))
}

// BackwardFlatFieldsPoints implements the points-shape backward derivation
// (Change 4 / Light_Source): PowerCharacterization -> (Algorithm_Type,
// Watts_To_Volts_Params).
//
// Returns (nil, nil) if pc is nil. No re-sort needed — CharacterizationPoints
// rows aren't named, and on-disk order is already correct. Blank params if
// empty, same absence-not-ambiguity reasoning as the coefficients shape.
func BackwardFlatFieldsPoints(pc *PowerCharacterization) (algorithmType, params *string) {
	if pc == nil {
		return nil, nil
	}
	if len(pc.CharacterizationPoints) == 0 {
		return pc.AlgorithmType, StrPtr("")
	}
	parts := make([]string, 0, len(pc.CharacterizationPoints)*2)
	for _, p := range pc.CharacterizationPoints {
		parts = append(parts, formatPCFloat(p.InputValue), formatPCFloat(p.OutputValue))
	}
	return pc.AlgorithmType, StrPtr(strings.Join(parts, ","))
}

func strOrEmptyPC(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}
