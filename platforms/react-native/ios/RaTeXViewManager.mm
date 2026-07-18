// RaTeXViewManager.mm — Apple bridge for RaTeXView (Fabric / New Architecture).

#import <React/RCTComponentViewProtocol.h>
#import <React/RCTFabricComponentsPlugins.h>
#import <React/RCTViewComponentView.h>
#import <react/renderer/components/RNRaTeXSpec/ComponentDescriptors.h>
#import <react/renderer/components/RNRaTeXSpec/EventEmitters.h>
#import <react/renderer/components/RNRaTeXSpec/Props.h>
#import <react/renderer/components/RNRaTeXSpec/RCTComponentViewHelpers.h>
#import <react/renderer/core/LayoutConstraints.h>
#import <react/renderer/core/LayoutContext.h>
#include <algorithm>
#include <cmath>

#if TARGET_OS_OSX
#import <AppKit/AppKit.h>
#else
#import <UIKit/UIKit.h>
#endif

// Swift-generated header (module name derived from podspec/target name).
// Prefer the framework/module form (<module/module-Swift.h>) when it resolves: it
// creates a module dependency that forces the Swift target to build BEFORE this
// Objective-C++ TU, avoiding a non-deterministic build race that fails `xcodebuild`
// (and EAS/CI) with "ratex_react_native-Swift.h file not found". But when the
// library is built as a *static library* rather than a framework/clang module —
// e.g. `use_frameworks! :linkage => :static` (Expo's `useFrameworks: 'static'`,
// required to statically link some RN pods such as RNFirebase) — the generated
// header is emitted only into this target's own DerivedSources and the
// angle/module form no longer resolves. Guard with __has_include so framework
// builds keep the race-avoidance and static-library builds fall back to the quote
// form (which resolves against DerivedSources). Linkage-agnostic.
#if __has_include(<ratex_react_native/ratex_react_native-Swift.h>)
#import <ratex_react_native/ratex_react_native-Swift.h>
#else
#import "ratex_react_native-Swift.h"
#endif
#import "RaTeXColorUtils.h"

using namespace facebook::react;

namespace {

// A ShadowNode that measures the formula synchronously during Yoga layout, using
// the (thread-safe) RaTeX engine. This makes the view's size available on the very
// first commit — i.e. at JS `useLayoutEffect` — instead of only after the async
// `onContentSizeChange` event, which otherwise causes a one-frame 0-size flash and
// breaks synchronous reserve-then-place layout (e.g. TextKit attachments).
class RaTeXViewMeasuringShadowNode final : public RaTeXViewShadowNode {
 public:
  using RaTeXViewShadowNode::RaTeXViewShadowNode;

  static ShadowNodeTraits BaseTraits() {
    auto traits = RaTeXViewShadowNode::BaseTraits();
    traits.set(ShadowNodeTraits::Trait::LeafYogaNode);
    traits.set(ShadowNodeTraits::Trait::MeasurableYogaNode);
    traits.set(ShadowNodeTraits::Trait::BaselineYogaNode);
    return traits;
  }

  facebook::react::Size measureContent(const LayoutContext &layoutContext,
                                       const LayoutConstraints &layoutConstraints) const override {
    const auto &props = getConcreteProps();
    if (props.latex.empty() || props.fontSize <= 0) {
      return layoutConstraints.clamp(facebook::react::Size{0, 0});
    }
    NSString *latex = [NSString stringWithUTF8String:props.latex.c_str()];
    if (latex == nil) {
      return layoutConstraints.clamp(facebook::react::Size{0, 0});
    }
    RaTeXTexMetrics *metrics = [RaTeXMeasure metricsLatex:latex
                                                 fontSize:static_cast<CGFloat>(props.fontSize)
                                              displayMode:props.displayMode ? YES : NO];
    if (metrics == nil) {
      return layoutConstraints.clamp(facebook::react::Size{0, 0});
    }
    Float measuredHeight = static_cast<Float>(metrics.height);
    // Inside <Text> (inlineAlign is only ever set there) the text engine pins the
    // view bottom to the baseline and reserves the whole height as line ASCENT.
    // Report the ascent-only box (height − depth) so the line above gets the same
    // spacing a baseline-aligned flex row produces; the view draws its natural-size
    // ink anchored to the bottom, descender overflowing (RaTeXRNView).
    if (props.inlineAlign == RaTeXViewInlineAlign::Baseline) {
      measuredHeight -= static_cast<Float>(metrics.depth);
    }
    // Snap up to the pixel grid so Yoga's position-dependent edge rounding can't vary the
    // reported height across placements of the same formula. Uses Yoga's own scale factor.
    Float scale = layoutContext.pointScaleFactor > 0 ? layoutContext.pointScaleFactor : 1;
    Float width = std::ceil(static_cast<Float>(metrics.width) * scale) / scale;
    Float height = std::ceil(measuredHeight * scale) / scale;
    facebook::react::Size size{width, height};
    return layoutConstraints.clamp(size);
  }

  // The drawn formula's alphabetic baseline, so `alignItems: 'baseline'` lines the
  // formula up with sibling <Text> exactly like a glyph. Mirrors the view's
  // fit-scale/centering draw math against the engine's natural metrics.
  Float baseline(const LayoutContext &layoutContext, facebook::react::Size size) const override {
    const auto &props = getConcreteProps();
    // Yoga's default for a node without a real baseline is its bottom edge.
    const Float fallback = size.height;
    if (props.latex.empty() || props.fontSize <= 0) {
      return fallback;
    }
    NSString *latex = [NSString stringWithUTF8String:props.latex.c_str()];
    if (latex == nil) {
      return fallback;
    }
    RaTeXTexMetrics *metrics = [RaTeXMeasure metricsLatex:latex
                                                 fontSize:static_cast<CGFloat>(props.fontSize)
                                              displayMode:props.displayMode ? YES : NO];
    if (metrics == nil) {
      return fallback;
    }
    const Float naturalWidth = static_cast<Float>(metrics.width);
    const Float naturalHeight = static_cast<Float>(metrics.height);
    const Float depth = static_cast<Float>(metrics.depth);
    if (naturalWidth <= 0 || naturalHeight <= 0) {
      return fallback;
    }
    const Float scale = std::min(
        Float(1), std::min(size.width / naturalWidth, size.height / naturalHeight));
    const Float dy = std::max(Float(0), (size.height - naturalHeight * scale) / 2);
    // Exact drawn ink baseline, snapped to the PIXEL grid (whole-point flooring
    // made the raise vary 0..1pt per formula), +1px uniform optical raise
    // (a larger reported baseline moves the child up). RaTeXRNView.inlineShift
    // applies the same rule inside <Text>.
    const Float pixelScale =
        layoutContext.pointScaleFactor > 0 ? layoutContext.pointScaleFactor : 1;
    const Float inkBaseline = dy + naturalHeight * scale - depth * scale;
    return std::round(inkBaseline * pixelScale) / pixelScale + 1 / pixelScale;
  }
};

using RaTeXViewMeasuringComponentDescriptor =
    ConcreteComponentDescriptor<RaTeXViewMeasuringShadowNode>;

}  // namespace

// Class name follows RN Fabric convention: {ComponentName}ComponentView
// so that RCTThirdPartyComponentsProvider can resolve it via NSClassFromString.
@interface RaTeXViewComponentView : RCTViewComponentView
@end

@implementation RaTeXViewComponentView {
  RaTeXRNView *_nativeView;
}

+ (ComponentDescriptorProvider)componentDescriptorProvider
{
  return concreteComponentDescriptorProvider<RaTeXViewMeasuringComponentDescriptor>();
}

- (instancetype)initWithFrame:(CGRect)frame
{
  if (self = [super initWithFrame:frame]) {
    static const auto defaultProps = std::make_shared<const RaTeXViewProps>();
    _props = defaultProps;

    _nativeView = [[RaTeXRNView alloc] initWithFrame:self.bounds];
#if TARGET_OS_OSX
    _nativeView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
#else
    _nativeView.autoresizingMask =
        UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
#endif

    __weak RaTeXViewComponentView *weakSelf = self;
    [_nativeView setErrorCallback:^(NSString *errorMsg) {
      RaTeXViewComponentView *strongSelf = weakSelf;
      if (!strongSelf || !strongSelf->_eventEmitter) return;
      auto emitter = std::dynamic_pointer_cast<const RaTeXViewEventEmitter>(
          strongSelf->_eventEmitter);
      if (emitter) {
        RaTeXViewEventEmitter::OnError event{
            .error = std::string(errorMsg.UTF8String ?: "")};
        emitter->onError(event);
      }
    }];
    [_nativeView setContentSizeCallback:^(CGFloat width, CGFloat height) {
      RaTeXViewComponentView *strongSelf = weakSelf;
      if (!strongSelf || !strongSelf->_eventEmitter) return;
      auto emitter = std::dynamic_pointer_cast<const RaTeXViewEventEmitter>(
          strongSelf->_eventEmitter);
      if (emitter) {
        RaTeXViewEventEmitter::OnContentSizeChange event{
            .width = static_cast<Float>(width), .height = static_cast<Float>(height)};
        emitter->onContentSizeChange(event);
      }
    }];

    self.contentView = _nativeView;
  }
  return self;
}

- (void)updateProps:(Props::Shared const &)props
           oldProps:(Props::Shared const &)oldProps
{
  const auto &newProps = *std::static_pointer_cast<const RaTeXViewProps>(props);

  NSString *latex = [NSString stringWithUTF8String:newProps.latex.c_str()];
  if (![latex isEqualToString:_nativeView.latex]) {
    _nativeView.latex = latex;
  }

  CGFloat fontSize = static_cast<CGFloat>(newProps.fontSize);
  if (fontSize > 0 && fontSize != _nativeView.fontSize) {
    _nativeView.fontSize = fontSize;
  }

  BOOL displayMode = newProps.displayMode ? YES : NO;
  if (displayMode != _nativeView.displayMode) {
    _nativeView.displayMode = displayMode;
  }

  // Derived from style.alignSelf by RaTeXView.tsx; drives the native in-<Text>
  // vertical alignment shift (see RaTeXRNView.inlineAlign — applied only when
  // the view detects a text host, so flex-sibling alignment stays pure Yoga).
  NSString *inlineAlign =
      [NSString stringWithUTF8String:toString(newProps.inlineAlign).c_str()];
  if (![inlineAlign isEqualToString:_nativeView.inlineAlign]) {
    _nativeView.inlineAlign = inlineAlign;
  }

#if TARGET_OS_OSX
  NSColor *color = RaTeXPlatformColorFromSharedColor(newProps.color);
#else
  UIColor *color = RaTeXPlatformColorFromSharedColor(newProps.color);
#endif
  if ((color == nil) != (_nativeView.color == nil) ||
      (color != nil && ![color isEqual:_nativeView.color])) {
    _nativeView.color = color;
  }

  [super updateProps:props oldProps:oldProps];
}

// When JS remounts (e.g. Fast Refresh or key changes), Fabric can reuse the same
// native view instance but swap the EventEmitter. If props don't change, the
// view would not re-emit content size, causing JS-side auto-sizing to get stuck.
- (void)updateEventEmitter:(EventEmitter::Shared const &)eventEmitter
{
  [super updateEventEmitter:eventEmitter];
  if (_nativeView) {
    [_nativeView resetContentSizeReporting];
  }
}

@end

Class<RCTComponentViewProtocol> RaTeXViewCls(void)
{
  return RaTeXViewComponentView.class;
}
