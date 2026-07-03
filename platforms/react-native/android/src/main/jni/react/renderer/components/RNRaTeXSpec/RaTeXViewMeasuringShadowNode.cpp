#include "RaTeXViewMeasuringShadowNode.h"

#include <react/renderer/core/LayoutContext.h>

namespace facebook::react {

void RaTeXViewMeasuringShadowNode::setMeasurementManager(
    const std::shared_ptr<RaTeXViewMeasurementManager>& measurementsManager) {
  ensureUnsealed();
  measurementsManager_ = measurementsManager;
}

Size RaTeXViewMeasuringShadowNode::measureContent(
    const LayoutContext& layoutContext,
    const LayoutConstraints& layoutConstraints) const {
  return measurementsManager_->measure(
      getSurfaceId(), layoutConstraints, getConcreteProps());
}

} // namespace facebook::react
