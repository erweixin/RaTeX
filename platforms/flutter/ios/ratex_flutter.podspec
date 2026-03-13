Pod::Spec.new do |s|
  s.name             = 'ratex_flutter'
  s.version          = '0.0.3'
  s.summary          = 'Flutter FFI bindings for RaTeX — native LaTeX math rendering.'
  s.description      = <<-DESC
    Provides a Flutter plugin that links the RaTeX static library (xcframework)
    and exposes it to Dart FFI via DynamicLibrary.process().
    Bundles KaTeX fonts for glyph rendering via Flutter's ParagraphBuilder.
  DESC
  s.homepage         = 'https://github.com/erweixin/RaTeX'
  s.license          = { :type => 'MIT' }
  s.author           = { 'RaTeX' => 'https://github.com/erweixin/RaTeX' }
  s.source           = { :path => '.' }

  s.platform         = :ios, '13.0'
  s.swift_version    = '5.7'

  s.source_files     = 'Classes/**/*.swift'

  s.dependency 'Flutter'

  # Link the prebuilt xcframework — contains both device (arm64) and
  # simulator (arm64 + x86_64) slices.  CocoaPods copies the correct
  # slice at build time, so iOS Simulator "just works".
  # The xcframework lives alongside this podspec so that Flutter's
  # pub get copy to .symlinks/plugins/ carries it along automatically.
  s.vendored_frameworks = 'RaTeX.xcframework'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE'              => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
  }

  # Force-load the static library into the app binary so that
  # DynamicLibrary.process() / dlsym(RTLD_DEFAULT, ...) can find the
  # ratex_parse_and_layout symbol at runtime.  Without this the linker
  # dead-strips the unreferenced C symbols before Dart FFI can look them up.
  s.user_target_xcconfig = {
    'OTHER_LDFLAGS' => '-force_load ${PODS_XCFRAMEWORKS_BUILD_DIR}/ratex_flutter/libratex_ffi.a',
  }
end
