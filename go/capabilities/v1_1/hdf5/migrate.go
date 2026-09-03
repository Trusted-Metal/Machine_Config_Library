// Phase 2 (V1_1_IMPLEMENTATION_PLAN.md, clean-up phase) removed
// MigrateV1ToV1_1/MigrateV1_1ToV1 from here — the coefficients/points
// shape-conversion functions they used now live in
// machine-config-go/internal/models (ForwardPowerCharacterizationCoefficients/
// Points, BackwardFlatFieldsCoefficients/Points). Per the
// POWER_CHARACTERIZATION_UNIFICATION_PLAN.md follow-on, PowerCharacterization
// is now the only StableModel representation of this concept: this
// package's Write always writes PowerCharacterization directly (no fallback
// derivation left to do), and capabilities/v1_0/hdf5's reader/writer always
// forward/backward-derive it unconditionally as part of ordinary parsing.
// See internal/models/power_characterization.go for the full design.
package hdf5
