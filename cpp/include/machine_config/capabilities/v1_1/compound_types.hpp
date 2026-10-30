#pragma once
// On-disk HDF5 compound-dataset row types for File_Version 1.1's
// Power_Characterization datasets (Derivation_Equation_Constants /
// Characterization_Points), plus SynchronousSensor's own pair of datasets,
// which share the identical on-disk shape.
//
// Independent copies of the equivalent types from any other version's
// compound-type header (see the isolation rule enforced by
// cpp/tests/test_version_adapter_isolation.cpp): even though this shape is
// identical to what an earlier File_Version already uses, this file must
// not include or reference that header. Distinctly named (V1_1 prefix) to
// avoid a global-scope redefinition if some future translation unit ends up
// including more than one version's compound-type header at once —
// HIGHFIVE_REGISTER_TYPE expands to a global-scope template specialization
// of HighFive::create_datatype<T>, so two same-named structs at global scope
// would collide.
//
// name is a 64-byte fixed-length, NULLPAD, UTF-8 string — not a
// variable-length string — because HDF5 cannot convert between fixed- and
// variable-length strings when they are compound-type *members*, and this
// exact width/padding/charset must match what every other language's HDF5
// binding writes for the same dataset shape.

#include <highfive/H5File.hpp>

#include <cstddef>

inline constexpr std::size_t V1_1EquationConstantMaxNameBytes = 64;

struct V1_1EquationConstantRow {
    char name[V1_1EquationConstantMaxNameBytes];
    double value;
};

inline HighFive::CompoundType create_compound_V1_1EquationConstantRow() {
    return {{"name", HighFive::FixedLengthStringType(
                          V1_1EquationConstantMaxNameBytes,
                          HighFive::StringPadding::NullPadded,
                          HighFive::CharacterSet::Utf8)},
            {"value", HighFive::create_datatype<double>()}};
}
HIGHFIVE_REGISTER_TYPE(V1_1EquationConstantRow, create_compound_V1_1EquationConstantRow)

struct V1_1CalibrationPointRow {
    double input_value;
    double output_value;
};

inline HighFive::CompoundType create_compound_V1_1CalibrationPointRow() {
    return {{"input_value", HighFive::create_datatype<double>()},
            {"output_value", HighFive::create_datatype<double>()}};
}
HIGHFIVE_REGISTER_TYPE(V1_1CalibrationPointRow, create_compound_V1_1CalibrationPointRow)
