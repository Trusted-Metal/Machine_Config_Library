#pragma once
// Where a machine configuration file is read from.
//
// Normally just a path. A FileSource can also carry a hook that configures the HDF5 file-access
// property list before the file is opened, which lets the MCF live somewhere other than a plain
// file -- e.g. as a STORED member of a Lossless Build Log (.lbl) container, read in place through
// the LBL virtual file driver without extracting it:
//
//     lbl::BuildLog log = lbl::BuildLog::open("build.lbl");
//     auto archive = *log.archive();
//     machine_config::FileSource src{archive.path(), archive.fapl_configurator(*log.machine_config_member())};
//     lbl::Hdf5Lock lock;                       // LBL and MCF share one (non thread-safe) HDF5
//     auto config = machine_config::MachineConfigReader(src).parse();
//
// The hook runs on a fresh H5P_FILE_ACCESS list; HDF5 then opens `path` read-only with it. The
// hook must belong to the same HDF5 library instance this library links. Writers stay path-based:
// a FileSource with a hook is read-only and never used as a save() target.

#include <highfive/H5File.hpp>
#include <highfive/H5PropertyList.hpp>

#include <filesystem>
#include <functional>
#include <string>
#include <utility>

namespace machine_config {

struct FileSource {
    std::filesystem::path path;
    std::function<void(hid_t fapl)> configure_fapl;

    FileSource() = default;
    FileSource(std::filesystem::path p) : path(std::move(p)) {}  // NOLINT: implicit by design
    FileSource(const std::string& p) : path(p) {}                 // NOLINT
    FileSource(const char* p) : path(p) {}                        // NOLINT
    FileSource(std::filesystem::path p, std::function<void(hid_t)> hook)
        : path(std::move(p)), configure_fapl(std::move(hook)) {}

    /// True when the bytes are not simply the file at `path` (a FAPL hook redirects the open).
    bool redirected() const { return static_cast<bool>(configure_fapl); }

    std::string string() const { return path.string(); }

    // Keeps call sites and registry callbacks written against `const std::filesystem::path&` working.
    operator const std::filesystem::path&() const { return path; }  // NOLINT

    HighFive::File openReadOnly() const {
        if (!configure_fapl) return HighFive::File(path.string(), HighFive::File::ReadOnly);
        HighFive::FileAccessProps fapl;
        fapl.add(Hook{&configure_fapl});
        return HighFive::File(path.string(), HighFive::File::ReadOnly, fapl);
    }

private:
    struct Hook {
        const std::function<void(hid_t)>* fn;
        void apply(hid_t fapl) const { (*fn)(fapl); }
    };
};

/// Default save() target for a file opened from `source`: none when the bytes came through a FAPL
/// hook (writing to `path` would overwrite the container that holds the MCF).
inline std::filesystem::path savePath(const FileSource& source) {
    return source.redirected() ? std::filesystem::path{} : source.path;
}

}  // namespace machine_config
