#pragma once
#include "machine_config/capabilities/errors.hpp"

#include <string>
#include <utility>
#include <vector>

namespace machine_config::capabilities {

template <typename T>
class Result {
 public:
  static Result Ok(T value) { return Result(true, std::move(value), CapabilityError{}); }
  // details names every individual violation at once (e.g. every missing
  // required OPCUA field) rather than only the first one encountered. Empty
  // for validation failures with nothing more specific to list.
  static Result Err(std::string code, std::string message,
                     std::vector<std::string> details = {}) {
    return Result(false, T{},
                   CapabilityError{std::move(code), std::move(message), std::move(details)});
  }

  bool ok() const { return ok_; }
  const T& value() const { return value_; }
  // The full error as one value — matches Rust's Err(CapabilityError),
  // Python's/Node.js's Err.error, and Go's second (*Error) return value.
  const CapabilityError& error() const { return error_; }
  const std::string& errorCode() const { return error_.code; }
  const std::string& errorMessage() const { return error_.message; }
  const std::vector<std::string>& errorDetails() const { return error_.details; }

 private:
  Result(bool ok, T value, CapabilityError error)
      : ok_(ok), value_(std::move(value)), error_(std::move(error)) {}
  bool ok_;
  T value_;
  CapabilityError error_;
};

template <>
class Result<void> {
 public:
  static Result Ok() { return Result(true, CapabilityError{}); }
  static Result Err(std::string code, std::string message,
                     std::vector<std::string> details = {}) {
    return Result(false,
                  CapabilityError{std::move(code), std::move(message), std::move(details)});
  }
  bool ok() const { return ok_; }
  const CapabilityError& error() const { return error_; }
  const std::string& errorCode() const { return error_.code; }
  const std::string& errorMessage() const { return error_.message; }
  const std::vector<std::string>& errorDetails() const { return error_.details; }

 private:
  Result(bool ok, CapabilityError error) : ok_(ok), error_(std::move(error)) {}
  bool ok_;
  CapabilityError error_;
};

}  // namespace machine_config::capabilities
