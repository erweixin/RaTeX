// RaTeXInlineView.swift — Platform view that renders a mix of plain text and $...$
// LaTeX formulas in a single text flow using NSLayoutManager.
//
// Each formula is converted to an image-backed NSTextAttachment with baseline-
// aligned bounds, so TextKit handles word-wrapping and line-breaking at character
// granularity — the same way a native label or text view would.

#if os(macOS)
import AppKit
#else
import UIKit
#endif

@MainActor
public class RaTeXInlineView: PlatformView {

    // MARK: - Public properties

    public var content: String = "" {
        didSet { guard content != oldValue else { return }; rebuild() }
    }

    public var formulaFontSize: CGFloat = 16 {
        didSet { guard formulaFontSize != oldValue else { return }; rebuild() }
    }

    public var formulaColor: PlatformColor = .black {
        didSet { guard !formulaColor.isEqual(oldValue) else { return }; rebuild() }
    }

    public var textColor: PlatformColor = .black {
        didSet { guard !textColor.isEqual(oldValue) else { return }; rebuild() }
    }

    public var textFontSize: CGFloat = 16 {
        didSet { guard textFontSize != oldValue else { return }; rebuild() }
    }

    public var textFontFamily: String? {
        didSet {
            let newTrimmed = textFontFamily?.trimmingCharacters(in: .whitespacesAndNewlines)
            let oldTrimmed = trimmedTextFontFamily
            guard oldTrimmed != newTrimmed else { return }
            trimmedTextFontFamily = newTrimmed
            rebuild()
        }
    }

    public var textItalic: Bool = false {
        didSet { guard textItalic != oldValue else { return }; rebuild() }
    }

    public var textUnderline: Bool = false {
        didSet { guard textUnderline != oldValue else { return }; rebuild() }
    }

    public var textLineThrough: Bool = false {
        didSet { guard textLineThrough != oldValue else { return }; rebuild() }
    }

    public var onContentSizeChange: ((CGFloat, CGFloat) -> Void)?

    // MARK: - Private state

    private let textStorage = NSTextStorage()
    private let layoutManager = NSLayoutManager()
    private let textContainer = NSTextContainer()
    private var lastReportedSize: CGSize = .zero
    private var lastEmittedSize: CGSize?
    private var lastLayoutWidth: CGFloat = -1
    private var trimmedTextFontFamily: String?

    // MARK: - Init

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
        layer?.backgroundColor = NSColor.clear.cgColor
        #else
        backgroundColor = .clear
        isOpaque = false
        contentMode = .redraw
        #endif
        textContainer.lineFragmentPadding = 0
        textContainer.lineBreakMode = .byWordWrapping
        layoutManager.addTextContainer(textContainer)
        textStorage.addLayoutManager(layoutManager)
    }

    #if os(macOS)
    public override var isFlipped: Bool { true }
    #endif

    // MARK: - Layout

    #if os(macOS)
    public override func layout() {
        super.layout()
        performLayoutPass()
    }
    #else
    public override func layoutSubviews() {
        super.layoutSubviews()
        performLayoutPass()
    }
    #endif

    private func performLayoutPass() {
        let w = bounds.width
        guard w > 0 else { return }
        if w != lastLayoutWidth {
            lastLayoutWidth = w
            textContainer.size = CGSize(width: w, height: .greatestFiniteMagnitude)
            layoutManager.ensureLayout(for: textContainer)
            reportContentSizeIfNeeded()
            platformSetNeedsDisplay()
        }
    }

    // MARK: - Drawing

    public override func draw(_ rect: CGRect) {
        guard textStorage.length > 0 else { return }
        let glyphRange = layoutManager.glyphRange(for: textContainer)
        layoutManager.drawBackground(forGlyphRange: glyphRange, at: .zero)
        layoutManager.drawGlyphs(forGlyphRange: glyphRange, at: .zero)
    }

    // MARK: - Intrinsic content size

    public override var intrinsicContentSize: CGSize {
        lastReportedSize
    }

    // MARK: - Content size

    private func reportContentSizeIfNeeded() {
        let usedRect = textStorage.length > 0
            ? layoutManager.usedRect(for: textContainer)
            : .zero
        let size = CGSize(
            width: max(0, ceil(usedRect.width)),
            height: max(0, ceil(usedRect.height))
        )
        updateContentSize(size)
    }

    private func updateContentSize(_ size: CGSize) {
        let shouldInvalidateIntrinsicSize = size != lastReportedSize
        lastReportedSize = size
        if shouldInvalidateIntrinsicSize {
            invalidateIntrinsicContentSize()
        }
        guard size != lastEmittedSize else { return }
        lastEmittedSize = size
        onContentSizeChange?(size.width, size.height)
    }

    public func resetContentSizeReporting() {
        lastEmittedSize = nil
        lastLayoutWidth = -1
        platformSetNeedsLayout()
    }

    // MARK: - Rebuild

    private func rebuild() {
        RaTeXFontLoader.ensureLoaded()
        let attributed = buildAttributedString()
        textStorage.setAttributedString(attributed)
        lastLayoutWidth = -1
        lastEmittedSize = nil

        // Compute single-line size so intrinsicContentSize is non-zero before
        // the first layout pass (prevents zero-width collapse when the
        // parent uses alignItems: 'center').
        if textStorage.length > 0 {
            textContainer.size = CGSize(
                width: CGFloat.greatestFiniteMagnitude,
                height: .greatestFiniteMagnitude
            )
            layoutManager.ensureLayout(for: textContainer)
            let usedRect = layoutManager.usedRect(for: textContainer)
            lastReportedSize = CGSize(
                width: max(0, ceil(usedRect.width)),
                height: max(0, ceil(usedRect.height))
            )
        } else {
            lastReportedSize = .zero
        }

        invalidateIntrinsicContentSize()
        platformSetNeedsLayout()
        platformSetNeedsDisplay()
    }

    private func buildAttributedString() -> NSAttributedString {
        let segments = Self.parseContent(content)
        let result = NSMutableAttributedString()

        let textFont = makeTextFont()
        var textAttrs: [NSAttributedString.Key: Any] = [
            .font: textFont,
            .foregroundColor: textColor,
        ]
        if textUnderline {
            textAttrs[.underlineStyle] = NSUnderlineStyle.single.rawValue
        }
        if textLineThrough {
            textAttrs[.strikethroughStyle] = NSUnderlineStyle.single.rawValue
        }

        for segment in segments {
            switch segment {
            case .text(let str):
                result.append(NSAttributedString(string: str, attributes: textAttrs))
            case .formula(let latex):
                if let attachment = makeFormulaAttachment(latex) {
                    result.append(NSAttributedString(attachment: attachment))
                } else {
                    result.append(NSAttributedString(string: "$\(latex)$", attributes: textAttrs))
                }
            }
        }
        return result
    }

    private func makeTextFont() -> PlatformFont {
        #if os(macOS)
        let customFont = trimmedTextFontFamily.flatMap { family -> NSFont? in
            guard !family.isEmpty else { return nil }
            return NSFont(name: family, size: textFontSize)
        }
        let baseFont = customFont ?? NSFont.systemFont(ofSize: textFontSize)
        guard textItalic else { return baseFont }
        let italicFont = NSFontManager.shared.convert(baseFont, toHaveTrait: .italicFontMask)
        return italicFont
        #else
        let customFont = trimmedTextFontFamily.flatMap { family -> UIFont? in
            guard !family.isEmpty else { return nil }
            return UIFont(name: family, size: textFontSize)
        }
        let baseFont = customFont ?? UIFont.systemFont(ofSize: textFontSize)
        guard textItalic else { return baseFont }
        if let descriptor = baseFont.fontDescriptor.withSymbolicTraits([.traitItalic]) {
            return UIFont(descriptor: descriptor, size: textFontSize)
        }
        return UIFont.italicSystemFont(ofSize: textFontSize)
        #endif
    }

    private func makeFormulaAttachment(_ latex: String) -> RaTeXTextAttachment? {
        do {
            #if os(macOS)
            let dl = try RaTeXEngine.shared.parse(
                latex,
                displayMode: false,
                color: formulaColor,
                appearance: effectiveAppearance
            )
            #else
            let dl = try RaTeXEngine.shared.parse(
                latex,
                displayMode: false,
                color: formulaColor,
                traitCollection: traitCollection
            )
            #endif
            let renderer = RaTeXRenderer(displayList: dl, fontSize: formulaFontSize)
            return RaTeXTextAttachment(renderer: renderer)
        } catch {
            return nil
        }
    }

    // MARK: - Parsing

    enum Segment {
        case text(String)
        case formula(String)
    }

    static func parseContent(_ content: String) -> [Segment] {
        var segments: [Segment] = []
        var current = ""
        var inFormula = false
        var index = content.startIndex

        while index < content.endIndex {
            let ch = content[index]
            let nextIndex = content.index(after: index)
            if ch == "\\", nextIndex < content.endIndex, content[nextIndex] == "$" {
                if inFormula {
                    current.append("\\$")
                } else {
                    current.append("$")
                }
                index = content.index(after: nextIndex)
                continue
            }

            if ch == "$" {
                if inFormula {
                    if !current.isEmpty {
                        segments.append(.formula(current))
                    } else {
                        segments.append(.text("$$"))
                    }
                    current = ""
                    inFormula = false
                } else {
                    if !current.isEmpty {
                        segments.append(.text(current))
                    }
                    current = ""
                    inFormula = true
                }
            } else {
                current.append(ch)
            }
            index = nextIndex
        }

        if inFormula {
            segments.append(.text("$\(current)"))
        } else if !current.isEmpty {
            segments.append(.text(current))
        }

        return segments
    }

    // MARK: - Appearance changes

    #if os(macOS)
    public override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        rebuild()
    }
    #else
    public override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        guard let previousTraitCollection else { return }
        guard traitCollection.hasDifferentColorAppearance(comparedTo: previousTraitCollection) else {
            return
        }
        rebuild()
    }
    #endif
}
