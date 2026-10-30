#pragma once
// Extension contract: the opaque, language-neutral tree core uses to carry data that belongs to an
// extension (a group under Extensions/<name>/), so a separate library can pack and unpack it
// without core knowing its schema.
//
// Dependency-free on purpose (no HDF5): extension libraries include only this header. Core reads
// and writes every tree verbatim, so unknown or private extensions round-trip untouched.
//
// Pre-stable (0.x) until a second extension has been ported against it. Every language produces
// the identical canonical digest and JSON form; see docs/EXTENSION_CONTRACT.md.

#include "machine_config/sha256.hpp"

#include <nlohmann/json.hpp>

#include <cmath>
#include <cstdint>
#include <cstring>
#include <limits>
#include <map>
#include <set>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

namespace machine_config::extension {

inline constexpr const char* CONTRACT_VERSION = "0.1.1";

// Fixed-width storage types; each maps 1:1 to an HDF5 type. Value shapes by dtype:
// utf8 -> string / vector<string>, i8..i64 -> int64_t / vector<int64_t>,
// u8..u64 -> uint64_t / vector<uint64_t>, f32/f64 -> double / vector<double>.

/// `path` is the slash-joined location of the offending node; attributes are prefixed '@'.
class ContractError : public std::runtime_error {
public:
    ContractError(std::string path, const std::string& message)
        : std::runtime_error((path.empty() ? std::string("<root>") : path) + ": " + message),
          path_(std::move(path)), message_(message) {}
    const std::string& path() const { return path_; }
    const std::string& message() const { return message_; }

private:
    std::string path_;
    std::string message_;
};

using AttrValue = std::variant<std::string, std::int64_t, std::uint64_t, double,
                               std::vector<std::string>, std::vector<std::int64_t>,
                               std::vector<std::uint64_t>, std::vector<double>>;
using DatasetData =
    std::variant<std::vector<std::int64_t>, std::vector<std::uint64_t>, std::vector<double>>;

/// A group or dataset attribute: a scalar or a 1-D vector of one dtype.
struct Attr {
    std::string dtype;
    AttrValue value;
};

/// N-d numeric array stored row-major in `data`; its length equals the product of `shape`
/// (an empty shape is a scalar).
struct Dataset {
    std::string dtype;
    std::vector<std::uint64_t> shape;
    DatasetData data;
    std::map<std::string, Attr> attrs;
};

/// One column of a Table. `size` is the fixed byte length of a utf8 column and 0 for numeric ones.
struct TableField {
    std::string name;
    std::string dtype;
    std::uint64_t size = 0;
};

/// The values of one table column, in the dtype's C++ shape.
using ColumnData = std::variant<std::vector<std::string>, std::vector<std::int64_t>,
                                std::vector<std::uint64_t>, std::vector<double>>;

/// 1-D compound dataset (HDF5 compound type): one row per record, one named column per field.
/// `fields` order is the compound member order and is significant; `columns[name]` holds `rows`
/// values.
struct Table {
    std::vector<TableField> fields;
    std::uint64_t rows = 0;
    std::map<std::string, ColumnData> columns;
    std::map<std::string, Attr> attrs;
};

/// One group. Order is not significant; the canonical form sorts by UTF-8 bytes. `tables` is empty
/// unless used, so a tree without compound data is encoded and hashed exactly as in contract 0.1.0.
struct Tree {
    std::map<std::string, Attr> attrs;
    std::map<std::string, Tree> children;
    std::map<std::string, Dataset> datasets;
    std::map<std::string, Table> tables;
};

namespace detail {

inline const std::map<std::string, std::uint8_t>& dtypeTags() {
    static const std::map<std::string, std::uint8_t> tags = {
        {"utf8", 1}, {"i8", 2},  {"u8", 3},  {"i16", 4}, {"u16", 5}, {"i32", 6},
        {"u32", 7},  {"i64", 8}, {"u64", 9}, {"f32", 10}, {"f64", 11}};
    return tags;
}

inline bool isSigned(const std::string& d) { return d == "i8" || d == "i16" || d == "i32" || d == "i64"; }
inline bool isUnsigned(const std::string& d) { return d == "u8" || d == "u16" || d == "u32" || d == "u64"; }
inline bool isFloat(const std::string& d) { return d == "f32" || d == "f64"; }

inline std::string joinPath(const std::string& path, const std::string& name) {
    return path.empty() ? name : path + "/" + name;
}

inline void checkSigned(const std::string& dtype, std::int64_t v, const std::string& path) {
    std::int64_t lo = 0, hi = 0;
    if (dtype == "i8") { lo = INT8_MIN; hi = INT8_MAX; }
    else if (dtype == "i16") { lo = INT16_MIN; hi = INT16_MAX; }
    else if (dtype == "i32") { lo = INT32_MIN; hi = INT32_MAX; }
    else { return; }
    if (v < lo || v > hi)
        throw ContractError(path, dtype + " value " + std::to_string(v) + " is out of range");
}

inline void checkUnsigned(const std::string& dtype, std::uint64_t v, const std::string& path) {
    std::uint64_t hi = 0;
    if (dtype == "u8") hi = UINT8_MAX;
    else if (dtype == "u16") hi = UINT16_MAX;
    else if (dtype == "u32") hi = UINT32_MAX;
    else { return; }
    if (v > hi)
        throw ContractError(path, dtype + " value " + std::to_string(v) + " is out of range");
}

inline void checkNoNul(const std::string& s, const std::string& path) {
    if (s.find('\0') != std::string::npos)
        throw ContractError(path, "utf8 value must not contain NUL");
}

/// Validates one attribute/dataset value against its dtype.
inline void checkValue(const std::string& dtype, const AttrValue& value, const std::string& path) {
    auto wrongType = [&]() {
        throw ContractError(path, dtype + " value has the wrong type for its dtype");
    };
    if (dtype == "utf8") {
        if (auto* s = std::get_if<std::string>(&value)) checkNoNul(*s, path);
        else if (auto* v = std::get_if<std::vector<std::string>>(&value)) {
            for (std::size_t i = 0; i < v->size(); ++i)
                checkNoNul((*v)[i], path + "[" + std::to_string(i) + "]");
        } else wrongType();
    } else if (isSigned(dtype)) {
        if (auto* n = std::get_if<std::int64_t>(&value)) checkSigned(dtype, *n, path);
        else if (auto* v = std::get_if<std::vector<std::int64_t>>(&value)) {
            for (std::size_t i = 0; i < v->size(); ++i)
                checkSigned(dtype, (*v)[i], path + "[" + std::to_string(i) + "]");
        } else wrongType();
    } else if (isUnsigned(dtype)) {
        if (auto* n = std::get_if<std::uint64_t>(&value)) checkUnsigned(dtype, *n, path);
        else if (auto* v = std::get_if<std::vector<std::uint64_t>>(&value)) {
            for (std::size_t i = 0; i < v->size(); ++i)
                checkUnsigned(dtype, (*v)[i], path + "[" + std::to_string(i) + "]");
        } else wrongType();
    } else {
        if (!std::holds_alternative<double>(value) &&
            !std::holds_alternative<std::vector<double>>(value))
            wrongType();
    }
}

inline void validateNodeName(const std::string& name, const std::string& path) {
    if (name.empty()) throw ContractError(path, "name must not be empty");
    if (name.find('/') != std::string::npos)
        throw ContractError(path, "name \"" + name + "\" must not contain \"/\"");
    if (name.find('\0') != std::string::npos) throw ContractError(path, "name must not contain NUL");
}

inline void validateAttrs(const std::map<std::string, Attr>& attrs, const std::string& path) {
    for (const auto& [name, a] : attrs) {
        std::string attrPath = joinPath(path, "@" + name);
        validateNodeName(name, attrPath);
        if (dtypeTags().find(a.dtype) == dtypeTags().end())
            throw ContractError(attrPath, "unknown dtype \"" + a.dtype + "\"");
        checkValue(a.dtype, a.value, attrPath);
    }
}

inline std::size_t dataLen(const DatasetData& d) {
    return std::visit([](const auto& v) { return v.size(); }, d);
}

inline std::string shapeText(const std::vector<std::uint64_t>& shape) {
    std::string s = "[";
    for (std::size_t i = 0; i < shape.size(); ++i) s += (i ? "," : "") + std::to_string(shape[i]);
    return s + "]";
}

inline std::size_t columnLen(const ColumnData& c) {
    return std::visit([](const auto& v) { return v.size(); }, c);
}

inline void validateTable(const Table& table, const std::string& path) {
    if (table.fields.empty()) throw ContractError(path, "a table needs at least one field");
    std::set<std::string> names;
    for (const TableField& f : table.fields) {
        std::string fieldPath = joinPath(path, f.name);
        validateNodeName(f.name, fieldPath);
        if (!names.insert(f.name).second) throw ContractError(fieldPath, "duplicate field name");
        if (dtypeTags().find(f.dtype) == dtypeTags().end())
            throw ContractError(fieldPath, "unknown dtype \"" + f.dtype + "\"");
        auto it = table.columns.find(f.name);
        if (it == table.columns.end()) throw ContractError(fieldPath, "missing column");
        const ColumnData& column = it->second;
        if (f.dtype == "utf8") {
            if (f.size < 1) throw ContractError(fieldPath, "a utf8 field needs a positive integer size");
        } else if (f.size != 0) {
            throw ContractError(fieldPath, "size is only valid for utf8 fields");
        }
        if (columnLen(column) != table.rows)
            throw ContractError(fieldPath, "column length " + std::to_string(columnLen(column)) +
                                              " does not match " + std::to_string(table.rows) + " rows");
        const bool typeOk = (f.dtype == "utf8" && std::holds_alternative<std::vector<std::string>>(column)) ||
                            (isSigned(f.dtype) && std::holds_alternative<std::vector<std::int64_t>>(column)) ||
                            (isUnsigned(f.dtype) && std::holds_alternative<std::vector<std::uint64_t>>(column)) ||
                            (isFloat(f.dtype) && std::holds_alternative<std::vector<double>>(column));
        if (!typeOk)
            throw ContractError(fieldPath, "column element type does not match the field dtype " + f.dtype);
        std::visit(
            [&](const auto& v) {
                using V = std::decay_t<decltype(v)>;
                checkValue(f.dtype, AttrValue{V(v)}, fieldPath);
            },
            column);
        if (const auto* strs = std::get_if<std::vector<std::string>>(&column)) {
            for (std::size_t i = 0; i < strs->size(); ++i)
                if ((*strs)[i].size() > f.size)
                    throw ContractError(fieldPath + "[" + std::to_string(i) + "]",
                                        std::to_string((*strs)[i].size()) + " UTF-8 bytes do not fit the " +
                                            std::to_string(f.size) + "-byte field");
        }
    }
    for (const auto& entry : table.columns)
        if (!names.count(entry.first)) throw ContractError(joinPath(path, entry.first), "column has no matching field");
    validateAttrs(table.attrs, path);
}

}  // namespace detail

/// Throws ContractError naming the first invalid node. `path` prefixes every reported location
/// (use the extension's name, or "" for none).
inline void validate(const Tree& t, const std::string& path = "") {
    detail::validateAttrs(t.attrs, path);
    for (const auto& [name, child] : t.children) {
        std::string childPath = detail::joinPath(path, name);
        detail::validateNodeName(name, childPath);
        validate(child, childPath);
    }
    for (const auto& [name, ds] : t.datasets) {
        std::string dsPath = detail::joinPath(path, name);
        detail::validateNodeName(name, dsPath);
        if (t.children.count(name))
            throw ContractError(dsPath, "a dataset and a child group cannot share a name");
        if (detail::dtypeTags().find(ds.dtype) == detail::dtypeTags().end() || ds.dtype == "utf8")
            throw ContractError(dsPath, "dataset dtype must be numeric, got \"" + ds.dtype + "\"");
        std::uint64_t expected = 1;
        for (std::uint64_t d : ds.shape) {
            if (d != 0 && expected > std::numeric_limits<std::uint32_t>::max() / d)
                throw ContractError(dsPath, "shape " + detail::shapeText(ds.shape) + " is too large");
            expected *= d;
        }
        if (detail::dataLen(ds.data) != expected)
            throw ContractError(dsPath, "data length " + std::to_string(detail::dataLen(ds.data)) +
                                            " does not match shape " + detail::shapeText(ds.shape));
        const bool typeOk = (detail::isSigned(ds.dtype) && std::holds_alternative<std::vector<std::int64_t>>(ds.data)) ||
                            (detail::isUnsigned(ds.dtype) && std::holds_alternative<std::vector<std::uint64_t>>(ds.data)) ||
                            (detail::isFloat(ds.dtype) && std::holds_alternative<std::vector<double>>(ds.data));
        if (!typeOk) throw ContractError(dsPath, ds.dtype + " data has the wrong element type");
        std::visit(
            [&](const auto& v) {
                using V = std::decay_t<decltype(v)>;
                detail::checkValue(ds.dtype, AttrValue{V(v)}, dsPath);
            },
            ds.data);
        detail::validateAttrs(ds.attrs, dsPath);
    }
    for (const auto& [name, table] : t.tables) {
        std::string tablePath = detail::joinPath(path, name);
        detail::validateNodeName(name, tablePath);
        if (t.children.count(name) || t.datasets.count(name))
            throw ContractError(tablePath, "a table cannot share a name with a dataset or child group");
        detail::validateTable(table, tablePath);
    }
}

// ---------------------------------------------------------------------------------------------
// Canonical digest
//
// SHA-256 over a canonical binary encoding, identical in every language (all little-endian):
//
//   tree    = u32 attrCount, attr*, u32 childCount, (name tree)*, u32 dsCount, (name dataset)*
//   name    = u32 byteLength, UTF-8 bytes
//   attr    = name, u8 dtypeTag, u8 kind (0 scalar, 1 array), [u32 count], value*
//   dataset = u8 dtypeTag, u32 rank, u64 dim*, u64 count, value*, attr map (as in tree)
//   value   = utf8: u32 byteLength + bytes; ints: fixed width; f32/f64: IEEE bits
//
// Appended after the datasets only when a tree has tables (so trees without them hash as in 0.1.0):
//
//   tables  = u32 0xFFFFFFFF, u32 tableCount, (name table)*
//   table   = u32 fieldCount, (name, u8 dtypeTag, u32 size)*, u64 rows, column values*, attr map
//
// Table fields keep their declared order (it is the compound member order); each column's values
// follow in that order, row by row within a column.
//
// Entries are sorted by the UTF-8 bytes of their names (std::map<std::string> already compares
// bytewise). NaN is the quiet NaN and -0 is +0.
// ---------------------------------------------------------------------------------------------

namespace detail {

inline void putLE(std::string& out, std::uint64_t v, int bytes) {
    for (int i = 0; i < bytes; ++i) out.push_back(static_cast<char>((v >> (8 * i)) & 0xFF));
}
inline void putName(std::string& out, const std::string& s) {
    putLE(out, s.size(), 4);
    out += s;
}

inline void putElement(std::string& out, const std::string& dtype, const std::string& v) {
    (void)dtype;
    putName(out, v);
}
inline void putElement(std::string& out, const std::string& dtype, std::int64_t v) {
    int width = dtype == "i8" ? 1 : dtype == "i16" ? 2 : dtype == "i32" ? 4 : 8;
    putLE(out, static_cast<std::uint64_t>(v), width);
}
inline void putElement(std::string& out, const std::string& dtype, std::uint64_t v) {
    int width = dtype == "u8" ? 1 : dtype == "u16" ? 2 : dtype == "u32" ? 4 : 8;
    putLE(out, v, width);
}
inline void putElement(std::string& out, const std::string& dtype, double v) {
    if (dtype == "f32") {
        if (std::isnan(v)) { putLE(out, 0x7FC00000u, 4); return; }
        float f = static_cast<float>(v);
        if (f == 0.0f) f = 0.0f;
        std::uint32_t bits;
        std::memcpy(&bits, &f, 4);
        putLE(out, bits, 4);
    } else {
        if (std::isnan(v)) { putLE(out, 0x7FF8000000000000ull, 8); return; }
        if (v == 0.0) v = 0.0;
        std::uint64_t bits;
        std::memcpy(&bits, &v, 8);
        putLE(out, bits, 8);
    }
}

inline void putAttrs(std::string& out, const std::map<std::string, Attr>& attrs) {
    putLE(out, attrs.size(), 4);
    for (const auto& [name, a] : attrs) {
        putName(out, name);
        out.push_back(static_cast<char>(dtypeTags().at(a.dtype)));
        std::visit(
            [&](const auto& v) {
                using V = std::decay_t<decltype(v)>;
                if constexpr (std::is_same_v<V, std::vector<std::string>> ||
                              std::is_same_v<V, std::vector<std::int64_t>> ||
                              std::is_same_v<V, std::vector<std::uint64_t>> ||
                              std::is_same_v<V, std::vector<double>>) {
                    out.push_back(1);
                    putLE(out, v.size(), 4);
                    for (const auto& e : v) putElement(out, a.dtype, e);
                } else {
                    out.push_back(0);
                    putElement(out, a.dtype, v);
                }
            },
            a.value);
    }
}

inline void putTree(std::string& out, const Tree& t) {
    putAttrs(out, t.attrs);
    putLE(out, t.children.size(), 4);
    for (const auto& [name, child] : t.children) {
        putName(out, name);
        putTree(out, child);
    }
    putLE(out, t.datasets.size(), 4);
    for (const auto& [name, ds] : t.datasets) {
        putName(out, name);
        out.push_back(static_cast<char>(dtypeTags().at(ds.dtype)));
        putLE(out, ds.shape.size(), 4);
        for (std::uint64_t d : ds.shape) putLE(out, d, 8);
        putLE(out, dataLen(ds.data), 8);
        std::visit([&](const auto& v) { for (const auto& e : v) putElement(out, ds.dtype, e); }, ds.data);
        putAttrs(out, ds.attrs);
    }
    if (!t.tables.empty()) {
        putLE(out, 0xFFFFFFFFu, 4);
        putLE(out, t.tables.size(), 4);
        for (const auto& [name, table] : t.tables) {
            putName(out, name);
            putLE(out, table.fields.size(), 4);
            for (const TableField& f : table.fields) {
                putName(out, f.name);
                out.push_back(static_cast<char>(dtypeTags().at(f.dtype)));
                putLE(out, f.size, 4);
            }
            putLE(out, table.rows, 8);
            for (const TableField& f : table.fields)
                std::visit([&](const auto& v) { for (const auto& e : v) putElement(out, f.dtype, e); },
                           table.columns.at(f.name));
            putAttrs(out, table.attrs);
        }
    }
}

}  // namespace detail

/// Hex SHA-256 of the tree's canonical encoding. Throws ContractError if the tree violates the
/// contract.
inline std::string canonicalDigest(const Tree& tree) {
    validate(tree);
    std::string buf;
    detail::putTree(buf, tree);
    return machine_config::detail::Sha256::hexDigest(buf);
}

// ---------------------------------------------------------------------------------------------
// JSON form (used by fixtures and exports): i64/u64 as decimal strings, non-finite floats as
// "NaN" / "Infinity" / "-Infinity", so the JSON is lossless in every language.
// ---------------------------------------------------------------------------------------------

namespace detail {

inline nlohmann::json scalarToJson(const std::string& dtype, std::int64_t v) {
    return dtype == "i64" ? nlohmann::json(std::to_string(v)) : nlohmann::json(v);
}
inline nlohmann::json scalarToJson(const std::string& dtype, std::uint64_t v) {
    return dtype == "u64" ? nlohmann::json(std::to_string(v)) : nlohmann::json(v);
}
inline nlohmann::json scalarToJson(const std::string&, double v) {
    if (std::isnan(v)) return "NaN";
    if (std::isinf(v)) return v > 0 ? "Infinity" : "-Infinity";
    return v;
}
inline nlohmann::json scalarToJson(const std::string&, const std::string& v) { return v; }

template <typename T>
inline nlohmann::json vectorToJson(const std::string& dtype, const std::vector<T>& v) {
    nlohmann::json arr = nlohmann::json::array();
    for (const auto& e : v) arr.push_back(scalarToJson(dtype, e));
    return arr;
}

inline nlohmann::json attrsToJson(const std::map<std::string, Attr>& attrs) {
    nlohmann::json out = nlohmann::json::object();
    for (const auto& [name, a] : attrs) {
        nlohmann::json value = std::visit(
            [&](const auto& v) -> nlohmann::json {
                using V = std::decay_t<decltype(v)>;
                if constexpr (std::is_same_v<V, std::vector<std::string>> ||
                              std::is_same_v<V, std::vector<std::int64_t>> ||
                              std::is_same_v<V, std::vector<std::uint64_t>> ||
                              std::is_same_v<V, std::vector<double>>)
                    return vectorToJson(a.dtype, v);
                else
                    return scalarToJson(a.dtype, v);
            },
            a.value);
        out[name] = {{"dtype", a.dtype}, {"value", std::move(value)}};
    }
    return out;
}

inline nlohmann::json integerFromJsonCheck(const nlohmann::json& v, const std::string& path,
                                           const std::string& dtype) {
    if (!v.is_number()) throw ContractError(path, dtype + " JSON value must be a number");
    return v;
}

inline AttrValue scalarFromJson(const std::string& dtype, const nlohmann::json& v, const std::string& path) {
    if (dtype == "utf8") {
        if (!v.is_string()) throw ContractError(path, "utf8 JSON value must be a string");
        return v.get<std::string>();
    }
    if (dtype == "i64" || dtype == "u64") {
        if (!v.is_string()) throw ContractError(path, dtype + " JSON value must be a decimal string");
        const std::string s = v.get<std::string>();
        try {
            std::size_t pos = 0;
            if (dtype == "i64") {
                long long n = std::stoll(s, &pos);
                if (pos != s.size()) throw std::invalid_argument("trailing");
                return static_cast<std::int64_t>(n);
            }
            if (!s.empty() && s[0] == '-') throw std::invalid_argument("negative");
            unsigned long long n = std::stoull(s, &pos);
            if (pos != s.size()) throw std::invalid_argument("trailing");
            return static_cast<std::uint64_t>(n);
        } catch (const std::exception&) {
            throw ContractError(path, dtype + " JSON value \"" + s + "\" is not a valid integer");
        }
    }
    if (isFloat(dtype)) {
        if (v.is_string()) {
            const std::string s = v.get<std::string>();
            if (s == "NaN") return std::numeric_limits<double>::quiet_NaN();
            if (s == "Infinity") return std::numeric_limits<double>::infinity();
            if (s == "-Infinity") return -std::numeric_limits<double>::infinity();
            throw ContractError(path, "invalid float JSON value \"" + s + "\"");
        }
        integerFromJsonCheck(v, path, dtype);
        return v.get<double>();
    }
    integerFromJsonCheck(v, path, dtype);
    if (isSigned(dtype)) {
        std::int64_t n;
        if (v.is_number_integer()) {
            if (v.is_number_unsigned() && v.get<std::uint64_t>() > static_cast<std::uint64_t>(INT64_MAX))
                throw ContractError(path, dtype + " value is out of range");
            n = v.get<std::int64_t>();
        } else {
            double d = v.get<double>();
            if (d != std::trunc(d) || std::fabs(d) > 9007199254740992.0)
                throw ContractError(path, dtype + " JSON value must be an integer");
            n = static_cast<std::int64_t>(d);
        }
        checkSigned(dtype, n, path);
        return n;
    }
    std::uint64_t n;
    if (v.is_number_integer()) {
        if (!v.is_number_unsigned() && v.get<std::int64_t>() < 0)
            throw ContractError(path, dtype + " JSON value must be a non-negative integer");
        n = v.get<std::uint64_t>();
    } else {
        double d = v.get<double>();
        if (d != std::trunc(d) || d < 0 || d > 9007199254740992.0)
            throw ContractError(path, dtype + " JSON value must be a non-negative integer");
        n = static_cast<std::uint64_t>(d);
    }
    checkUnsigned(dtype, n, path);
    return n;
}

template <typename T>
inline std::vector<T> vectorFromJson(const std::string& dtype, const nlohmann::json& arr,
                                     const std::string& path) {
    std::vector<T> out;
    out.reserve(arr.size());
    for (std::size_t i = 0; i < arr.size(); ++i)
        out.push_back(std::get<T>(scalarFromJson(dtype, arr[i], path + "[" + std::to_string(i) + "]")));
    return out;
}

inline AttrValue valueFromJson(const std::string& dtype, const nlohmann::json& raw, const std::string& path) {
    if (!raw.is_array()) return scalarFromJson(dtype, raw, path);
    if (dtype == "utf8") return vectorFromJson<std::string>(dtype, raw, path);
    if (isSigned(dtype)) return vectorFromJson<std::int64_t>(dtype, raw, path);
    if (isUnsigned(dtype)) return vectorFromJson<std::uint64_t>(dtype, raw, path);
    return vectorFromJson<double>(dtype, raw, path);
}

inline std::map<std::string, Attr> attrsFromJson(const nlohmann::json& j, const std::string& path) {
    std::map<std::string, Attr> out;
    if (!j.is_object()) return out;
    for (const auto& [name, entry] : j.items()) {
        std::string attrPath = joinPath(path, "@" + name);
        std::string dtype = entry.value("dtype", std::string());
        if (dtypeTags().find(dtype) == dtypeTags().end())
            throw ContractError(attrPath, "unknown dtype \"" + dtype + "\"");
        out[name] = Attr{dtype, valueFromJson(dtype, entry.value("value", nlohmann::json()), attrPath)};
    }
    return out;
}

inline Table tableFromJson(const nlohmann::json& j, const std::string& path) {
    Table table;
    if (!j.contains("rows") || !j.at("rows").is_number_unsigned())
        throw ContractError(path, "missing rows");
    table.rows = j.at("rows").get<std::uint64_t>();
    const nlohmann::json columns = j.contains("columns") ? j.at("columns") : nlohmann::json::object();
    if (j.contains("fields")) {
        for (const auto& fj : j.at("fields")) {
            TableField f;
            f.name = fj.value("name", std::string());
            f.dtype = fj.value("dtype", std::string());
            f.size = fj.value("size", std::uint64_t{0});
            std::string fieldPath = joinPath(path, f.name);
            if (dtypeTags().find(f.dtype) == dtypeTags().end())
                throw ContractError(fieldPath, "unknown dtype \"" + f.dtype + "\"");
            if (!columns.contains(f.name) || !columns.at(f.name).is_array())
                throw ContractError(fieldPath, "missing column");
            const nlohmann::json& raw = columns.at(f.name);
            if (f.dtype == "utf8") table.columns[f.name] = vectorFromJson<std::string>(f.dtype, raw, fieldPath);
            else if (isSigned(f.dtype)) table.columns[f.name] = vectorFromJson<std::int64_t>(f.dtype, raw, fieldPath);
            else if (isUnsigned(f.dtype)) table.columns[f.name] = vectorFromJson<std::uint64_t>(f.dtype, raw, fieldPath);
            else table.columns[f.name] = vectorFromJson<double>(f.dtype, raw, fieldPath);
            table.fields.push_back(std::move(f));
        }
    }
    if (j.contains("attrs")) table.attrs = attrsFromJson(j.at("attrs"), path);
    return table;
}

inline Tree treeFromJsonImpl(const nlohmann::json& j, const std::string& path) {
    Tree t;
    if (j.contains("attrs")) t.attrs = attrsFromJson(j.at("attrs"), path);
    if (j.contains("children") && j.at("children").is_object())
        for (const auto& [name, c] : j.at("children").items())
            t.children[name] = treeFromJsonImpl(c, joinPath(path, name));
    if (j.contains("datasets") && j.at("datasets").is_object()) {
        for (const auto& [name, d] : j.at("datasets").items()) {
            std::string dsPath = joinPath(path, name);
            Dataset ds;
            ds.dtype = d.value("dtype", std::string());
            if (dtypeTags().find(ds.dtype) == dtypeTags().end() || ds.dtype == "utf8")
                throw ContractError(dsPath, "dataset dtype must be numeric, got \"" + ds.dtype + "\"");
            if (d.contains("shape"))
                for (const auto& n : d.at("shape")) ds.shape.push_back(n.get<std::uint64_t>());
            nlohmann::json data = d.contains("data") ? d.at("data") : nlohmann::json::array();
            if (isSigned(ds.dtype)) ds.data = vectorFromJson<std::int64_t>(ds.dtype, data, dsPath);
            else if (isUnsigned(ds.dtype)) ds.data = vectorFromJson<std::uint64_t>(ds.dtype, data, dsPath);
            else ds.data = vectorFromJson<double>(ds.dtype, data, dsPath);
            if (d.contains("attrs")) ds.attrs = attrsFromJson(d.at("attrs"), dsPath);
            t.datasets[name] = std::move(ds);
        }
    }
    if (j.contains("tables") && j.at("tables").is_object())
        for (const auto& [name, tj] : j.at("tables").items())
            t.tables[name] = tableFromJson(tj, joinPath(path, name));
    return t;
}

}  // namespace detail

/// The tree's lossless JSON form.
inline nlohmann::json treeToJson(const Tree& t) {
    nlohmann::json children = nlohmann::json::object();
    for (const auto& [name, c] : t.children) children[name] = treeToJson(c);
    nlohmann::json datasets = nlohmann::json::object();
    for (const auto& [name, ds] : t.datasets) {
        nlohmann::json data = std::visit(
            [&](const auto& v) { return detail::vectorToJson(ds.dtype, v); }, ds.data);
        datasets[name] = {{"dtype", ds.dtype}, {"shape", ds.shape}, {"data", std::move(data)},
                          {"attrs", detail::attrsToJson(ds.attrs)}};
    }
    nlohmann::json out = {{"attrs", detail::attrsToJson(t.attrs)}, {"children", std::move(children)},
                          {"datasets", std::move(datasets)}};
    if (!t.tables.empty()) {
        nlohmann::json tables = nlohmann::json::object();
        for (const auto& [name, table] : t.tables) {
            nlohmann::json fields = nlohmann::json::array();
            nlohmann::json columns = nlohmann::json::object();
            for (const TableField& f : table.fields) {
                nlohmann::json field = {{"name", f.name}, {"dtype", f.dtype}};
                if (f.size > 0) field["size"] = f.size;
                fields.push_back(std::move(field));
                columns[f.name] = std::visit(
                    [&](const auto& v) { return detail::vectorToJson(f.dtype, v); }, table.columns.at(f.name));
            }
            tables[name] = {{"fields", std::move(fields)}, {"rows", table.rows},
                            {"columns", std::move(columns)}, {"attrs", detail::attrsToJson(table.attrs)}};
        }
        out["tables"] = std::move(tables);
    }
    return out;
}

/// Inverse of treeToJson; the result is validated.
inline Tree treeFromJson(const nlohmann::json& j) {
    Tree t = detail::treeFromJsonImpl(j, "");
    validate(t);
    return t;
}

inline void to_json(nlohmann::json& j, const Tree& t) { j = treeToJson(t); }
inline void from_json(const nlohmann::json& j, Tree& t) { t = treeFromJson(j); }

}  // namespace machine_config::extension
