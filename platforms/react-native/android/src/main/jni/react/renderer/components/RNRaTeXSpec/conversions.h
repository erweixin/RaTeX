#pragma once

#include <folly/dynamic.h>
#include <react/renderer/components/RNRaTeXSpec/Props.h>

namespace facebook::react {

// Serialize the props the Kotlin measure() needs. Color does not affect the
// measured size, so it is intentionally omitted.
inline folly::dynamic toDynamic(const RaTeXViewProps& props) {
  folly::dynamic serializedProps = folly::dynamic::object();
  serializedProps["latex"] = props.latex;
  serializedProps["fontSize"] = props.fontSize;
  serializedProps["displayMode"] = props.displayMode;
  return serializedProps;
}

} // namespace facebook::react
