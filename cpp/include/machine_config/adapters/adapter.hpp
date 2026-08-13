#pragma once
#include <string_view>
#include <nlohmann/json.hpp>

namespace machine_config::adapters {

class IAdapter {
public:
    virtual ~IAdapter() = default;
    virtual std::string_view from_version() const noexcept = 0;
    virtual std::string_view to_version()   const noexcept = 0;
    virtual nlohmann::json adapt(nlohmann::json config) const = 0;
};

} // namespace machine_config::adapters
