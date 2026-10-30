// Package machineconfig provides data types for reading and writing machine
// configuration HDF5 files used by additive manufacturing systems.
//
// Struct definitions mirror python/src/machine_config/models.py field-for-field.
// JSON field names use snake_case and match fixtures/reference_output.json, the
// canonical output shared across Python, Rust, Node.js, C++, and Go.
//
// Optional scalar fields are represented as pointers (*string, *float64, *int,
// *bool) so absent HDF5 attributes decode as nil rather than zero values.
// Validation (for example Scanner axis_configuration constraints) belongs in the
// reader, not in these plain data structs.
package machineconfig
