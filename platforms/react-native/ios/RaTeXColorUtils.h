#pragma once

// C++-only helper (uses facebook::react types). The guard keeps this header
// inert when CocoaPods pulls it into non-C++ contexts (umbrella header /
// Swift module compilation), where the React C++ headers cannot be parsed.
#ifdef __cplusplus

#import <TargetConditionals.h>
#import <react/renderer/graphics/Color.h>

#if TARGET_OS_OSX
#import <AppKit/AppKit.h>
typedef NSColor RaTeXPlatformColor;
#else
#import <UIKit/UIKit.h>
typedef UIColor RaTeXPlatformColor;
#endif

inline RaTeXPlatformColor *_Nullable RaTeXPlatformColorFromSharedColor(
    const facebook::react::SharedColor &sharedColor)
{
  if (!sharedColor) {
    return nil;
  }

  const facebook::react::ColorComponents components = (*sharedColor).getColorComponents();

#if TARGET_OS_OSX
  return [NSColor colorWithSRGBRed:components.red
                             green:components.green
                              blue:components.blue
                             alpha:components.alpha];
#else
  return [UIColor colorWithRed:components.red
                         green:components.green
                          blue:components.blue
                         alpha:components.alpha];
#endif
}

#endif  // __cplusplus
