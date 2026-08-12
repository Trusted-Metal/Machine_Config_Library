#pragma once
#include <string>
#include <utility>

namespace machine_config::capabilities {

template <typename T>
class Result {
 public:
  static Result Ok(T value) { return Result(true, std::move(value), {}); }
  static Result Err(std::string code, std::string message) {
    return Result(false, T{}, std::move(code), std::move(message));
  }

  bool ok() const { return ok_; }
  const T& value() const { return value_; }
  const std::string& errorCode() const { return code_; }
  const std::string& errorMessage() const { return message_; }

 private:
  Result(bool ok, T value, std::string code = {}, std::string message = {})
      : ok_(ok), value_(std::move(value)), code_(std::move(code)), message_(std::move(message)) {}
  bool ok_;
  T value_;
  std::string code_;
  std::string message_;
};

template <>
class Result<void> {
 public:
  static Result Ok() { return Result(true, {}, {}); }
  static Result Err(std::string code, std::string message) {
    return Result(false, std::move(code), std::move(message));
  }
  bool ok() const { return ok_; }
  const std::string& errorCode() const { return code_; }
  const std::string& errorMessage() const { return message_; }

 private:
  Result(bool ok, std::string code, std::string message)
      : ok_(ok), code_(std::move(code)), message_(std::move(message)) {}
  bool ok_;
  std::string code_;
  std::string message_;
};

}  // namespace machine_config::capabilities
