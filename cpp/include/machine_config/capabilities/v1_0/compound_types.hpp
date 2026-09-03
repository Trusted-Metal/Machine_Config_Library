#pragma once
// On-disk HDF5 compound-dataset row types for SynchronousSensor's two
// datasets — shared by hdf5.hpp (reader) and writer.hpp (writer). First use
// of HighFive::CompoundType anywhere in this project.
//
// Distinct from the model-facing EquationConstant/CalibrationPoint in
// models.hpp (which use std::string and have no HDF5-specific dependency)
// because HighFive's raw-struct read/write path needs a fixed C-layout type,
// not std::string — the same split every other language's adapter layer
// already uses (e.g. Rust's RawEquationConstant vs. EquationConstant).
//
// EquationConstantRow.name is a 64-byte fixed-length UTF-8 string
// (NULLPAD-padded on disk), not a variable-length string — a deliberate,
// cross-language decision, not a C++-specific shortcut: the HDF5 C library
// cannot convert between fixed-length and variable-length strings when
// they're compound-type *members* (confirmed at the raw H5Tinsert/H5Dread
// level during the original cross-language investigation), and Node.js's
// h5wasm cannot write a non-empty VLEN string in that position at all. A
// 64-byte fixed-length UTF-8 string is the one representation every
// language's HDF5 binding can both read and write here — see
// SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string convention" for the
// full investigation. Rust's FixedUnicode<64>, Python's
// h5py.string_dtype(encoding="utf-8", length=64), Node's explicit "S64"
// dtype, and Go's h5c_create_equation_constant_type must all agree exactly
// (same width, NULLPAD, UTF-8) for cross-language read/write to succeed —
// confirmed directly against the real committed fixture before relying on
// it, not assumed.
//
// NOTE: HIGHFIVE_REGISTER_TYPE expands to an explicit specialization of
// HighFive::create_datatype<T> using HighFive's fully-qualified name, which
// must be declared in a namespace enclosing HighFive's own — so these types
// and their registrations live at global scope, not inside
// `namespace machine_config`, matching HighFive's own compound_types.cpp
// example. EquationConstantMaxNameBytes is likewise global for the same
// reason it needs to be visible to the macro-generated factory function.

#include <highfive/H5File.hpp>

#include <cstddef>

// Fixed on-disk width, in UTF-8 bytes, of an EquationConstantRow.name. Matches
// every other language's convention exactly (see SYNCHRONOUS_SENSOR_PLAN.md's
// "Compound dataset string convention").
inline constexpr std::size_t EquationConstantMaxNameBytes = 64;

struct EquationConstantRow {
    char name[EquationConstantMaxNameBytes];
    double value;
};

inline HighFive::CompoundType create_compound_EquationConstantRow() {
    return {{"name", HighFive::FixedLengthStringType(
                          EquationConstantMaxNameBytes,
                          HighFive::StringPadding::NullPadded,
                          HighFive::CharacterSet::Utf8)},
            {"value", HighFive::create_datatype<double>()}};
}
HIGHFIVE_REGISTER_TYPE(EquationConstantRow, create_compound_EquationConstantRow)

struct CalibrationPointRow {
    double input_value;
    double output_value;
};

inline HighFive::CompoundType create_compound_CalibrationPointRow() {
    return {{"input_value", HighFive::create_datatype<double>()},
            {"output_value", HighFive::create_datatype<double>()}};
}
HIGHFIVE_REGISTER_TYPE(CalibrationPointRow, create_compound_CalibrationPointRow)
