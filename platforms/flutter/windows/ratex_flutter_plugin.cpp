// ratex_flutter_plugin.cpp — Flutter plugin registration for RaTeX (Windows).
//
// All rendering is done via Dart FFI: DynamicLibrary.open("ratex_ffi.dll").
// The DLL is bundled from windows/ratex_ffi.dll (see CMakeLists.txt).
// No method channels or event channels.

#include "ratex_flutter/ratex_flutter_plugin.h"

#include <flutter/plugin_registrar_windows.h>

namespace {

class RatexFlutterPlugin : public flutter::Plugin {
 public:
  static void RegisterWithRegistrar(flutter::PluginRegistrarWindows* registrar) {
    // FFI-only plugin: no channels to register.
  }

  RatexFlutterPlugin() = default;
  virtual ~RatexFlutterPlugin() = default;
  RatexFlutterPlugin(const RatexFlutterPlugin&) = delete;
  RatexFlutterPlugin& operator=(const RatexFlutterPlugin&) = delete;
};

}  // namespace

void RatexFlutterPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  RatexFlutterPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
