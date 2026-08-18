// S-07: OPCUA config roundtrip
#include "s07_opcua.hpp"
#include "machine_config/machine_config.hpp"

#include <set>

namespace s07 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config_opcua.h5";
    if (!std::filesystem::exists(path)) {
        return {false, "OPCUA fixture not found: " + path.string()};
    }

    try {
        MachineConfig cfg = MachineConfigReader{path}.parse();
        if (!cfg.opcua) return {false, "opcua is absent after reading OPCUA fixture"};

        std::string origUrl = cfg.opcua->client.server_url;
        auto origTimeout = cfg.opcua->client.session_timeout;
        auto origTriggersEnabled = cfg.opcua->triggers_enabled;
        std::set<std::string> origNames;
        for (const auto& [name, trig] : cfg.opcua->triggers) origNames.insert(name);

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();
        if (!rb.opcua) return {false, "opcua is absent after roundtrip"};

        if (rb.opcua->client.server_url != origUrl) {
            return {false, "server_url changed"};
        }
        if (rb.opcua->client.session_timeout != origTimeout) {
            return {false, "session_timeout changed"};
        }
        if (rb.opcua->triggers_enabled != origTriggersEnabled) {
            return {false, "triggers_enabled changed"};
        }

        std::set<std::string> rbNames;
        for (const auto& [name, trig] : rb.opcua->triggers) rbNames.insert(name);
        if (rbNames != origNames) {
            return {false, "trigger names changed"};
        }

        const std::string co = "Chamber Oxygen Level";
        auto oit = cfg.opcua->triggers.find(co);
        auto rit = rb.opcua->triggers.find(co);
        if (oit != cfg.opcua->triggers.end() && rit != rb.opcua->triggers.end()) {
            if (oit->second.signal != rit->second.signal || oit->second.subsystem != rit->second.subsystem) {
                return {false, "'" + co + "' signal/subsystem changed"};
            }
        }

        return {true, "OPCUA roundtrip OK: " + std::to_string(origNames.size()) + " triggers, url=\"" + origUrl + "\""};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s07
