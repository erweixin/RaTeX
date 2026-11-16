Pod::Spec.new do |s|
  s.name             = 'ratex_flutter'
  s.version = '0.1.16'
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

  s.source_files     = 'ratex_flutter/Sources/ratex_flutter/**/*.swift'

  s.dependency 'Flutter'

  # Link the prebuilt xcframework — contains both device (arm64) and
  # simulator (arm64 + x86_64) slices.  CocoaPods copies the correct
  # slice at build time, so iOS Simulator "just works".
  # Keep the XCFramework inside the Swift package directory so Flutter's
  # generated SwiftPM symlink can resolve it; CocoaPods shares the same copy.
  s.vendored_frameworks = 'ratex_flutter/RaTeX.xcframework'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE'              => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
  }

  # Keep the FFI entry points reachable for DynamicLibrary.process() without
  # passing the generated libratex_ffi.a path as a direct Xcode build input.
  s.user_target_xcconfig = {
    'OTHER_LDFLAGS' => '-Wl,-u,_ratex_parse_and_layout -Wl,-u,_ratex_free_display_list -Wl,-u,_ratex_get_last_error -l"ratex_ffi"',
  }
end
