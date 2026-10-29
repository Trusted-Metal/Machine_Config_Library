#pragma once
// Reads and writes extension::Tree as HDF5 groups.
//
// Version-independent: the adapters decide where an extension lives (Extensions/<name>/ in 1.1, the
// legacy OPCUA/ group in 1.0); this header only maps the contract's tree to and from the HDF5 group
// at that location. Everything is read and written verbatim with dtypes preserved, so extensions
// core has no model for survive a read-modify-write.
//
// Uses the HDF5 C API directly (HighFive hides the exact stored integer width/sign that the
// contract must preserve); HighFive objects are only used to obtain hid_t handles.

#include "machine_config/extension_contract.hpp"

#include <hdf5.h>

#include <cstdint>
#include <cstring>
#include <functional>
#include <map>
#include <string>
#include <vector>

namespace machine_config::extension {

namespace h5 {

/// Closes an HDF5 handle with the matching close function when it goes out of scope.
class Handle {
public:
    using Closer = herr_t (*)(hid_t);
    Handle(hid_t id, Closer close) : id_(id), close_(close) {}
    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;
    ~Handle() { if (id_ >= 0) close_(id_); }
    hid_t id() const { return id_; }
    bool ok() const { return id_ >= 0; }

private:
    hid_t id_;
    Closer close_;
};

inline std::string dtypeOf(hid_t tid) {
    const std::size_t size = H5Tget_size(tid);
    switch (H5Tget_class(tid)) {
        case H5T_STRING: return "utf8";
        case H5T_INTEGER:
            if (size == 1 || size == 2 || size == 4 || size == 8)
                return (H5Tget_sign(tid) == H5T_SGN_NONE ? "u" : "i") + std::to_string(size * 8);
            break;
        case H5T_FLOAT:
            if (size == 4 || size == 8) return "f" + std::to_string(size * 8);
            break;
        default: break;
    }
    return "";
}

inline hid_t fileTypeFor(const std::string& dtype) {
    if (dtype == "i8") return H5T_STD_I8LE;
    if (dtype == "u8") return H5T_STD_U8LE;
    if (dtype == "i16") return H5T_STD_I16LE;
    if (dtype == "u16") return H5T_STD_U16LE;
    if (dtype == "i32") return H5T_STD_I32LE;
    if (dtype == "u32") return H5T_STD_U32LE;
    if (dtype == "i64") return H5T_STD_I64LE;
    if (dtype == "u64") return H5T_STD_U64LE;
    if (dtype == "f32") return H5T_IEEE_F32LE;
    return H5T_IEEE_F64LE;
}

/// Names of every attribute on an object (no count cap). Probes past the last attribute with
/// HDF5 error printing suspended, since that probe fails by design.
inline std::vector<std::string> attrNames(hid_t obj) {
    std::vector<std::string> names;
    for (hsize_t i = 0;; ++i) {
        ssize_t len = -1;
        H5E_BEGIN_TRY {
            len = H5Aget_name_by_idx(obj, ".", H5_INDEX_NAME, H5_ITER_INC, i, nullptr, 0, H5P_DEFAULT);
        } H5E_END_TRY
        if (len < 0) break;
        std::string name(static_cast<std::size_t>(len), '\0');
        H5Aget_name_by_idx(obj, ".", H5_INDEX_NAME, H5_ITER_INC, i, name.data(),
                           static_cast<std::size_t>(len) + 1, H5P_DEFAULT);
        names.push_back(std::move(name));
    }
    return names;
}

/// Reads `n` elements of dtype from `read` (which wraps H5Aread/H5Dread) into one of the three
/// numeric element vectors.
inline DatasetData readNumeric(const std::string& dtype, std::size_t n,
                               const std::function<herr_t(hid_t, void*)>& read, const std::string& path) {
    auto fail = [&]() { throw ContractError(path, "HDF5 read failed"); };
    if (detail::isSigned(dtype)) {
        std::vector<std::int64_t> v(n);
        if (n && read(H5T_NATIVE_INT64, v.data()) < 0) fail();
        return v;
    }
    if (detail::isUnsigned(dtype)) {
        std::vector<std::uint64_t> v(n);
        if (n && read(H5T_NATIVE_UINT64, v.data()) < 0) fail();
        return v;
    }
    std::vector<double> v(n);
    if (n && read(H5T_NATIVE_DOUBLE, v.data()) < 0) fail();
    return v;
}

inline Attr readAttr(hid_t obj, const std::string& name, const std::string& path) {
    Handle aid(H5Aopen(obj, name.c_str(), H5P_DEFAULT), H5Aclose);
    if (!aid.ok()) throw ContractError(path, "cannot open attribute");
    Handle tid(H5Aget_type(aid.id()), H5Tclose);
    if (!tid.ok()) throw ContractError(path, "cannot read attribute type");
    std::string dtype = dtypeOf(tid.id());
    if (dtype.empty()) throw ContractError(path, "unsupported HDF5 type");
    Handle space(H5Aget_space(aid.id()), H5Sclose);
    const int rank = H5Sget_simple_extent_ndims(space.id());
    if (rank > 1)
        throw ContractError(path, "attributes must be scalar or 1-D, got rank " + std::to_string(rank));
    const std::size_t n = static_cast<std::size_t>(H5Sget_simple_extent_npoints(space.id()));

    Attr a;
    a.dtype = dtype;
    if (dtype == "utf8") {
        std::vector<std::string> out(n);
        if (n) {
            if (H5Tis_variable_str(tid.id()) > 0) {
                std::vector<char*> ptrs(n, nullptr);
                if (H5Aread(aid.id(), tid.id(), ptrs.data()) < 0)
                    throw ContractError(path, "HDF5 read failed");
                for (std::size_t i = 0; i < n; ++i) {
                    if (ptrs[i]) { out[i] = ptrs[i]; H5free_memory(ptrs[i]); }
                }
            } else {
                const std::size_t sz = H5Tget_size(tid.id());
                std::vector<char> buf(sz * n);
                if (H5Aread(aid.id(), tid.id(), buf.data()) < 0)
                    throw ContractError(path, "HDF5 read failed");
                for (std::size_t i = 0; i < n; ++i)
                    out[i] = std::string(buf.data() + i * sz, strnlen(buf.data() + i * sz, sz));
            }
        }
        if (rank == 0) a.value = out.at(0);
        else a.value = std::move(out);
        return a;
    }
    DatasetData data = readNumeric(
        dtype, n, [&](hid_t mem, void* buf) { return H5Aread(aid.id(), mem, buf); }, path);
    std::visit(
        [&](auto& v) {
            if (rank == 0) a.value = v.at(0);
            else a.value = std::move(v);
        },
        data);
    return a;
}

inline std::map<std::string, Attr> readAttrs(hid_t obj, const std::string& path) {
    std::map<std::string, Attr> out;
    for (const auto& name : attrNames(obj))
        out[name] = readAttr(obj, name, detail::joinPath(path, "@" + name));
    return out;
}

inline void writeAttr(hid_t obj, const std::string& name, const Attr& a, const std::string& path) {
    const bool isVec = std::visit(
        [](const auto& v) {
            using V = std::decay_t<decltype(v)>;
            return !(std::is_same_v<V, std::string> || std::is_same_v<V, std::int64_t> ||
                     std::is_same_v<V, std::uint64_t> || std::is_same_v<V, double>);
        },
        a.value);
    const std::size_t count = std::visit(
        [](const auto& v) -> std::size_t {
            using V = std::decay_t<decltype(v)>;
            if constexpr (std::is_same_v<V, std::string> || std::is_same_v<V, std::int64_t> ||
                          std::is_same_v<V, std::uint64_t> || std::is_same_v<V, double>)
                return 1;
            else
                return v.size();
        },
        a.value);
    Handle space(isVec ? [&] {
        hsize_t dim = count;
        return H5Screate_simple(1, &dim, nullptr);
    }() : H5Screate(H5S_SCALAR), H5Sclose);

    auto fail = [&]() { throw ContractError(path, "HDF5 write failed"); };

    if (a.dtype == "utf8") {
        Handle tid(H5Tcopy(H5T_C_S1), H5Tclose);
        H5Tset_size(tid.id(), H5T_VARIABLE);
        H5Tset_cset(tid.id(), H5T_CSET_UTF8);
        Handle aid(H5Acreate2(obj, name.c_str(), tid.id(), space.id(), H5P_DEFAULT, H5P_DEFAULT), H5Aclose);
        if (!aid.ok()) fail();
        std::vector<const char*> ptrs;
        if (auto* s = std::get_if<std::string>(&a.value)) ptrs.push_back(s->c_str());
        else for (const auto& s2 : std::get<std::vector<std::string>>(a.value)) ptrs.push_back(s2.c_str());
        ptrs.push_back(nullptr);  // keeps data() valid for empty arrays
        if (H5Awrite(aid.id(), tid.id(), ptrs.data()) < 0) fail();
        return;
    }
    Handle aid(H5Acreate2(obj, name.c_str(), fileTypeFor(a.dtype), space.id(), H5P_DEFAULT, H5P_DEFAULT), H5Aclose);
    if (!aid.ok()) fail();
    int64_t dummy = 0;
    herr_t rc = 0;
    std::visit(
        [&](const auto& v) {
            using V = std::decay_t<decltype(v)>;
            hid_t mem = detail::isSigned(a.dtype) ? H5T_NATIVE_INT64
                        : detail::isUnsigned(a.dtype) ? H5T_NATIVE_UINT64 : H5T_NATIVE_DOUBLE;
            if constexpr (std::is_same_v<V, std::string> || std::is_same_v<V, std::vector<std::string>>) {
                rc = -1;
            } else if constexpr (std::is_same_v<V, std::int64_t> || std::is_same_v<V, std::uint64_t> ||
                                 std::is_same_v<V, double>) {
                rc = H5Awrite(aid.id(), mem, &v);
            } else {
                rc = H5Awrite(aid.id(), mem, v.empty() ? static_cast<const void*>(&dummy) : v.data());
            }
        },
        a.value);
    if (rc < 0) fail();
}

inline void writeAttrs(hid_t obj, const std::map<std::string, Attr>& attrs, const std::string& path) {
    for (const auto& [name, a] : attrs) writeAttr(obj, name, a, detail::joinPath(path, "@" + name));
}

inline Dataset readDataset(hid_t group, const std::string& name, const std::string& path) {
    Handle ds(H5Dopen2(group, name.c_str(), H5P_DEFAULT), H5Dclose);
    if (!ds.ok()) throw ContractError(path, "cannot open dataset");
    Handle tid(H5Dget_type(ds.id()), H5Tclose);
    std::string dtype = dtypeOf(tid.id());
    if (dtype.empty()) throw ContractError(path, "unsupported HDF5 type");
    if (dtype == "utf8") throw ContractError(path, "string datasets are not supported");
    Handle space(H5Dget_space(ds.id()), H5Sclose);
    const int rank = H5Sget_simple_extent_ndims(space.id());
    std::vector<hsize_t> dims(static_cast<std::size_t>(rank));
    if (rank > 0) H5Sget_simple_extent_dims(space.id(), dims.data(), nullptr);
    const std::size_t n = static_cast<std::size_t>(H5Sget_simple_extent_npoints(space.id()));

    Dataset out;
    out.dtype = dtype;
    out.shape.assign(dims.begin(), dims.end());
    out.data = readNumeric(
        dtype, n,
        [&](hid_t mem, void* buf) { return H5Dread(ds.id(), mem, H5S_ALL, H5S_ALL, H5P_DEFAULT, buf); },
        path);
    out.attrs = readAttrs(ds.id(), path);
    return out;
}

/// True when the named dataset has a compound type.
inline bool isCompoundDataset(hid_t group, const std::string& name) {
    Handle ds(H5Dopen2(group, name.c_str(), H5P_DEFAULT), H5Dclose);
    if (!ds.ok()) return false;
    Handle tid(H5Dget_type(ds.id()), H5Tclose);
    return tid.ok() && H5Tget_class(tid.id()) == H5T_COMPOUND;
}

inline std::size_t memberWidth(const TableField& f) {
    if (f.dtype == "utf8") return static_cast<std::size_t>(f.size);
    if (f.dtype == "i8" || f.dtype == "u8") return 1;
    if (f.dtype == "i16" || f.dtype == "u16") return 2;
    if (f.dtype == "i32" || f.dtype == "u32" || f.dtype == "f32") return 4;
    return 8;
}

template <typename T>
inline T loadNative(const unsigned char* p) {
    T v;
    std::memcpy(&v, p, sizeof(T));
    return v;
}

/// Reads a 1-D compound dataset as a Table; members the contract cannot carry fail with the
/// member's path.
inline Table readTable(hid_t group, const std::string& name, const std::string& path) {
    Handle ds(H5Dopen2(group, name.c_str(), H5P_DEFAULT), H5Dclose);
    if (!ds.ok()) throw ContractError(path, "cannot open dataset");
    Handle ftid(H5Dget_type(ds.id()), H5Tclose);
    Handle space(H5Dget_space(ds.id()), H5Sclose);
    const int rank = H5Sget_simple_extent_ndims(space.id());
    if (rank != 1)
        throw ContractError(path, "compound datasets must be 1-D, got rank " + std::to_string(rank));
    const std::size_t rows = static_cast<std::size_t>(H5Sget_simple_extent_npoints(space.id()));
    Handle ntid(H5Tget_native_type(ftid.id(), H5T_DIR_ASCEND), H5Tclose);
    if (!ntid.ok()) throw ContractError(path, "cannot read the compound type");
    const std::size_t rowSize = H5Tget_size(ntid.id());

    Table table;
    table.rows = rows;
    std::vector<std::size_t> offsets;
    const unsigned n = static_cast<unsigned>(H5Tget_nmembers(ftid.id()));
    for (unsigned i = 0; i < n; ++i) {
        char* raw = H5Tget_member_name(ftid.id(), i);
        const std::string member = raw ? raw : "";
        if (raw) H5free_memory(raw);
        const std::string memberPath = detail::joinPath(path, member);
        Handle mt(H5Tget_member_type(ftid.id(), i), H5Tclose);
        if (!mt.ok()) throw ContractError(memberPath, "cannot read the member type");
        TableField f;
        f.name = member;
        const H5T_class_t cls = H5Tget_class(mt.id());
        if (cls == H5T_STRING) {
            if (H5Tis_variable_str(mt.id()) > 0)
                throw ContractError(memberPath, "variable-length strings are not supported in a compound type");
            f.dtype = "utf8";
            f.size = H5Tget_size(mt.id());
        } else if (cls == H5T_INTEGER || cls == H5T_FLOAT) {
            if (H5Tget_order(mt.id()) == H5T_ORDER_BE)
                throw ContractError(memberPath, "big-endian compound members are not supported");
            f.dtype = dtypeOf(mt.id());
        }
        if (f.dtype.empty())
            throw ContractError(memberPath,
                                "unsupported compound member type (nested compound, array, enum and bool members are not supported)");
        offsets.push_back(H5Tget_member_offset(ntid.id(), i));
        table.fields.push_back(std::move(f));
    }

    std::vector<unsigned char> buf(rows * rowSize);
    if (rows && H5Dread(ds.id(), ntid.id(), H5S_ALL, H5S_ALL, H5P_DEFAULT, buf.data()) < 0)
        throw ContractError(path, "HDF5 read failed");

    for (std::size_t c = 0; c < table.fields.size(); ++c) {
        const TableField& f = table.fields[c];
        const std::size_t w = memberWidth(f);
        auto at = [&](std::size_t r) { return buf.data() + r * rowSize + offsets[c]; };
        if (f.dtype == "utf8") {
            std::vector<std::string> v;
            for (std::size_t r = 0; r < rows; ++r)
                v.emplace_back(reinterpret_cast<const char*>(at(r)), strnlen(reinterpret_cast<const char*>(at(r)), w));
            table.columns[f.name] = std::move(v);
        } else if (detail::isSigned(f.dtype)) {
            std::vector<std::int64_t> v;
            for (std::size_t r = 0; r < rows; ++r) {
                switch (w) {
                    case 1: v.push_back(loadNative<std::int8_t>(at(r))); break;
                    case 2: v.push_back(loadNative<std::int16_t>(at(r))); break;
                    case 4: v.push_back(loadNative<std::int32_t>(at(r))); break;
                    default: v.push_back(loadNative<std::int64_t>(at(r))); break;
                }
            }
            table.columns[f.name] = std::move(v);
        } else if (detail::isUnsigned(f.dtype)) {
            std::vector<std::uint64_t> v;
            for (std::size_t r = 0; r < rows; ++r) {
                switch (w) {
                    case 1: v.push_back(loadNative<std::uint8_t>(at(r))); break;
                    case 2: v.push_back(loadNative<std::uint16_t>(at(r))); break;
                    case 4: v.push_back(loadNative<std::uint32_t>(at(r))); break;
                    default: v.push_back(loadNative<std::uint64_t>(at(r))); break;
                }
            }
            table.columns[f.name] = std::move(v);
        } else {
            std::vector<double> v;
            for (std::size_t r = 0; r < rows; ++r)
                v.push_back(w == 4 ? static_cast<double>(loadNative<float>(at(r))) : loadNative<double>(at(r)));
            table.columns[f.name] = std::move(v);
        }
    }
    table.attrs = readAttrs(ds.id(), path);
    return table;
}

template <typename T>
inline void storeLE(unsigned char* p, T v) {
    std::memcpy(p, &v, sizeof(T));
}

inline void writeTable(hid_t group, const std::string& name, const Table& table, const std::string& path) {
    std::size_t rowSize = 0;
    for (const TableField& f : table.fields) rowSize += memberWidth(f);
    Handle tid(H5Tcreate(H5T_COMPOUND, rowSize), H5Tclose);
    if (!tid.ok()) throw ContractError(path, "HDF5 write failed");
    std::size_t offset = 0;
    std::vector<std::size_t> offsets;
    for (const TableField& f : table.fields) {
        herr_t rc;
        if (f.dtype == "utf8") {
            Handle st(H5Tcopy(H5T_C_S1), H5Tclose);
            if (H5Tset_size(st.id(), static_cast<std::size_t>(f.size)) < 0 ||
                H5Tset_cset(st.id(), H5T_CSET_UTF8) < 0 || H5Tset_strpad(st.id(), H5T_STR_NULLPAD) < 0)
                throw ContractError(detail::joinPath(path, f.name), "HDF5 write failed");
            rc = H5Tinsert(tid.id(), f.name.c_str(), offset, st.id());
        } else {
            rc = H5Tinsert(tid.id(), f.name.c_str(), offset, fileTypeFor(f.dtype));
        }
        if (rc < 0) throw ContractError(detail::joinPath(path, f.name), "HDF5 write failed");
        offsets.push_back(offset);
        offset += memberWidth(f);
    }
    hsize_t dim = table.rows;
    Handle space(H5Screate_simple(1, &dim, nullptr), H5Sclose);
    Handle id(H5Dcreate2(group, name.c_str(), tid.id(), space.id(), H5P_DEFAULT, H5P_DEFAULT, H5P_DEFAULT),
              H5Dclose);
    if (!id.ok()) throw ContractError(path, "HDF5 write failed");

    if (table.rows > 0) {
        std::vector<unsigned char> buf(static_cast<std::size_t>(table.rows) * rowSize, 0);
        for (std::size_t c = 0; c < table.fields.size(); ++c) {
            const TableField& f = table.fields[c];
            const ColumnData& column = table.columns.at(f.name);
            for (std::size_t r = 0; r < table.rows; ++r) {
                unsigned char* p = buf.data() + r * rowSize + offsets[c];
                if (const auto* s = std::get_if<std::vector<std::string>>(&column)) {
                    std::memcpy(p, (*s)[r].data(), (*s)[r].size());
                } else if (const auto* i = std::get_if<std::vector<std::int64_t>>(&column)) {
                    const std::int64_t v = (*i)[r];
                    switch (memberWidth(f)) {
                        case 1: storeLE(p, static_cast<std::int8_t>(v)); break;
                        case 2: storeLE(p, static_cast<std::int16_t>(v)); break;
                        case 4: storeLE(p, static_cast<std::int32_t>(v)); break;
                        default: storeLE(p, v); break;
                    }
                } else if (const auto* u = std::get_if<std::vector<std::uint64_t>>(&column)) {
                    const std::uint64_t v = (*u)[r];
                    switch (memberWidth(f)) {
                        case 1: storeLE(p, static_cast<std::uint8_t>(v)); break;
                        case 2: storeLE(p, static_cast<std::uint16_t>(v)); break;
                        case 4: storeLE(p, static_cast<std::uint32_t>(v)); break;
                        default: storeLE(p, v); break;
                    }
                } else {
                    const double v = std::get<std::vector<double>>(column)[r];
                    if (memberWidth(f) == 4) storeLE(p, static_cast<float>(v));
                    else storeLE(p, v);
                }
            }
        }
        if (H5Dwrite(id.id(), tid.id(), H5S_ALL, H5S_ALL, H5P_DEFAULT, buf.data()) < 0)
            throw ContractError(path, "HDF5 write failed");
    }
    writeAttrs(id.id(), table.attrs, path);
}

}  // namespace h5

/// Reads a group and everything below it verbatim; unsupported HDF5 types fail with their path.
/// `path` prefixes every reported location (use the extension's name).
inline Tree readExtensionTree(hid_t group, const std::string& path) {
    Tree tree;
    tree.attrs = h5::readAttrs(group, path);
    H5G_info_t info;
    if (H5Gget_info(group, &info) < 0) throw ContractError(path, "cannot list members");
    for (hsize_t i = 0; i < info.nlinks; ++i) {
        ssize_t len = H5Lget_name_by_idx(group, ".", H5_INDEX_NAME, H5_ITER_INC, i, nullptr, 0, H5P_DEFAULT);
        if (len < 0) throw ContractError(path, "cannot list members");
        std::string name(static_cast<std::size_t>(len), '\0');
        H5Lget_name_by_idx(group, ".", H5_INDEX_NAME, H5_ITER_INC, i, name.data(),
                           static_cast<std::size_t>(len) + 1, H5P_DEFAULT);
        const std::string childPath = detail::joinPath(path, name);
        h5::Handle obj(H5Oopen(group, name.c_str(), H5P_DEFAULT), H5Oclose);
        if (!obj.ok()) throw ContractError(childPath, "cannot open member");
        const H5I_type_t kind = H5Iget_type(obj.id());
        if (kind == H5I_GROUP) tree.children[name] = readExtensionTree(obj.id(), childPath);
        else if (kind == H5I_DATASET) {
            if (h5::isCompoundDataset(group, name)) tree.tables[name] = h5::readTable(group, name, childPath);
            else tree.datasets[name] = h5::readDataset(group, name, childPath);
        }
        else throw ContractError(childPath, "only groups and datasets are supported");
    }
    return tree;
}

namespace h5 {

inline void writeGroup(hid_t group, const Tree& t, const std::string& path) {
    writeAttrs(group, t.attrs, path);
    for (const auto& [name, ds] : t.datasets) {
        const std::string dsPath = detail::joinPath(path, name);
        Handle space(ds.shape.empty() ? H5Screate(H5S_SCALAR) : [&] {
            std::vector<hsize_t> dims(ds.shape.begin(), ds.shape.end());
            return H5Screate_simple(static_cast<int>(dims.size()), dims.data(), nullptr);
        }(), H5Sclose);
        Handle id(H5Dcreate2(group, name.c_str(), fileTypeFor(ds.dtype), space.id(), H5P_DEFAULT,
                             H5P_DEFAULT, H5P_DEFAULT), H5Dclose);
        if (!id.ok()) throw ContractError(dsPath, "HDF5 write failed");
        int64_t dummy = 0;
        herr_t rc = 0;
        std::visit(
            [&](const auto& v) {
                using V = std::decay_t<decltype(v)>;
                hid_t mem = std::is_same_v<V, std::vector<std::int64_t>> ? H5T_NATIVE_INT64
                            : std::is_same_v<V, std::vector<std::uint64_t>> ? H5T_NATIVE_UINT64
                                                                           : H5T_NATIVE_DOUBLE;
                if (!v.empty())
                    rc = H5Dwrite(id.id(), mem, H5S_ALL, H5S_ALL, H5P_DEFAULT, v.data());
                (void)dummy;
            },
            ds.data);
        if (rc < 0) throw ContractError(dsPath, "HDF5 write failed");
        writeAttrs(id.id(), ds.attrs, dsPath);
    }
    for (const auto& [name, table] : t.tables) writeTable(group, name, table, detail::joinPath(path, name));
    for (const auto& [name, child] : t.children) {
        const std::string childPath = detail::joinPath(path, name);
        Handle g(H5Gcreate2(group, name.c_str(), H5P_DEFAULT, H5P_DEFAULT, H5P_DEFAULT), H5Gclose);
        if (!g.ok()) throw ContractError(childPath, "HDF5 write failed");
        writeGroup(g.id(), child, childPath);
    }
}

}  // namespace h5

/// Validates `tree` and writes it as a new group `name` under `parent`; the caller guarantees
/// `name` is unused.
inline void writeExtensionTree(hid_t parent, const std::string& name, const Tree& tree) {
    validate(tree, name);
    h5::Handle g(H5Gcreate2(parent, name.c_str(), H5P_DEFAULT, H5P_DEFAULT, H5P_DEFAULT), H5Gclose);
    if (!g.ok()) throw ContractError(name, "HDF5 write failed");
    h5::writeGroup(g.id(), tree, name);
}

}  // namespace machine_config::extension
