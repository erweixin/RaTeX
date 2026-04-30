Pod::Spec.new do |s|
  s.name             = 'ratex_flutter'
  s.version = '0.1.3'
  s.summary          = 'Flutter FFI bindings for RaTeX — native LaTeX math rendering (macOS).'
  s.description      = <<-DESC
    Provides a Flutter plugin that links the RaTeX dynamic library
    and exposes it to Dart FFI via DynamicLibrary.process().
    Bundles KaTeX fonts for glyph rendering via Flutter's ParagraphBuilder.
  DESC
  s.homepage         = 'https://github.com/erweixin/RaTeX'
  s.license          = { :type => 'MIT' }
  s.author           = { 'RaTeX' => 'https://github.com/erweixin/RaTeX' }
  s.source           = { :path => '.' }

  s.platform         = :osx, '11.0'
  s.swift_version    = '5.7'

  s.source_files     = 'Classes/**/*.swift'

  s.dependency 'FlutterMacOS'

  s.vendored_libraries = 'Libraries/libratex_ffi.dylib'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
  }
end
