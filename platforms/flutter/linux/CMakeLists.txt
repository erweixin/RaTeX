# RaTeX plugin for Linux — all rendering via Dart FFI.
# The plugin .so is a registration stub only; the real work happens via
# libratex_ffi.so which is bundled alongside the plugin.
cmake_minimum_required(VERSION 3.14)
project(ratex_flutter LANGUAGES CXX)

set(PLUGIN_NAME "ratex_flutter_plugin")

list(APPEND PLUGIN_SOURCES
  "ratex_flutter_plugin.cc"
)

add_library(${PLUGIN_NAME} SHARED
  ${PLUGIN_SOURCES}
)

apply_standard_settings(${PLUGIN_NAME})
set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden)
target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)
target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_link_libraries(${PLUGIN_NAME} PRIVATE flutter flutter_wrapper_plugin)

# Copy the arch-specific libratex_ffi.so next to the plugin .so.
# ${CMAKE_SYSTEM_PROCESSOR} returns "x86_64" or "aarch64" on Linux.
add_custom_command(TARGET ${PLUGIN_NAME} POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_if_different
    "${CMAKE_CURRENT_SOURCE_DIR}/lib/${CMAKE_SYSTEM_PROCESSOR}/libratex_ffi.so"
    "$<TARGET_FILE_DIR:${PLUGIN_NAME}>/libratex_ffi.so"
)
