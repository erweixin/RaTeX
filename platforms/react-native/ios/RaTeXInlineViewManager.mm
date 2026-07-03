// RaTeXInlineViewManager.mm — Apple bridge for RaTeXInlineView (old arch & Fabric).

#ifdef RCT_NEW_ARCH_ENABLED
#import <React/RCTComponentViewProtocol.h>
#import <React/RCTFabricComponentsPlugins.h>
#import <React/RCTViewComponentView.h>
#import <react/renderer/components/RNRaTeXSpec/ComponentDescriptors.h>
#import <react/renderer/components/RNRaTeXSpec/EventEmitters.h>
#import <react/renderer/components/RNRaTeXSpec/Props.h>
#import <react/renderer/components/RNRaTeXSpec/RCTComponentViewHelpers.h>
#else
#import "RaTeXInlineViewManager.h"
#import <React/RCTUIManager.h>
#endif

#if TARGET_OS_OSX
#import <AppKit/AppKit.h>
#else
#import <UIKit/UIKit.h>
#endif

// Prefer the framework/module form (forces the Swift module to build before this
// Objective-C++ TU, avoiding a non-deterministic "-Swift.h file not found" compile
// race on clean xcodebuild/EAS/CI builds). Fall back to the quote form when the
// module form does not resolve — i.e. when the library builds as a *static library*
// rather than a framework/clang module (`use_frameworks! :linkage => :static` /
// Expo `useFrameworks: 'static'`), where the generated header lands only in this
// target's own DerivedSources. Guard with __has_include so both linkages work.
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

@interface RaTeXInlineViewComponentView : RCTViewComponentView
@end

@implementation RaTeXInlineViewComponentView {
  RaTeXInlineRNView *_nativeView;
}

+ (ComponentDescriptorProvider)componentDescriptorProvider
{
  return concreteComponentDescriptorProvider<RaTeXInlineViewComponentDescriptor>();
}

- (instancetype)initWithFrame:(CGRect)frame
{
  if (self = [super initWithFrame:frame]) {
    static const auto defaultProps = std::make_shared<const RaTeXInlineViewProps>();
    _props = defaultProps;

    _nativeView = [[RaTeXInlineRNView alloc] initWithFrame:self.bounds];
#if TARGET_OS_OSX
    _nativeView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
#else
    _nativeView.autoresizingMask =
        UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
#endif

    __weak RaTeXInlineViewComponentView *weakSelf = self;
    [_nativeView setContentSizeCallback:^(CGFloat width, CGFloat height) {
      RaTeXInlineViewComponentView *strongSelf = weakSelf;
      if (!strongSelf || !strongSelf->_eventEmitter) return;
      auto emitter = std::dynamic_pointer_cast<const RaTeXInlineViewEventEmitter>(
          strongSelf->_eventEmitter);
      if (emitter) {
        RaTeXInlineViewEventEmitter::OnContentSizeChange event{
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
  const auto &newProps = *std::static_pointer_cast<const RaTeXInlineViewProps>(props);

  NSString *content = [NSString stringWithUTF8String:newProps.content.c_str()];
  if (![content isEqualToString:_nativeView.content]) {
    _nativeView.content = content;
  }

  CGFloat fontSize = static_cast<CGFloat>(newProps.fontSize);
  if (fontSize > 0 && fontSize != _nativeView.fontSize) {
    _nativeView.fontSize = fontSize;
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

#if TARGET_OS_OSX
  NSColor *textColor = RaTeXPlatformColorFromSharedColor(newProps.textColor);
#else
  UIColor *textColor = RaTeXPlatformColorFromSharedColor(newProps.textColor);
#endif
  if ((textColor == nil) != (_nativeView.textColor == nil) ||
      (textColor != nil && ![textColor isEqual:_nativeView.textColor])) {
    _nativeView.textColor = textColor;
  }

  CGFloat textFontSize = static_cast<CGFloat>(newProps.textFontSize);
  if (textFontSize > 0 && textFontSize != _nativeView.textFontSize) {
    _nativeView.textFontSize = textFontSize;
  }

  NSString *textFontFamily = nil;
  if (!newProps.textFontFamily.empty()) {
    textFontFamily =
        [NSString stringWithUTF8String:newProps.textFontFamily.c_str()];
  }
  if ((textFontFamily == nil) != (_nativeView.textFontFamily == nil) ||
      (textFontFamily != nil &&
       ![textFontFamily isEqualToString:_nativeView.textFontFamily])) {
    _nativeView.textFontFamily = textFontFamily;
  }

  if (newProps.textItalic != _nativeView.textItalic) {
    _nativeView.textItalic = newProps.textItalic;
  }

  if (newProps.textUnderline != _nativeView.textUnderline) {
    _nativeView.textUnderline = newProps.textUnderline;
  }

  if (newProps.textLineThrough != _nativeView.textLineThrough) {
    _nativeView.textLineThrough = newProps.textLineThrough;
  }

  [super updateProps:props oldProps:oldProps];
}

- (void)updateEventEmitter:(EventEmitter::Shared const &)eventEmitter
{
  [super updateEventEmitter:eventEmitter];
  if (_nativeView) {
    [_nativeView resetContentSizeReporting];
  }
}

@end

Class<RCTComponentViewProtocol> RaTeXInlineViewCls(void)
{
  return RaTeXInlineViewComponentView.class;
}

// ---------------------------------------------------------------------------
// MARK: - Old Architecture (Bridge)
// ---------------------------------------------------------------------------

#else // !RCT_NEW_ARCH_ENABLED

@implementation RaTeXInlineViewManager

RCT_EXPORT_MODULE(RaTeXInlineView)

#if TARGET_OS_OSX
- (NSView *)view
#else
- (UIView *)view
#endif
{
  return [[RaTeXInlineRNView alloc] init];
}

RCT_EXPORT_VIEW_PROPERTY(content, NSString)
RCT_EXPORT_VIEW_PROPERTY(fontSize, CGFloat)
#if TARGET_OS_OSX
RCT_EXPORT_VIEW_PROPERTY(color, NSColor)
RCT_EXPORT_VIEW_PROPERTY(textColor, NSColor)
#else
RCT_EXPORT_VIEW_PROPERTY(color, UIColor)
RCT_EXPORT_VIEW_PROPERTY(textColor, UIColor)
#endif
RCT_EXPORT_VIEW_PROPERTY(textFontSize, CGFloat)
RCT_EXPORT_VIEW_PROPERTY(textFontFamily, NSString)
RCT_EXPORT_VIEW_PROPERTY(textItalic, BOOL)
RCT_EXPORT_VIEW_PROPERTY(textUnderline, BOOL)
RCT_EXPORT_VIEW_PROPERTY(textLineThrough, BOOL)
RCT_EXPORT_VIEW_PROPERTY(onContentSizeChange, RCTDirectEventBlock)

@end

#endif // RCT_NEW_ARCH_ENABLED
