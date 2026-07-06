// RaTeXRNView.swift — ObjC-bridgeable wrapper around RaTeXView for React Native.

#if os(macOS)
import AppKit
#else
import UIKit
#endif

/// ObjC-compatible view wrapper around `RaTeXView` used as the React Native native view.
///
/// Exposes `@objc` properties so React Native can set them via KVC (old arch) or direct
/// property access from ObjC++ (new arch / Fabric).
@objc(RaTeXRNView)
@MainActor
public class RaTeXRNView: PlatformView {

    private let innerView = RaTeXView()
    private var bridgedColor: PlatformColor?

    // MARK: - ObjC-bridgeable properties

    @objc public var latex: String {
        get { innerView.latex }
        set {
            innerView.latex = newValue
            lastReportedContentSize = .zero
            innerView.invalidateIntrinsicContentSize()
            invalidateIntrinsicContentSize()
            platformSetNeedsLayout()
        }
    }

    @objc public var fontSize: CGFloat {
        get { innerView.fontSize }
        set {
            innerView.fontSize = newValue
            lastReportedContentSize = .zero
            innerView.invalidateIntrinsicContentSize()
            invalidateIntrinsicContentSize()
            platformSetNeedsLayout()
        }
    }

    /// `true` (default) = display/block style; `false` = inline/text style.
    @objc public var displayMode: Bool {
        get { innerView.displayMode }
        set {
            innerView.displayMode = newValue
            lastReportedContentSize = .zero
            innerView.invalidateIntrinsicContentSize()
            invalidateIntrinsicContentSize()
            platformSetNeedsLayout()
        }
    }

    @objc public var color: PlatformColor? {
        get { bridgedColor }
        set {
            let oldBridgedColor = bridgedColor
            let isSameValue = (newValue == nil && oldBridgedColor == nil)
                || (newValue != nil && oldBridgedColor != nil && newValue!.isEqual(oldBridgedColor!))
            guard !isSameValue else { return }

            bridgedColor = newValue
            innerView.color = newValue ?? .black
            lastReportedContentSize = .zero
            innerView.invalidateIntrinsicContentSize()
            invalidateIntrinsicContentSize()
            platformSetNeedsLayout()
        }
    }

    /// Old-arch event block set by React Native via KVC.
    /// When called, passes `{ "error": "<message>" }` as the body.
    @objc public var onError: ((NSDictionary?) -> Void)? {
        didSet {
            if let block = onError {
                innerView.onError = { error in
                    block(["error": error.localizedDescription])
                }
            } else {
                innerView.onError = nil
            }
        }
    }

    /// New-arch (Fabric) helper: lets ObjC++ install an error handler without
    /// needing to bridge the Swift `Error` type.
    @objc public func setErrorCallback(_ handler: @escaping (String) -> Void) {
        innerView.onError = { error in handler(error.localizedDescription) }
    }

    /// Old-arch: set by RN via KVC. Called with @{ @"width": @(w), @"height": @(h) }.
    @objc public var onContentSizeChange: ((NSDictionary?) -> Void)? {
        didSet {
            // Fast Refresh can remount JS without remounting the native view.
            // Reset so the next layout pass re-emits size to the new JS listener.
            lastReportedContentSize = .zero
            platformSetNeedsLayout()
        }
    }

    /// New-arch: set by ComponentView to dispatch content size events.
    @objc public func setContentSizeCallback(_ handler: ((CGFloat, CGFloat) -> Void)?) {
        contentSizeCallback = handler
        lastReportedContentSize = .zero
        platformSetNeedsLayout()
    }
    private var contentSizeCallback: ((CGFloat, CGFloat) -> Void)?

    /// Last size we reported to avoid duplicate events.
    private var lastReportedContentSize: CGSize = .zero

    /// Force the next layout pass to emit a content size event even if the size
    /// hasn't changed. This is important for Fast Refresh / remount scenarios
    /// where JS listeners are replaced but the native view instance is reused.
    @objc public func resetContentSizeReporting() {
        lastReportedContentSize = .zero
        platformSetNeedsLayout()
    }

    // MARK: - Init

    public override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    // MARK: - Layout

    #if os(macOS)
    public override var isFlipped: Bool { true }
    #endif

    public override var intrinsicContentSize: CGSize {
        innerView.intrinsicContentSize
    }

    #if os(macOS)
    public override func layout() {
        super.layout()
        performLayoutReporting()
    }
    #else
    public override func layoutSubviews() {
        super.layoutSubviews()
        performLayoutReporting()
    }
    #endif

    private func performLayoutReporting() {
        let size = innerView.intrinsicContentSize
        guard size.width > 0, size.height > 0 else { return }
        guard size != lastReportedContentSize else { return }
        lastReportedContentSize = size
        contentSizeCallback?(size.width, size.height)
        onContentSizeChange?(["width": size.width, "height": size.height])
    }

    // MARK: - Private

    /// The bundle containing KaTeX fonts bundled via CocoaPods resource_bundles.
    private static let fontsBundle: Bundle = {
        let module = Bundle(for: RaTeXRNView.self)
        if let url = module.url(forResource: "RaTeXFonts", withExtension: "bundle"),
           let bundle = Bundle(url: url) {
            return bundle
        }
        return module
    }()

    private static var fontsLoaded = false

    private func setup() {
        #if os(macOS)
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
        #else
        backgroundColor = .clear
        #endif
        addSubview(innerView)
        innerView.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            innerView.leadingAnchor.constraint(equalTo: leadingAnchor),
            innerView.trailingAnchor.constraint(equalTo: trailingAnchor),
            innerView.topAnchor.constraint(equalTo: topAnchor),
            innerView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
        // Load fonts from the CocoaPods resource bundle (not the main bundle or SPM bundle).
        // Guard ensures we only do this once across all RaTeXRNView instances.
        if !RaTeXRNView.fontsLoaded {
            RaTeXFontLoader.loadFromBundle(RaTeXRNView.fontsBundle)
            RaTeXRNView.fontsLoaded = true
        }
    }
}

/// Thread-safe synchronous LaTeX measurement.
///
/// Used by the Fabric shadow node's `measureContent` so the component self-sizes
/// during Yoga layout — making its size available at `useLayoutEffect` instead of
/// only after the async `onContentSizeChange` event. Safe to call off the main
/// thread: `RaTeXEngine.parse` is thread-safe (thread-local FFI error state) and
/// `RaTeXRenderer` is a value type; fonts are not needed for measurement.
@objc(RaTeXMeasure)
public final class RaTeXMeasure: NSObject {
    @objc public static func measureLatex(_ latex: String, fontSize: CGFloat, displayMode: Bool) -> CGSize {
        guard !latex.isEmpty, fontSize > 0 else { return .zero }
        do {
            let displayList = try RaTeXEngine.shared.parse(latex, displayMode: displayMode, color: .black)
            let renderer = RaTeXRenderer(displayList: displayList, fontSize: fontSize)
            return CGSize(width: renderer.width, height: renderer.totalHeight)
        } catch {
            return .zero
        }
    }
}
