// machine_config_cli — CLI entry point.
// Sub-steps: §4.8 export-json; §4.15 copy-hdf5; §4.16 correction-hash.
#include "machine_config/reader.hpp"

#include <CLI/CLI.hpp>

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
    app.require_subcommand(1);

    // §4.8: export-json <path>
    auto* cmd = app.add_subcommand("export-json", "Parse an HDF5 machine config and print canonical JSON");
    std::string path;
    cmd->add_option("path", path, "Path to .h5 machine config file")->required();

    CLI11_PARSE(app, argc, argv);

    try {
        machine_config::MachineConfigReader reader{path};
        std::cout << reader.toJson() << "\n";
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        return 1;
    }
    return 0;
}
