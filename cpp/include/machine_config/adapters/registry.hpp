#pragma once
#include <memory>
#include <string_view>
#include <vector>
#include "machine_config/adapters/adapter.hpp"
#include "machine_config/adapters/test_v0_9_to_v1_0.hpp"

namespace machine_config::adapters {

class AdapterRegistry {
public:
    static AdapterRegistry& instance() {
        static AdapterRegistry inst;
        return inst;
    }

    void register_adapter(std::shared_ptr<IAdapter> adapter) {
        adapters_.push_back(std::move(adapter));
    }

    // Returns the ordered chain needed to migrate from `from` to `to`.
    std::vector<std::shared_ptr<IAdapter>> get_chain(
        std::string_view from, std::string_view to) const
    {
        if (from == to) return {};
        if (from == "0.9" && to == "1.0")
            return { std::make_shared<V0_9_to_V1_0>() };
        return {};
    }

private:
    std::vector<std::shared_ptr<IAdapter>> adapters_;
};

} // namespace machine_config::adapters
