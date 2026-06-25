// PlatformCompat.swift - Cross-platform type aliases for iOS (UIKit) and macOS (AppKit).

#if os(macOS)
import AppKit
public typealias PlatformView = NSView
public typealias PlatformColor = NSColor
public typealias PlatformFont = NSFont
public typealias PlatformImage = NSImage
#else
import UIKit
public typealias PlatformView = UIView
public typealias PlatformColor = UIColor
public typealias PlatformFont = UIFont
public typealias PlatformImage = UIImage
#endif

extension PlatformView {
    func platformSetNeedsDisplay() {
        #if os(macOS)
        needsDisplay = true
        #else
        setNeedsDisplay()
        #endif
    }

    func platformSetNeedsLayout() {
        #if os(macOS)
        needsLayout = true
        #else
        setNeedsLayout()
        #endif
    }
}
