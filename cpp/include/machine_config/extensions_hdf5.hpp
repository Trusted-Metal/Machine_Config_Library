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
        else if (kind == H5I_DATASET) tree.datasets[name] = h5::readDataset(group, name, childPath);
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
