// RaTeXModule.mm — sync TeX metrics for JS (TurboModule). Sync by design
// (callers need it in useLayoutEffect); backed by RaTeXMeasure's parse cache.

#import <React/RCTBridgeModule.h>
#import <RNRaTeXSpec/RNRaTeXSpec.h>

// Swift-generated header — same framework/static-library dance as RaTeXViewManager.mm.
#if __has_include(<ratex_react_native/ratex_react_native-Swift.h>)
#import <ratex_react_native/ratex_react_native-Swift.h>
#else
#import "ratex_react_native-Swift.h"
#endif

@interface RaTeXModule : NSObject <RCTBridgeModule, NativeRaTeXModuleSpec>
@end

@implementation RaTeXModule

RCT_EXPORT_MODULE(RaTeXModule)

+ (BOOL)requiresMainQueueSetup
{
  return NO;
}

- (NSDictionary *_Nullable)texMetricsForLatex:(NSString *)latex
                                     fontSize:(double)fontSize
                                  displayMode:(BOOL)displayMode
{
  RaTeXTexMetrics *metrics = [RaTeXMeasure metricsLatex:latex
                                                fontSize:(CGFloat)fontSize
                                             displayMode:displayMode];
  if (metrics == nil) {
    return nil;
  }
  return @{
    @"width" : @(metrics.width),
    @"height" : @(metrics.height),
    @"depth" : @(metrics.depth),
  };
}

// `color` exists for Android's color-keyed cache; iOS metrics are color-blind.
- (NSDictionary *_Nullable)getTexMetrics:(NSString *)latex
                                fontSize:(double)fontSize
                             displayMode:(BOOL)displayMode
                                   color:(double)color
{
  return [self texMetricsForLatex:latex fontSize:fontSize displayMode:displayMode];
}

- (std::shared_ptr<facebook::react::TurboModule>)getTurboModule:
    (const facebook::react::ObjCTurboModule::InitParams &)params
{
  return std::make_shared<facebook::react::NativeRaTeXModuleSpecJSI>(params);
}

@end
