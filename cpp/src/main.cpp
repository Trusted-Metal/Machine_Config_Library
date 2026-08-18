// machine_config_cli — CLI entry point.
// Sub-steps: §4.8 export-json; §4.14 write-hdf5; §4.15 copy-hdf5; §4.16 correction-hash.
#include "machine_config/machine_config.hpp"

#include <CLI/CLI.hpp>
#include <picosha2.h>

#include <fstream>
#include <iostream>
#include <string>

#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#endif

int main(int argc, char* argv[]) {
    // Force binary-mode stdout on Windows so JSON newlines are not
    // silently converted to \r\n (which would break cross-platform diffs).
#ifdef _WIN32
    _setmode(_fileno(stdout), _O_BINARY);
#endif

    CLI::App app{"Machine Config Library CLI"};
    app.set_version_flag("--version", std::string(machine_config::kVersionString));
    app.require_subcommand(1);

    // §4.8: export-json <path>
    auto* exp_cmd = app.add_subcommand("export-json", "Parse an HDF5 machine config and print canonical JSON");
    std::string exp_path;
    exp_cmd->add_option("path", exp_path, "Path to .h5 machine config file")->required();

    // §4.14: write-hdf5 <json> <output.h5>
    auto* write_cmd = app.add_subcommand("write-hdf5", "Write canonical JSON to an HDF5 machine config file");
    std::string write_json, write_out;
    write_cmd->add_option("json",   write_json, "Path to canonical JSON input")->required();
    write_cmd->add_option("output", write_out,  "Path to output .h5 file")->required();

    // §4.15: copy-hdf5 <input> <output>
    auto* copy_cmd = app.add_subcommand("copy-hdf5", "Binary round-trip copy of an HDF5 machine config file");
    std::string copy_in, copy_out;
    copy_cmd->add_option("input",  copy_in,  "Path to input .h5 file")->required();
    copy_cmd->add_option("output", copy_out, "Path to output .h5 file")->required();

    // §4.16: correction-hash <path> [--train N] [--inverse]
    auto* chash_cmd = app.add_subcommand("correction-hash", "Print SHA-256 of a flat little-endian float64 correction grid");
    std::string chash_path;
    int         chash_train   = 0;
    bool        chash_inverse = false;
    chash_cmd->add_option("path",    chash_path,   "Path to .h5 file")->required();
    chash_cmd->add_option("--train", chash_train,  "Train index (default: 0)");
    chash_cmd->add_flag("--inverse", chash_inverse, "Hash the inverse correction grid");

    CLI11_PARSE(app, argc, argv);

    try {
        if (app.got_subcommand(exp_cmd)) {
            machine_config::MachineConfigReader reader{exp_path};
            std::cout << reader.toJson() << "\n";
        } else if (app.got_subcommand(write_cmd)) {
            std::ifstream ifs(write_json);
            if (!ifs)
                throw std::runtime_error("Cannot open JSON file: " + write_json);
            auto cfg = nlohmann::json::parse(ifs).get<machine_config::MachineConfig>();
            machine_config::MachineConfigWriter{cfg}.write(write_out);
        } else if (app.got_subcommand(copy_cmd)) {
            auto cfg = machine_config::MachineConfigReader{copy_in}.parseWithBinary();
            machine_config::MachineConfigWriter{cfg}.write(copy_out);
        } else if (app.got_subcommand(chash_cmd)) {
            machine_config::MachineConfigReader reader{chash_path};
            auto cd = chash_inverse
                ? reader.getInverseCorrectionData(static_cast<size_t>(chash_train))
                : reader.getCorrectionData(static_cast<size_t>(chash_train));
            const auto* p = reinterpret_cast<const uint8_t*>(cd.data.data());
            std::cout << picosha2::hash256_hex_string(p, p + cd.data.size() * sizeof(double)) << "\n";
        }
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        return 1;
    }
    return 0;
}
