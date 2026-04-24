// ratex_flutter_plugin.h — Public header for RaTeX Flutter plugin (Linux).
#ifndef FLUTTER_PLUGIN_RATEX_FLUTTER_PLUGIN_H_
#define FLUTTER_PLUGIN_RATEX_FLUTTER_PLUGIN_H_

#include <flutter_linux/flutter_linux.h>

G_BEGIN_DECLS

#ifdef FLUTTER_PLUGIN_IMPL
#define FLUTTER_PLUGIN_EXPORT __attribute__((visibility("default")))
#else
#define FLUTTER_PLUGIN_EXPORT
#endif

G_DECLARE_FINAL_TYPE(RatexFlutterPlugin, ratex_flutter_plugin,
                     RATEX, FLUTTER_PLUGIN, GObject)

FLUTTER_PLUGIN_EXPORT void ratex_flutter_plugin_register_with_registrar(
    FlPluginRegistrar* registrar);

G_END_DECLS

#endif  // FLUTTER_PLUGIN_RATEX_FLUTTER_PLUGIN_H_
