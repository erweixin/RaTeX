version = File.read(File.join(__dir__, "VERSION")).strip

Pod::Spec.new do |s|
  s.name             = "RaTeXCore"
  s.version          = version
  s.summary          = "Core RaTeX engine and CoreGraphics renderer for Apple platforms"
  s.description      = <<-DESC
    A static, UI-free CocoaPods target for RaTeX. It includes the Swift display-list
    types, parsing engine, CoreGraphics renderer, KaTeX fonts, and the RaTeX FFI
    XCFramework without React Native, Fabric, Codegen, or view components.
  DESC
  s.homepage         = "https://github.com/erweixin/RaTeX"
  s.license          = { :type => "MIT", :file => "LICENSE" }
  s.author           = { "erweixin" => "https://github.com/erweixin" }
  s.source           = {
    :git => "https://github.com/erweixin/RaTeX.git",
    # RaTeXCore is not present in the current v#{s.version} tag yet. Keep the
    # development podspec aligned with the documented git-branch installation;
    # the first RaTeXCore release should switch this to :tag.
    :branch => "main"
  }

  s.ios.deployment_target = "14.0"
  s.osx.deployment_target = "14.0"
  s.swift_version         = "5.9"
  s.static_framework      = true

  s.source_files = [
    "platforms/ios/Sources/Ratex/DisplayList.swift",
    "platforms/ios/Sources/Ratex/RaTeXEngine.swift",
    "platforms/ios/Sources/Ratex/RaTeXFontLoader.swift",
    "platforms/ios/Sources/Ratex/RaTeXRenderer.swift",
  ]
  s.vendored_frameworks = "platforms/ios/RaTeX.xcframework"
  s.resource_bundles = {
    "RaTeXCoreFonts" => ["platforms/ios/Sources/Ratex/Fonts/*.ttf"]
  }
  s.frameworks = ["CoreGraphics", "CoreText", "Foundation"]

  # Release tags already publish the static XCFramework as an asset for SPM.
  # CocoaPods checks out the Swift sources from the same tag, then reuses that
  # exact binary instead of requiring consumers to build Rust locally.
  s.prepare_command = <<-CMD
    set -eu
    if [ ! -d platforms/ios/RaTeX.xcframework ]; then
      archive="platforms/ios/RaTeX.xcframework.zip"
      curl --fail --location --retry 3 \
        "https://github.com/erweixin/RaTeX/releases/download/v#{s.version}/RaTeX.xcframework.zip" \
        --output "$archive"
      unzip -q "$archive" -d platforms/ios
      rm "$archive"
    fi
  CMD
end
