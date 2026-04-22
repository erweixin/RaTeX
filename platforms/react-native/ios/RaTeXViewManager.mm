// RaTeXViewManager.mm — iOS bridge for RaTeXView (supports old arch & Fabric new arch).

#ifdef RCT_NEW_ARCH_ENABLED
#import <React/RCTComponentViewProtocol.h>
#import <React/RCTFabricComponentsPlugins.h>
#import <React/RCTViewComponentView.h>
// RN 0.76+ 常用预编译 React-Core：不再向 Pods 暴露 <React/Fabric/RCTConversions.h>。
// 使用 ReactCommon 的 Color API 做 SharedColor → UIColor，避免依赖已移除/未打包的头文件。
#import <react/renderer/graphics/Color.h>
#import <react/renderer/components/RNRaTeXSpec/ComponentDescriptors.h>
#import <react/renderer/components/RNRaTeXSpec/EventEmitters.h>
#import <react/renderer/components/RNRaTeXSpec/Props.h>
#import <react/renderer/components/RNRaTeXSpec/RCTComponentViewHelpers.h>
#else
#import "RaTeXViewManager.h"
#import <React/RCTUIManager.h>
#endif

// Swift-generated header (module name derived from podspec/target name)
#import "ratex_react_native-Swift.h"

// ---------------------------------------------------------------------------
// MARK: - New Architecture (Fabric)
// ---------------------------------------------------------------------------

#ifdef RCT_NEW_ARCH_ENABLED

using namespace facebook::react;

namespace {

/// Mirrors the old `RCTUIColorFromSharedColor` helper without importing RCTConversions.h
/// (unavailable when React is consumed as a prebuilt XCFramework).
inline UIColor *_Nullable RaTeXUIColorFromSharedColor(const SharedColor &sharedColor)
{
  if (!sharedColor) {
    return nil;
  }
  const ColorComponents components = (*sharedColor).getColorComponents();
  return [UIColor colorWithRed:components.red
                         green:components.green
                          blue:components.blue
                         alpha:components.alpha];
}

} // namespace

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
    _nativeView.autoresizingMask =
        UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;

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

  UIColor *color = RaTeXUIColorFromSharedColor(newProps.color);
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

- (UIView *)view
{
  return [[RaTeXRNView alloc] init];
}

RCT_EXPORT_VIEW_PROPERTY(latex, NSString)
RCT_EXPORT_VIEW_PROPERTY(fontSize, CGFloat)
RCT_EXPORT_VIEW_PROPERTY(displayMode, BOOL)
RCT_EXPORT_VIEW_PROPERTY(color, UIColor)
RCT_EXPORT_VIEW_PROPERTY(onError, RCTDirectEventBlock)
RCT_EXPORT_VIEW_PROPERTY(onContentSizeChange, RCTDirectEventBlock)

@end

#endif // RCT_NEW_ARCH_ENABLED
