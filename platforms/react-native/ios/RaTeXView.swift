// RaTeXView.swift — UIKit view and SwiftUI wrapper for rendering a LaTeX formula.

import UIKit
import SwiftUI

// MARK: - UIKit

/// A UIView that renders a LaTeX formula using the RaTeX engine.
///
/// ```swift
/// let view = RaTeXView()
/// view.latex = #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#
/// view.fontSize = 28
/// ```
@MainActor
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

    // MARK: Private state

    private var renderer: RaTeXRenderer?

    // MARK: Init

    public override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        // Redraw whenever the frame changes (needed in Fabric / any layout system
        // that sets the frame after updateProps has already called setNeedsDisplay).
        contentMode = .redraw
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        backgroundColor = .clear
        contentMode = .redraw
    }

    // MARK: Layout

    public override var intrinsicContentSize: CGSize {
        guard let r = renderer else { return .zero }
        return CGSize(width: r.width, height: r.totalHeight)
    }

    // MARK: Drawing

    public override func draw(_ rect: CGRect) {
        guard let renderer, let ctx = UIGraphicsGetCurrentContext() else { return }

        let contentW = renderer.width
        let contentH = renderer.totalHeight
        let availW = max(0, bounds.width)
        let availH = max(0, bounds.height)

        ctx.saveGState()
        ctx.clip(to: bounds)

        // Scale down to fit in the assigned layout size; never scale up.
        let sx: CGFloat = contentW > 0 ? (availW / contentW) : 1
        let sy: CGFloat = contentH > 0 ? (availH / contentH) : 1
        let scale = min(1, min(sx, sy))

        let scaledW = contentW * scale
        let scaledH = contentH * scale
        let dx = max(0, (availW - scaledW) / 2)
        let dy = max(0, (availH - scaledH) / 2)

        ctx.translateBy(x: dx, y: dy)
        ctx.scaleBy(x: scale, y: scale)
        renderer.draw(in: ctx)
        ctx.restoreGState()
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
            invalidateIntrinsicContentSize()
            setNeedsDisplay()
        } catch {
            onError?(error)
        }
    }
}

// MARK: - SwiftUI

/// A SwiftUI view that renders a LaTeX formula.
///
/// ```swift
/// RaTeXFormula(latex: #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#, fontSize: 24)
/// ```
@available(iOS 14, *)
public struct RaTeXFormula: UIViewRepresentable {
    public let latex: String
    public var fontSize: CGFloat = 24
    public var onError: ((Error) -> Void)? = nil

    public init(latex: String, fontSize: CGFloat = 24, onError: ((Error) -> Void)? = nil) {
        self.latex = latex
        self.fontSize = fontSize
        self.onError = onError
    }

    public func makeUIView(context: Context) -> RaTeXView {
        let view = RaTeXView()
        view.setContentHuggingPriority(.required, for: .horizontal)
        view.setContentHuggingPriority(.required, for: .vertical)
        return view
    }

    public func updateUIView(_ uiView: RaTeXView, context: Context) {
        uiView.fontSize = fontSize
        uiView.onError  = onError
        uiView.latex    = latex
    }
}
