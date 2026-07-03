/**
 * Hand-written override of the codegen ComponentDescriptors.h. It is placed
 * ahead of the generated one on the include path so that autolinking (which
 * registers `RaTeXViewComponentDescriptor`) and the codegen registration both
 * pick up the measurable descriptor for <RaTeXView>. <RaTeXInlineView> keeps the
 * default descriptor.
 */
#pragma once

#include <react/renderer/components/RNRaTeXSpec/ShadowNodes.h>
#include <react/renderer/componentregistry/ComponentDescriptorProviderRegistry.h>
#include <react/renderer/core/ConcreteComponentDescriptor.h>

#include "RaTeXViewMeasuringComponentDescriptor.h"

namespace facebook::react {

using RaTeXInlineViewComponentDescriptor =
    ConcreteComponentDescriptor<RaTeXInlineViewShadowNode>;
using RaTeXViewComponentDescriptor = RaTeXViewMeasuringComponentDescriptor;

void RNRaTeXSpec_registerComponentDescriptorsFromCodegen(
    std::shared_ptr<const ComponentDescriptorProviderRegistry> registry);

} // namespace facebook::react
