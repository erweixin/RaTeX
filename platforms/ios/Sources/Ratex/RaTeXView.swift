// RaTeXView.swift - Platform view and SwiftUI wrapper for rendering a LaTeX formula.

#if os(macOS)
import AppKit
#else
import UIKit
#endif
import SwiftUI

private struct RaTeXColorKey: EnvironmentKey {
    static let defaultValue: Color = .black
}

@available(iOS 14, macOS 11, *)
public extension EnvironmentValues {
    var ratexColor: Color {
        get { self[RaTeXColorKey.self] }
        set { self[RaTeXColorKey.self] = newValue }
    }
}

@available(iOS 14, macOS 11, *)
public extension View {
    func ratexColor(_ color: Color) -> some View {
        environment(\.ratexColor, color)
    }
}

@available(iOS 14, macOS 11, *)
private func platformColor(from color: Color) -> PlatformColor {
    PlatformColor(color)
}

// MARK: - Platform View

/// A view that renders a LaTeX formula using the RaTeX engine.
///
/// ```swift
/// let view = RaTeXView()
/// view.latex = #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#
/// view.fontSize = 28
/// ```
@MainActor
@objcMembers
public class RaTeXView: PlatformView {

    // MARK: Public properties

    /// The LaTeX math-mode string to render.
    public var latex: String = "" {
        didSet { guard latex != oldValue else { return }; rerender() }
    }

    /// Font size in points. Determines the physical size of the formula.
    public var fontSize: CGFloat = 24 {
        didSet { guard fontSize != oldValue else { return }; rerender() }
    }

    /// Rendering mode. `true` (default) for display/block style (`$$...$$`);
    /// `false` for inline/text style (`$...$`).
    public var displayMode: Bool = true {
        didSet { guard displayMode != oldValue else { return }; rerender() }
    }

    /// Default formula color. Explicit LaTeX colors still take precedence.
    public var color: PlatformColor = .black {
        didSet { guard !color.isEqual(oldValue) else { return }; rerender() }
    }

    /// Called when a render error occurs (e.g. invalid LaTeX).
    public var onError: ((Error) -> Void)?

    /// Called after each successful render with the formula's ascent and total
    /// height in points.
    public var onLayout: ((CGFloat, CGFloat) -> Void)?

    /// Distance from top to baseline (points).
    public private(set) var mathAscent: CGFloat = 0

    /// Distance from baseline to bottom (points).
    public private(set) var mathDescent: CGFloat = 0

    // MARK: Private state

    private var renderer: RaTeXRenderer?

    #if !os(macOS)
    /// Invisible 0-height marker whose top edge sits exactly on the formula's
    /// alphabetic baseline. UIKit reads `forFirstBaselineLayout.frame.minY` to
    /// resolve `firstBaselineAnchor`, which SwiftUI then uses for baseline
    /// alignment guides (e.g. HStack with .firstTextBaseline).
    private let baselineMarker = UIView()
    #endif

    // MARK: Init

    public override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        #if os(macOS)
        wantsLayer = true
        layerContentsRedrawPolicy = .onSetNeedsDisplay
        layer?.backgroundColor = NSColor.clear.cgColor
        #else
        backgroundColor = .clear
        contentMode = .redraw
        baselineMarker.isHidden = true
        baselineMarker.isUserInteractionEnabled = false
        addSubview(baselineMarker)
        #endif
    }

    #if os(macOS)
    public override var isFlipped: Bool { true }

    public override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        if renderer != nil {
            platformSetNeedsDisplay()
        }
    }
    #else
    // MARK: Baseline

    /// Return the marker so UIKit derives `firstBaselineAnchor` from its top edge.
    public override var forFirstBaselineLayout: UIView { baselineMarker }
    public override var forLastBaselineLayout: UIView { baselineMarker }
    #endif

    // MARK: Layout

    public override var intrinsicContentSize: CGSize {
        guard let r = renderer else { return .zero }
        return CGSize(width: r.width, height: r.totalHeight)
    }

    // MARK: Drawing

    public override func draw(_ rect: CGRect) {
        #if os(macOS)
        guard let renderer, let ctx = NSGraphicsContext.current?.cgContext else { return }
        #else
        guard let renderer, let ctx = UIGraphicsGetCurrentContext() else { return }
        #endif
        renderer.draw(in: ctx)
    }

    #if os(macOS)
    public override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        rerender()
    }
    #else
    public override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        guard let previousTraitCollection else { return }
        guard traitCollection.hasDifferentColorAppearance(comparedTo: previousTraitCollection) else {
            return
        }
        rerender()
    }
    #endif

    // MARK: Private

    private func rerender() {
        // Parsing + layout is < 1ms - run synchronously on the main thread.
        // Async dispatch would cause UITableView/List to lock in a zero height
        // before the render completes, making cells invisible.
        RaTeXFontLoader.ensureLoaded()
        do {
            #if os(macOS)
            let dl = try RaTeXEngine.shared.parse(
                latex,
                displayMode: displayMode,
                color: color,
                appearance: effectiveAppearance
            )
            #else
            let dl = try RaTeXEngine.shared.parse(
                latex,
                displayMode: displayMode,
                color: color,
                traitCollection: traitCollection
            )
            #endif
            renderer = RaTeXRenderer(displayList: dl, fontSize: fontSize)
            mathAscent = renderer?.height ?? 0
            mathDescent = renderer?.depth ?? 0
            let ascent = renderer?.height ?? 0
            let totalHeight = renderer?.totalHeight ?? 0
            #if !os(macOS)
            baselineMarker.frame = CGRect(x: 0, y: ascent, width: 1, height: 0)
            #endif
            invalidateIntrinsicContentSize()
            platformSetNeedsDisplay()
            onLayout?(ascent, totalHeight)
        } catch {
            mathAscent = 0
            mathDescent = 0
            onError?(error)
        }
    }
}

// MARK: - LayoutValueKey (iOS 16+ / macOS 13+)

/// The typographic ascent (top-of-view -> baseline, in points) of a ``RaTeXFormula``.
///
/// ``RaTeXFormula`` writes this value automatically on every render. Custom
/// `Layout` implementations can read it to perform baseline-aligned inline
/// formula+text mixing without any extra wiring:
///
/// ```swift
/// struct FlowLayout: Layout {
///     func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize,
///                        subviews: Subviews, cache: inout Cache) {
///         for subview in subviews {
///             let ascent = subview[RaTeXFormulaAscentKey.self]
///             // ascent > 0 for RaTeXFormula; 0 for plain Text views
///         }
///     }
/// }
/// ```
@available(iOS 16, macOS 13, *)
public struct RaTeXFormulaAscentKey: LayoutValueKey {
    public static let defaultValue: CGFloat = 0
}

// MARK: - SwiftUI

/// A SwiftUI view that renders a LaTeX formula.
///
/// ```swift
/// RaTeXFormula(latex: #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#, fontSize: 24)
/// ```
///
/// ### Baseline alignment (all versions, iOS + macOS)
///
/// `RaTeXFormula` reports its math baseline via `.alignmentGuide(.firstTextBaseline)`,
/// so it aligns with surrounding text out of the box on both platforms:
///
/// ```swift
/// HStack(alignment: .firstTextBaseline) {
///     Text("Euler's identity:")
///     RaTeXFormula(latex: #"e^{i\pi}+1=0"#, fontSize: 17, displayMode: false)
/// }
/// ```
///
/// ### Inline mixing with custom layouts (iOS 16+ / macOS 13+)
///
/// `RaTeXFormula` automatically writes its typographic ascent into
/// ``RaTeXFormulaAscentKey`` on every render. Any parent `Layout` can read
/// this value to align formulas on the same baseline as surrounding text:
///
/// ```swift
/// let ascent = subview[RaTeXFormulaAscentKey.self] // > 0 for RaTeXFormula
/// ```
@available(iOS 14, macOS 11, *)
public struct RaTeXFormula: View {
    public let latex: String
    public var fontSize: CGFloat = 24
    public var displayMode: Bool = true
    public var color: Color? = nil
    public var onError: ((Error) -> Void)? = nil
    public var onLayout: ((CGFloat, CGFloat) -> Void)? = nil
    @Environment(\.ratexColor) private var environmentColor

    public init(
        latex: String,
        fontSize: CGFloat = 24,
        displayMode: Bool = true,
        color: Color? = nil,
        onError: ((Error) -> Void)? = nil,
        onLayout: ((CGFloat, CGFloat) -> Void)? = nil
    ) {
        self.latex = latex
        self.fontSize = fontSize
        self.displayMode = displayMode
        self.color = color
        self.onError = onError
        self.onLayout = onLayout
    }

    private var resolvedColor: Color {
        color ?? environmentColor
    }

    /// Synchronously computes the formula's ascent (top-of-view -> baseline).
    /// Called in `body` so the value is available on the very first layout pass.
    /// `parse()` is < 1ms and is called internally by `RaTeXView.rerender()` anyway.
    private var ascent: CGFloat {
        guard let dl = try? RaTeXEngine.shared.parse(
            latex,
            displayMode: displayMode,
            color: platformColor(from: resolvedColor)
        ) else { return 0 }
        return CGFloat(dl.height) * fontSize
    }

    public var body: some View {
        // `.alignmentGuide(.firstTextBaseline)` reports the formula's math baseline
        // explicitly, so `HStack(alignment: .firstTextBaseline)` and `Text` baseline
        // alignment work identically on iOS and macOS, on every supported OS version.
        // This does not rely on SwiftUI bridging the UIKit/AppKit native baseline
        // anchors through the representable (which is undocumented and inconsistent).
        representable
            .alignmentGuide(.firstTextBaseline) { _ in ascent }
            .alignmentGuide(.lastTextBaseline) { _ in ascent }
    }

    @ViewBuilder
    private var representable: some View {
        if #available(iOS 16, macOS 13, *) {
            _RaTeXRepresentable(
                latex: latex,
                fontSize: fontSize,
                displayMode: displayMode,
                color: resolvedColor,
                onError: onError,
                onLayout: onLayout
            )
            // Read by custom `Layout`s for flow/wrapping (see ``RaTeXFormulaAscentKey``).
            .layoutValue(key: RaTeXFormulaAscentKey.self, value: ascent)
        } else {
            _RaTeXRepresentable(
                latex: latex,
                fontSize: fontSize,
                displayMode: displayMode,
                color: resolvedColor,
                onError: onError,
                onLayout: onLayout
            )
        }
    }
}

#if os(macOS)

// MARK: - Internal NSViewRepresentable

@available(macOS 11, *)
private struct _RaTeXRepresentable: NSViewRepresentable {
    let latex: String
    var fontSize: CGFloat
    var displayMode: Bool
    var color: Color
    var onError: ((Error) -> Void)?
    var onLayout: ((CGFloat, CGFloat) -> Void)?

    func makeNSView(context: Context) -> RaTeXView {
        let view = RaTeXView()
        view.setContentHuggingPriority(.required, for: .horizontal)
        view.setContentHuggingPriority(.required, for: .vertical)
        return view
    }

    func updateNSView(_ nsView: RaTeXView, context: Context) {
        nsView.fontSize = fontSize
        nsView.displayMode = displayMode
        nsView.color = platformColor(from: color)
        nsView.onError = onError
        nsView.onLayout = onLayout
        nsView.latex = latex
    }
}

#else

// MARK: - Internal UIViewRepresentable

@available(iOS 14, *)
private struct _RaTeXRepresentable: UIViewRepresentable {
    let latex: String
    var fontSize: CGFloat
    var displayMode: Bool
    var color: Color
    var onError: ((Error) -> Void)?
    var onLayout: ((CGFloat, CGFloat) -> Void)?

    func makeUIView(context: Context) -> RaTeXView {
        let view = RaTeXView()
        view.setContentHuggingPriority(.required, for: .horizontal)
        view.setContentHuggingPriority(.required, for: .vertical)
        return view
    }

    func updateUIView(_ uiView: RaTeXView, context: Context) {
        uiView.fontSize = fontSize
        uiView.displayMode = displayMode
        uiView.color = platformColor(from: color)
        uiView.onError = onError
        uiView.onLayout = onLayout
        uiView.latex = latex
    }
}

#endif
