#pragma once

#include <folly/dynamic.h>
#include <react/renderer/components/RNRaTeXSpec/Props.h>

namespace facebook::react {

// Serialize the props the Kotlin measure() needs. Color does not affect the
// measured size, but it IS part of the parse-cache key (the Rust engine bakes
// it into the DisplayList), so it must be forwarded for the entry produced by
// measure() to be reusable by the view's synchronous render.
inline folly::dynamic toDynamic(const RaTeXViewProps& props) {
  folly::dynamic serializedProps = folly::dynamic::object();
  serializedProps["latex"] = props.latex;
  serializedProps["fontSize"] = props.fontSize;
  serializedProps["displayMode"] = props.displayMode;
  if (props.color) {
    // On Android, Color is the ARGB int32 — same representation Kotlin uses.
    serializedProps["color"] = *props.color;
  }
  return serializedProps;
}

} // namespace facebook::react
