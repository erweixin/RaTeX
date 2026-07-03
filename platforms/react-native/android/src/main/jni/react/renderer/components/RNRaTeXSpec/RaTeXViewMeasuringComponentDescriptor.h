#pragma once

#include "RaTeXViewMeasurementManager.h"
#include "RaTeXViewMeasuringShadowNode.h"

#include <react/renderer/core/ConcreteComponentDescriptor.h>

namespace facebook::react {

// Component descriptor for the measurable <RaTeXView> shadow node. Creates one
// measurement manager (holding the contextContainer) and injects it into every
// shadow node via adopt().
class RaTeXViewMeasuringComponentDescriptor final
    : public ConcreteComponentDescriptor<RaTeXViewMeasuringShadowNode> {
 public:
  RaTeXViewMeasuringComponentDescriptor(
      const ComponentDescriptorParameters& parameters)
      : ConcreteComponentDescriptor(parameters),
        measurementsManager_(
            std::make_shared<RaTeXViewMeasurementManager>(contextContainer_)) {}

  void adopt(ShadowNode& shadowNode) const override {
    ConcreteComponentDescriptor::adopt(shadowNode);
    auto& node = static_cast<RaTeXViewMeasuringShadowNode&>(shadowNode);
    node.setMeasurementManager(measurementsManager_);
  }

 private:
  const std::shared_ptr<RaTeXViewMeasurementManager> measurementsManager_;
};

} // namespace facebook::react
