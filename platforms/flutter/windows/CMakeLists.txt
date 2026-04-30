# RaTeX plugin for Windows — all rendering via Dart FFI.
# The plugin DLL is a registration stub only; the real work happens via
# ratex_ffi.dll which is bundled alongside the plugin.
cmake_minimum_required(VERSION 3.14)
project(ratex_flutter LANGUAGES CXX)

set(PLUGIN_NAME "ratex_flutter_plugin")

list(APPEND PLUGIN_SOURCES
  "ratex_flutter_plugin.cpp"
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

# Copy ratex_ffi.dll next to the plugin DLL so DynamicLibrary.open() finds it.
add_custom_command(TARGET ${PLUGIN_NAME} POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_if_different
    "${CMAKE_CURRENT_SOURCE_DIR}/ratex_ffi.dll"
    "$<TARGET_FILE_DIR:${PLUGIN_NAME}>/ratex_ffi.dll"
)
