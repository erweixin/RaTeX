require "json"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name           = "ratex-react-native"
  s.version        = package["version"]
  s.summary        = package["description"]
  s.homepage       = "https://github.com/erweixin/RaTeX"
  s.license        = package["license"]
  s.authors        = { "erweixin" => "https://github.com/erweixin" }
  s.platforms      = { :ios => "14.0", :osx => "13.0" }
  s.source         = { :git => "https://github.com/erweixin/RaTeX.git", :tag => s.version.to_s }

  # Swift source files + ObjC++ bridge
  s.source_files   = "ios/**/*.{h,m,mm,swift}"
  # The vendored XCFramework ships a `ratex.h` inside every slice's Headers/ dir
  # (ios-arm64, ios-arm64_x86_64-simulator, macos-arm64_x86_64). Without this
  # exclude, the recursive source_files glob picks up all three copies and
  # CocoaPods emits three CpHeader build commands that resolve to the same output
  # path → Xcode 15+ fails the build with
  #   "Multiple commands produce '…/ratex-react-native/ratex.h'".
  # The framework's headers are exposed through its module (vendored_frameworks),
  # not through source_files, so excluding them here is safe.
  s.exclude_files  = "ios/RaTeX.xcframework/**/*"
  s.swift_version  = "5.9"

  # Prebuilt static library (libratex_ffi.a) packaged as XCFramework
  s.vendored_frameworks = "ios/RaTeX.xcframework"

  # KaTeX fonts — loaded at runtime from this bundle
  s.resource_bundles = {
    "RaTeXFonts" => ["ios/Fonts/*.ttf"]
  }

  # install_modules_dependencies wires up React Core / Fabric / Codegen
  # dependencies. RaTeX supports only the New Architecture (RN >= 0.84).
  install_modules_dependencies(s)
end
