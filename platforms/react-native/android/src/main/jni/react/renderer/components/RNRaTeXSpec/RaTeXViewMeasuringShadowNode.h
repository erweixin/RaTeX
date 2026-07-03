#pragma once

#include "RaTeXViewMeasurementManager.h"

#include <react/renderer/components/RNRaTeXSpec/ShadowNodes.h>

namespace facebook::react {

// Measurable shadow node for <RaTeXView>. Subclasses the codegen shadow node and
// adds the Yoga measure traits + a synchronous measureContent, so the view has
// its real size on the first commit (e.g. at JS useLayoutEffect) instead of only
// after the async onContentSizeChange event. Mirrors the iOS measuring node.
class RaTeXViewMeasuringShadowNode final : public RaTeXViewShadowNode {
 public:
  using RaTeXViewShadowNode::RaTeXViewShadowNode;

  static ShadowNodeTraits BaseTraits() {
    auto traits = RaTeXViewShadowNode::BaseTraits();
    traits.set(ShadowNodeTraits::Trait::LeafYogaNode);
    traits.set(ShadowNodeTraits::Trait::MeasurableYogaNode);
    return traits;
  }

  void setMeasurementManager(
      const std::shared_ptr<RaTeXViewMeasurementManager>& measurementsManager);

  Size measureContent(
      const LayoutContext& layoutContext,
      const LayoutConstraints& layoutConstraints) const override;

 private:
  std::shared_ptr<RaTeXViewMeasurementManager> measurementsManager_;
};

} // namespace facebook::react
