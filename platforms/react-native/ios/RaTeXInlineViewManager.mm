// RaTeXInlineViewManager.mm — iOS bridge for RaTeXInlineView (old arch & Fabric).

#ifdef RCT_NEW_ARCH_ENABLED
#import <React/RCTComponentViewProtocol.h>
#import <React/RCTFabricComponentsPlugins.h>
#import <React/RCTViewComponentView.h>
#import <react/renderer/graphics/Color.h>
#import <react/renderer/components/RNRaTeXSpec/ComponentDescriptors.h>
#import <react/renderer/components/RNRaTeXSpec/EventEmitters.h>
#import <react/renderer/components/RNRaTeXSpec/Props.h>
#import <react/renderer/components/RNRaTeXSpec/RCTComponentViewHelpers.h>
#else
#import "RaTeXInlineViewManager.h"
#import <React/RCTUIManager.h>
#endif

#import "ratex_react_native-Swift.h"

// ---------------------------------------------------------------------------
// MARK: - New Architecture (Fabric)
// ---------------------------------------------------------------------------

#ifdef RCT_NEW_ARCH_ENABLED

using namespace facebook::react;

namespace {

inline UIColor *_Nullable RaTeXInlineUIColorFromSharedColor(const SharedColor &sharedColor)
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
    _nativeView.autoresizingMask =
        UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;

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

  UIColor *color = RaTeXInlineUIColorFromSharedColor(newProps.color);
  if ((color == nil) != (_nativeView.color == nil) ||
      (color != nil && ![color isEqual:_nativeView.color])) {
    _nativeView.color = color;
  }

  UIColor *textColor = RaTeXInlineUIColorFromSharedColor(newProps.textColor);
  if ((textColor == nil) != (_nativeView.textColor == nil) ||
      (textColor != nil && ![textColor isEqual:_nativeView.textColor])) {
    _nativeView.textColor = textColor;
  }

  CGFloat textFontSize = static_cast<CGFloat>(newProps.textFontSize);
  if (textFontSize > 0 && textFontSize != _nativeView.textFontSize) {
    _nativeView.textFontSize = textFontSize;
  }

  [super updateProps:props oldProps:oldProps];
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

- (UIView *)view
{
  return [[RaTeXInlineRNView alloc] init];
}

RCT_EXPORT_VIEW_PROPERTY(content, NSString)
RCT_EXPORT_VIEW_PROPERTY(fontSize, CGFloat)
RCT_EXPORT_VIEW_PROPERTY(color, UIColor)
RCT_EXPORT_VIEW_PROPERTY(textColor, UIColor)
RCT_EXPORT_VIEW_PROPERTY(textFontSize, CGFloat)
RCT_EXPORT_VIEW_PROPERTY(onContentSizeChange, RCTDirectEventBlock)

@end

#endif // RCT_NEW_ARCH_ENABLED
