// ratex_flutter_plugin.cc — Flutter plugin registration for RaTeX (Linux).
//
// All rendering is done via Dart FFI: DynamicLibrary.open("libratex_ffi.so").
// The .so is bundled from linux/lib/<arch>/ (see CMakeLists.txt).
// No method channels or event channels.

#include "ratex_flutter/ratex_flutter_plugin.h"

#include <flutter_linux/flutter_linux.h>

struct _RatexFlutterPlugin {
  GObject parent_instance;
};

G_DEFINE_TYPE(RatexFlutterPlugin, ratex_flutter_plugin, g_object_get_type())

static void ratex_flutter_plugin_dispose(GObject* object) {
  G_OBJECT_CLASS(ratex_flutter_plugin_parent_class)->dispose(object);
}

static void ratex_flutter_plugin_class_init(RatexFlutterPluginClass* klass) {
  G_OBJECT_CLASS(klass)->dispose = ratex_flutter_plugin_dispose;
}

static void ratex_flutter_plugin_init(RatexFlutterPlugin* self) {}

void ratex_flutter_plugin_register_with_registrar(FlPluginRegistrar* registrar) {
  RatexFlutterPlugin* plugin = RATEX_FLUTTER_PLUGIN(
      g_object_new(ratex_flutter_plugin_get_type(), nullptr));
  g_object_unref(plugin);
}
