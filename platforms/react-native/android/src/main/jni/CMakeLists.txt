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

target_link_libraries(
        ${LIB_TARGET_NAME}
        fbjni
        jsi
        reactnative
)

target_compile_reactnative_options(${LIB_TARGET_NAME} PRIVATE)
