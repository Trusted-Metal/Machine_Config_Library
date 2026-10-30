cmake_minimum_required(VERSION 3.20)
project(mcl_cpp_tarball_consumer CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# No add_subdirectory into the repo, no repo source in sight — this proves
# the *installed package* is self-sufficient, not the from-source app
# (that's what CppTarballConsumer is for vs. the existing Cpp/ folder).
find_package(MachineConfig REQUIRED)

add_executable(consumer_app src/main.cpp)
target_link_libraries(consumer_app PRIVATE MachineConfig::machine_config)
