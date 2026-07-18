// RaTeXRNView.swift — ObjC-bridgeable wrapper around RaTeXView for React Native.

#if os(macOS)
import AppKit
#else
import UIKit
#endif

/// ObjC-compatible view wrapper around `RaTeXView` used as the React Native native view.
///
/// Exposes `@objc` properties for direct property access from ObjC++ (Fabric).
@objc(RaTeXRNView)
@MainActor
public class RaTeXRNView: PlatformView {

    private let innerView = RaTeXView()
    private var bridgedColor: PlatformColor?
    private var innerTopConstraint: NSLayoutConstraint?
    private var innerBottomConstraint: NSLayoutConstraint?

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

    /// Vertical alignment inside a host `<Text>` (which pins the view bottom to
    /// the text baseline and ignores alignSelf). Set by RaTeXView.tsx from
    /// `style.alignSelf`, and only under a <Text> ancestor — in a flex row Yoga
    /// aligns via the shadow node's baseline() and this stays "none".
    @objc public var inlineAlign: String = "none" {
        didSet {
            guard inlineAlign != oldValue else { return }
            platformSetNeedsLayout()
        }
    }

    #if !os(macOS)
    /// Stock RN bug: the NSTextAttachment run has no font attribute, so
    /// RCTTextLayoutManager's placement uses the 12pt default font's descender
    /// and every inline view sinks `hostDescent − |systemFont(12).descender|`
    /// below the baseline. Reconstruct that error from the hosting paragraph's
    /// `attributedText` (public introspection API): our run's font (nil on
    /// stock RN) vs the nearest neighboring run's font (the line's real font).
    /// On a fixed RN the run carries the real font and this returns ~0.
    private func stockAttachmentSink() -> CGFloat {
        let defaultRunDescent = -UIFont.systemFont(ofSize: 12).descender
        // Fallback: assume host text is system font at our size.
        let contractSink =
            -UIFont.systemFont(ofSize: innerView.fontSize).descender - defaultRunDescent
        var child: UIView = self
        var paragraph: UIView?
        while let parent = child.superview {
            if NSStringFromClass(type(of: parent)).contains("ParagraphComponentView") {
                paragraph = parent
                break
            }
            child = parent
        }
        guard let paragraph, paragraph.responds(to: NSSelectorFromString("attributedText")),
            let text = paragraph.value(forKey: "attributedText") as? NSAttributedString,
            text.length > 0
        else { return contractSink }
        // Our run = k-th attachment, k = our ordinal among the paragraph's
        // (attachment-only) subviews.
        let ordinal = paragraph.subviews.firstIndex(of: child) ?? 0
        var runRange = NSRange(location: NSNotFound, length: 0)
        var runFont: UIFont?
        var seen = 0
        text.enumerateAttribute(
            .attachment, in: NSRange(location: 0, length: text.length)
        ) { value, range, stop in
            guard value is NSTextAttachment else { return }
            if seen == ordinal {
                runRange = range
                runFont = text.attribute(.font, at: range.location, effectiveRange: nil) as? UIFont
                stop.pointee = true
            }
            seen += 1
        }
        guard runRange.location != NSNotFound else { return contractSink }
        // Nearest run with a font (skips adjacent font-less attachment runs).
        var hostFont: UIFont?
        var i = runRange.location - 1
        while i >= 0 {
            var eff = NSRange()
            if let f = text.attribute(.font, at: i, effectiveRange: &eff) as? UIFont {
                hostFont = f
                break
            }
            i = eff.location - 1
        }
        if hostFont == nil {
            var j = NSMaxRange(runRange)
            while j < text.length {
                var eff = NSRange()
                if let f = text.attribute(.font, at: j, effectiveRange: &eff) as? UIFont {
                    hostFont = f
                    break
                }
                j = NSMaxRange(eff)
            }
        }
        guard let hostFont else { return contractSink }
        return -hostFont.descender - (runFont.map { -$0.descender } ?? defaultRunDescent)
    }
    #endif

    /// Inner-view offsets against the host text's bottom-on-baseline placement,
    /// in points, pixel-snapped. "baseline": the wrapper is the ASCENT-ONLY box
    /// (the shadow node measures height − depth), so the natural-size inner view
    /// anchors its ink baseline to the wrapper bottom and its descender overflows
    /// below (top ≠ bottom constant — the inner is taller than the wrapper).
    /// center/start/end keep the full-height wrapper and a pure translate,
    /// assuming host text at our fontSize (math axis 0.25em, line box
    /// 0.75/0.25em em fractions).
    private var inlineOffsets: (top: CGFloat, bottom: CGFloat) {
        guard inlineAlign != "none",
            bounds.width > 0, bounds.height > 0,
            let metrics = RaTeXMeasure.metricsLatex(
                innerView.latex, fontSize: innerView.fontSize, displayMode: innerView.displayMode),
            metrics.width > 0, metrics.height > 0
        else { return (0, 0) }
        #if os(macOS)
        let scale = window?.backingScaleFactor ?? 2
        let sink: CGFloat = 0
        #else
        let sink = stockAttachmentSink()
        let scale = window?.screen.scale ?? UIScreen.main.scale
        #endif
        if inlineAlign == "baseline" {
            // Height no longer constrains the fit — it was derived from the ink.
            let k = min(1, bounds.width / metrics.width)
            var top = bounds.height - (metrics.height - metrics.depth) * k - sink
            // −2 physical px, measured: −1 residual TextKit sub-pixel bias
            // (always low, never high), −1 optical raise matching baseline().
            top = (top * scale).rounded() / scale - 2 / scale
            // Inner keeps its natural (scaled) height; descender overflows below.
            let bottom = top + metrics.height * k - bounds.height
            return (top, bottom)
        }
        let k = min(1, min(bounds.width / metrics.width, bounds.height / metrics.height))
        let g = max(0, (bounds.height - metrics.height * k) / 2)
        let inkHeight = metrics.height * k
        let em = innerView.fontSize
        var shift: CGFloat
        switch inlineAlign {
        case "center":
            shift = g + inkHeight / 2 - 0.25 * em
        case "start":
            shift = g + inkHeight - 0.75 * em
        case "end":
            shift = g + 0.25 * em
        default:
            return (0, 0)
        }
        shift = ((shift - sink) * scale).rounded() / scale - 2 / scale
        return (shift, shift)
    }

    private func updateInlineShift() {
        let offsets = inlineOffsets
        // Runs from the layout pass — the guard prevents a re-layout loop.
        guard innerTopConstraint?.constant != offsets.top
            || innerBottomConstraint?.constant != offsets.bottom
        else { return }
        innerTopConstraint?.constant = offsets.top
        innerBottomConstraint?.constant = offsets.bottom
        platformSetNeedsLayout()
    }

    // The pixel scale is only a guess until the view joins a window.
    #if os(macOS)
    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        platformSetNeedsLayout()
    }
    #else
    public override func didMoveToWindow() {
        super.didMoveToWindow()
        platformSetNeedsLayout()
    }
    #endif

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

    /// Lets ObjC++ install an error handler without needing to bridge the
    /// Swift `Error` type.
    @objc public func setErrorCallback(_ handler: @escaping (String) -> Void) {
        innerView.onError = { error in handler(error.localizedDescription) }
    }

    /// Set by ComponentView to dispatch content size events.
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
        // The shift depends on the laid-out bounds — re-resolve every pass.
        updateInlineShift()
        updateBaselineGuide()
        let size = innerView.intrinsicContentSize
        guard size.width > 0, size.height > 0 else { return }
        guard size != lastReportedContentSize else { return }
        lastReportedContentSize = size
        contentSizeCallback?(size.width, size.height)
    }

    // MARK: - Baseline (native AutoLayout)

    #if os(macOS)
    public override var firstBaselineOffsetFromTop: CGFloat {
        innerView.frame.origin.y + (innerView.baselineFromTop ?? bounds.height)
    }

    public override var baselineOffsetFromBottom: CGFloat {
        bounds.height - firstBaselineOffsetFromTop
    }

    private func updateBaselineGuide() {}
    #else
    /// Zero-size guide whose bottom edge tracks the drawn formula's alphabetic
    /// baseline, so `firstBaselineAnchor` / `lastBaselineAnchor` constraints align
    /// the formula with neighboring text.
    private let baselineGuide: UIView = {
        let guide = UIView()
        guide.isHidden = true
        guide.isUserInteractionEnabled = false
        return guide
    }()

    public override var forFirstBaselineLayout: UIView {
        updateBaselineGuide()
        return baselineGuide
    }

    public override var forLastBaselineLayout: UIView {
        updateBaselineGuide()
        return baselineGuide
    }

    private func updateBaselineGuide() {
        guard baselineGuide.superview === self else { return }
        let baseline =
            innerView.frame.origin.y + (innerView.baselineFromTop ?? bounds.height)
        baselineGuide.frame = CGRect(x: 0, y: 0, width: bounds.width, height: baseline)
    }
    #endif

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
        addSubview(baselineGuide)
        #endif
        addSubview(innerView)
        innerView.translatesAutoresizingMaskIntoConstraints = false
        // Top/bottom constants position the inner view: equal constants = pure
        // translate (center/start/end); baseline mode sets them apart so the
        // inner keeps natural height and its descender overflows the
        // ascent-only wrapper (clipsToBounds stays false).
        let top = innerView.topAnchor.constraint(equalTo: topAnchor)
        let bottom = innerView.bottomAnchor.constraint(equalTo: bottomAnchor)
        innerTopConstraint = top
        innerBottomConstraint = bottom
        NSLayoutConstraint.activate([
            innerView.leadingAnchor.constraint(equalTo: leadingAnchor),
            innerView.trailingAnchor.constraint(equalTo: trailingAnchor),
            top,
            bottom,
        ])
        // Load fonts from the CocoaPods resource bundle (not the main bundle or SPM bundle).
        // Guard ensures we only do this once across all RaTeXRNView instances.
        if !RaTeXRNView.fontsLoaded {
            RaTeXFontLoader.loadFromBundle(RaTeXRNView.fontsBundle)
            RaTeXRNView.fontsLoaded = true
        }
    }
}

/// Natural (unscaled) formula metrics in points at a given font size.
/// `height` is the total ink height; the alphabetic baseline sits at
/// `height - depth` from the top of the ink box.
@objc(RaTeXTexMetrics)
public final class RaTeXTexMetrics: NSObject {
    @objc public let width: CGFloat
    @objc public let height: CGFloat
    @objc public let depth: CGFloat

    init(width: CGFloat, height: CGFloat, depth: CGFloat) {
        self.width = width
        self.height = height
        self.depth = depth
    }
}

/// Thread-safe synchronous LaTeX measurement.
///
/// Used by the Fabric shadow node's `measureContent` (self-sizing during Yoga
/// layout) and `baseline` (alignItems:'baseline'), which run per layout pass —
/// so parses are cached, keyed by (latex, displayMode). The DisplayList is in em
/// units: fontSize is applied afterwards and color never affects metrics. Safe to
/// call off the main thread: `RaTeXEngine.parse` is thread-safe (thread-local FFI
/// error state) and `RaTeXRenderer` is a value type; fonts are not needed for
/// measurement.
@objc(RaTeXMeasure)
public final class RaTeXMeasure: NSObject {
    private static let cacheLock = NSLock()
    private static var cache: [String: DisplayList] = [:]
    private static var cacheOrder: [String] = []
    private static let cacheMaxEntries = 128

    private static func parseCached(_ latex: String, displayMode: Bool) -> DisplayList? {
        let key = (displayMode ? "D|" : "T|") + latex
        cacheLock.lock()
        if let hit = cache[key] {
            cacheLock.unlock()
            return hit
        }
        cacheLock.unlock()
        // Parse outside the lock so a slow parse never blocks other lookups; racing
        // duplicate parses of the same key produce identical output (last put wins).
        guard let parsed = try? RaTeXEngine.shared.parse(latex, displayMode: displayMode, color: .black) else {
            return nil
        }
        cacheLock.lock()
        if cache[key] == nil {
            cache[key] = parsed
            cacheOrder.append(key)
            if cacheOrder.count > cacheMaxEntries {
                cache.removeValue(forKey: cacheOrder.removeFirst())
            }
        }
        cacheLock.unlock()
        return parsed
    }

    @objc public static func measureLatex(_ latex: String, fontSize: CGFloat, displayMode: Bool) -> CGSize {
        guard let m = metricsLatex(latex, fontSize: fontSize, displayMode: displayMode) else { return .zero }
        return CGSize(width: m.width, height: m.height)
    }

    /// Natural width / total height / depth in points, or nil when the input is
    /// empty or fails to parse.
    @objc public static func metricsLatex(_ latex: String, fontSize: CGFloat, displayMode: Bool) -> RaTeXTexMetrics? {
        guard !latex.isEmpty, fontSize > 0 else { return nil }
        guard let displayList = parseCached(latex, displayMode: displayMode) else { return nil }
        let renderer = RaTeXRenderer(displayList: displayList, fontSize: fontSize)
        return RaTeXTexMetrics(width: renderer.width, height: renderer.totalHeight, depth: renderer.depth)
    }
}
