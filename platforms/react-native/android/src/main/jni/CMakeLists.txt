cmake_minimum_required(VERSION 3.13)
set(CMAKE_VERBOSE_MAKEFILE on)

set(LIB_LITERAL RNRaTeXSpec)
set(LIB_TARGET_NAME react_codegen_${LIB_LITERAL})

set(LIB_ANDROID_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../..)
set(LIB_ANDROID_GENERATED_JNI_DIR ${LIB_ANDROID_DIR}/build/generated/source/codegen/jni)
set(LIB_ANDROID_GENERATED_COMPONENTS_DIR ${LIB_ANDROID_GENERATED_JNI_DIR}/react/renderer/components/${LIB_LITERAL})

file(GLOB LIB_MODULE_SRCS CONFIGURE_DEPENDS *.cpp react/renderer/components/${LIB_LITERAL}/*.cpp)
file(GLOB LIB_CODEGEN_SRCS CONFIGURE_DEPENDS ${LIB_ANDROID_GENERATED_COMPONENTS_DIR}/*.cpp ${LIB_ANDROID_GENERATED_JNI_DIR}/*.cpp)

add_library(
        ${LIB_TARGET_NAME}
        SHARED
        ${LIB_MODULE_SRCS}
        ${LIB_CODEGEN_SRCS}
)

target_include_directories(
        ${LIB_TARGET_NAME}
        PUBLIC
        .
        ./react/renderer/components/${LIB_LITERAL}
        ${LIB_ANDROID_GENERATED_JNI_DIR}
        ${LIB_ANDROID_GENERATED_COMPONENTS_DIR}
)

# https://github.com/react-native-community/discussions-and-proposals/discussions/816
# This if-then-else can be removed once this library does not support versions below 0.76
if(REACTNATIVE_MERGED_SO)
  target_link_libraries(
          ${LIB_TARGET_NAME}
          fbjni
          jsi
          reactnative
  )
else()
  target_link_libraries(
          ${LIB_TARGET_NAME}
          fbjni
          folly_runtime
          glog
          jsi
          react_codegen_rncore
          react_debug
          react_nativemodule_core
          react_render_core
          react_render_debug
          react_render_graphics
          react_render_mapbuffer
          react_render_componentregistry
          react_utils
          rrc_view
          turbomodulejsijni
          yoga
  )
endif()

# target_compile_reactnative_options only exists in React Native >= 0.80
if(ReactAndroid_VERSION_MINOR GREATER_EQUAL 80)
  target_compile_reactnative_options(${LIB_TARGET_NAME} PRIVATE)
else()
  target_compile_options(
          ${LIB_TARGET_NAME}
          PRIVATE
          -fexceptions
          -frtti
          -std=c++20
          -Wall
  )
endif()
