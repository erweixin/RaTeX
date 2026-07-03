// RaTeXViewManager.mm — Apple bridge for RaTeXView (supports old arch & Fabric new arch).

#ifdef RCT_NEW_ARCH_ENABLED
#import <React/RCTComponentViewProtocol.h>
#import <React/RCTFabricComponentsPlugins.h>
#import <React/RCTViewComponentView.h>
#import <react/renderer/components/RNRaTeXSpec/ComponentDescriptors.h>
#import <react/renderer/components/RNRaTeXSpec/EventEmitters.h>
#import <react/renderer/components/RNRaTeXSpec/Props.h>
#import <react/renderer/components/RNRaTeXSpec/RCTComponentViewHelpers.h>
#else
#import "RaTeXViewManager.h"
#import <React/RCTUIManager.h>
#endif

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

// ---------------------------------------------------------------------------
// MARK: - New Architecture (Fabric)
// ---------------------------------------------------------------------------

#ifdef RCT_NEW_ARCH_ENABLED

using namespace facebook::react;

// Class name follows RN Fabric convention: {ComponentName}ComponentView
// so that RCTThirdPartyComponentsProvider can resolve it via NSClassFromString.
@interface RaTeXViewComponentView : RCTViewComponentView
@end

@implementation RaTeXViewComponentView {
  RaTeXRNView *_nativeView;
}

+ (ComponentDescriptorProvider)componentDescriptorProvider
{
  return concreteComponentDescriptorProvider<RaTeXViewComponentDescriptor>();
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

// ---------------------------------------------------------------------------
// MARK: - Old Architecture (Bridge)
// ---------------------------------------------------------------------------

#else // !RCT_NEW_ARCH_ENABLED

@implementation RaTeXViewManager

RCT_EXPORT_MODULE(RaTeXView)

#if TARGET_OS_OSX
- (NSView *)view
#else
- (UIView *)view
#endif
{
  return [[RaTeXRNView alloc] init];
}

RCT_EXPORT_VIEW_PROPERTY(latex, NSString)
RCT_EXPORT_VIEW_PROPERTY(fontSize, CGFloat)
RCT_EXPORT_VIEW_PROPERTY(displayMode, BOOL)
#if TARGET_OS_OSX
RCT_EXPORT_VIEW_PROPERTY(color, NSColor)
#else
RCT_EXPORT_VIEW_PROPERTY(color, UIColor)
#endif
RCT_EXPORT_VIEW_PROPERTY(onError, RCTDirectEventBlock)
RCT_EXPORT_VIEW_PROPERTY(onContentSizeChange, RCTDirectEventBlock)

@end

#endif // RCT_NEW_ARCH_ENABLED
