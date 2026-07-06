#pragma once

#include <react/renderer/components/RNRaTeXSpec/Props.h>
#include <react/renderer/core/LayoutConstraints.h>
#include <react/utils/ContextContainer.h>

namespace facebook::react {

// Bridges the shadow node's synchronous measureContent to the platform, by
// invoking FabricUIManager.measure (which routes to RaTeXViewManager.measure in
// Kotlin) over JNI. The FabricUIManager instance is retrieved from the
// contextContainer supplied to the component descriptor.
class RaTeXViewMeasurementManager {
 public:
  RaTeXViewMeasurementManager(
      const std::shared_ptr<const ContextContainer>& contextContainer)
      : contextContainer_(contextContainer) {}

  Size measure(
      SurfaceId surfaceId,
      LayoutConstraints layoutConstraints,
      const RaTeXViewProps& props) const;

 private:
  const std::shared_ptr<const ContextContainer> contextContainer_;
};

} // namespace facebook::react
