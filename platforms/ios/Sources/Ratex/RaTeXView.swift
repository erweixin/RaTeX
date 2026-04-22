// RaTeXView.swift — UIKit view and SwiftUI wrapper for rendering a LaTeX formula.

import UIKit
import SwiftUI

private struct RaTeXColorKey: EnvironmentKey {
    static let defaultValue: Color = .black
}

@available(iOS 14, *)
public extension EnvironmentValues {
    public var ratexColor: Color {
        get { self[RaTeXColorKey.self] }
        set { self[RaTeXColorKey.self] = newValue }
    }
}

@available(iOS 14, *)
public extension View {
    public func ratexColor(_ color: Color) -> some View {
        environment(\.ratexColor, color)
    }
}

@available(iOS 14, *)
private func uiColor(from color: Color) -> UIColor {
    UIColor(color)
}

// MARK: - UIKit

/// A UIView that renders a LaTeX formula using the RaTeX engine.
///
/// ```swift
/// let view = RaTeXView()
/// view.latex = #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#
/// view.fontSize = 28
/// ```
@MainActor
@objcMembers
public class RaTeXView: UIView {

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
    public var color: UIColor = .black {
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

    /// Invisible 0-height marker whose top edge sits exactly on the formula's
    /// alphabetic baseline. UIKit reads `forFirstBaselineLayout.frame.minY` to
    /// resolve `firstBaselineAnchor`, which SwiftUI then uses for baseline
    /// alignment guides (e.g. HStack with .firstTextBaseline).
    private let baselineMarker = UIView()

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
        backgroundColor = .clear
        baselineMarker.isHidden = true
        baselineMarker.isUserInteractionEnabled = false
        addSubview(baselineMarker)
    }

    // MARK: Baseline

    /// Return the marker so UIKit derives `firstBaselineAnchor` from its top edge.
    public override var forFirstBaselineLayout: UIView { baselineMarker }
    public override var forLastBaselineLayout:  UIView { baselineMarker }

    // MARK: Layout

    public override var intrinsicContentSize: CGSize {
        guard let r = renderer else { return .zero }
        return CGSize(width: r.width, height: r.totalHeight)
    }

    // MARK: Drawing

    public override func draw(_ rect: CGRect) {
        guard let renderer, let ctx = UIGraphicsGetCurrentContext() else { return }
        renderer.draw(in: ctx)
    }

    public override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        guard let previousTraitCollection else { return }
        guard traitCollection.hasDifferentColorAppearance(comparedTo: previousTraitCollection) else {
            return
        }
        rerender()
    }

    // MARK: Private

    private func rerender() {
        // Parsing + layout is < 1ms — run synchronously on the main thread.
        // Async dispatch would cause UITableView/List to lock in a zero height
        // before the render completes, making cells invisible.
        RaTeXFontLoader.ensureLoaded()
        do {
            let dl = try RaTeXEngine.shared.parse(
                latex,
                displayMode: displayMode,
                color: color,
                traitCollection: traitCollection
            )
            renderer = RaTeXRenderer(displayList: dl, fontSize: fontSize)
            mathAscent  = renderer?.height ?? 0
            mathDescent = renderer?.depth  ?? 0
            let ascent      = renderer?.height      ?? 0
            let totalHeight = renderer?.totalHeight ?? 0
            baselineMarker.frame = CGRect(x: 0, y: ascent, width: 1, height: 0)
            invalidateIntrinsicContentSize()
            setNeedsDisplay()
            onLayout?(ascent, totalHeight)
        } catch {
            mathAscent  = 0
            mathDescent = 0
            onError?(error)
        }
    }
}

// MARK: - LayoutValueKey (iOS 16+)

/// The typographic ascent (top-of-view → baseline, in points) of a ``RaTeXFormula``.
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
@available(iOS 16, *)
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
/// ### Inline mixing with custom layouts (iOS 16+)
///
/// `RaTeXFormula` automatically writes its typographic ascent into
/// ``RaTeXFormulaAscentKey`` on every render. Any parent `Layout` can read
/// this value to align formulas on the same baseline as surrounding text:
///
/// ```swift
/// let ascent = subview[RaTeXFormulaAscentKey.self] // > 0 for RaTeXFormula
/// ```
@available(iOS 14, *)
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
        self.latex       = latex
        self.fontSize    = fontSize
        self.displayMode = displayMode
        self.color       = color
        self.onError     = onError
        self.onLayout    = onLayout
    }

    private var resolvedColor: Color {
        color ?? environmentColor
    }

    /// Synchronously computes the formula's ascent (top-of-view → baseline).
    /// Called in `body` so the value is available on the very first layout pass.
    /// `parse()` is < 1ms and is called internally by `RaTeXView.rerender()` anyway.
    private var ascent: CGFloat {
        guard let dl = try? RaTeXEngine.shared.parse(
            latex,
            displayMode: displayMode,
            color: uiColor(from: resolvedColor)
        ) else { return 0 }
        return CGFloat(dl.height) * fontSize
    }

    public var body: some View {
        if #available(iOS 16, *) {
            _RaTeXRepresentable(latex: latex, fontSize: fontSize, displayMode: displayMode,
                                color: resolvedColor,
                                onError: onError, onLayout: onLayout)
                .layoutValue(key: RaTeXFormulaAscentKey.self, value: ascent)
        } else {
            _RaTeXRepresentable(latex: latex, fontSize: fontSize, displayMode: displayMode,
                                color: resolvedColor,
                                onError: onError, onLayout: onLayout)
        }
    }
}

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
        uiView.fontSize    = fontSize
        uiView.displayMode = displayMode
        uiView.color       = uiColor(from: color)
        uiView.onError     = onError
        uiView.onLayout    = onLayout
        uiView.latex       = latex
    }
}
