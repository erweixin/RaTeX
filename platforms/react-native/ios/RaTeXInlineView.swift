// RaTeXInlineView.swift — UIView that renders a mix of plain text and $...$
// LaTeX formulas in a single text flow using NSLayoutManager.
//
// Each formula is converted to a UIImage-backed NSTextAttachment with baseline-
// aligned bounds, so TextKit handles word-wrapping and line-breaking at character
// granularity — the same way a native UILabel or UITextView would.

import UIKit

@MainActor
public class RaTeXInlineView: UIView {

    // MARK: - Public properties

    public var content: String = "" {
        didSet { guard content != oldValue else { return }; rebuild() }
    }

    public var formulaFontSize: CGFloat = 16 {
        didSet { guard formulaFontSize != oldValue else { return }; rebuild() }
    }

    public var formulaColor: UIColor = .black {
        didSet { guard !formulaColor.isEqual(oldValue) else { return }; rebuild() }
    }

    public var textColor: UIColor = .black {
        didSet { guard !textColor.isEqual(oldValue) else { return }; rebuild() }
    }

    public var textFontSize: CGFloat = 16 {
        didSet { guard textFontSize != oldValue else { return }; rebuild() }
    }

    public var onContentSizeChange: ((CGFloat, CGFloat) -> Void)?

    // MARK: - Private state

    private let textStorage = NSTextStorage()
    private let layoutManager = NSLayoutManager()
    private let textContainer = NSTextContainer()
    private var lastReportedSize: CGSize = .zero
    private var lastLayoutWidth: CGFloat = -1

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
        backgroundColor = .clear
        isOpaque = false
        contentMode = .redraw
        textContainer.lineFragmentPadding = 0
        textContainer.lineBreakMode = .byWordWrapping
        layoutManager.addTextContainer(textContainer)
        textStorage.addLayoutManager(layoutManager)
    }

    // MARK: - Layout

    public override func layoutSubviews() {
        super.layoutSubviews()
        let w = bounds.width
        guard w > 0 else { return }
        if w != lastLayoutWidth {
            lastLayoutWidth = w
            textContainer.size = CGSize(width: w, height: .greatestFiniteMagnitude)
            layoutManager.ensureLayout(for: textContainer)
            reportContentSizeIfNeeded()
            setNeedsDisplay()
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
        let usedRect = layoutManager.usedRect(for: textContainer)
        let size = CGSize(width: ceil(usedRect.width), height: ceil(usedRect.height))
        guard size != lastReportedSize, size.width > 0, size.height > 0 else { return }
        lastReportedSize = size
        invalidateIntrinsicContentSize()
        onContentSizeChange?(size.width, size.height)
    }

    // MARK: - Rebuild

    private func rebuild() {
        RaTeXFontLoader.ensureLoaded()
        let attributed = buildAttributedString()
        textStorage.setAttributedString(attributed)
        lastLayoutWidth = -1
        lastReportedSize = .zero
        invalidateIntrinsicContentSize()
        setNeedsLayout()
        setNeedsDisplay()
    }

    private func buildAttributedString() -> NSAttributedString {
        let segments = Self.parseContent(content)
        let result = NSMutableAttributedString()

        let textFont = UIFont.systemFont(ofSize: textFontSize)
        let textAttrs: [NSAttributedString.Key: Any] = [
            .font: textFont,
            .foregroundColor: textColor,
        ]

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

    private func makeFormulaAttachment(_ latex: String) -> RaTeXTextAttachment? {
        do {
            let dl = try RaTeXEngine.shared.parse(
                latex,
                displayMode: false,
                color: formulaColor,
                traitCollection: traitCollection
            )
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
        var prevWasBackslash = false

        for ch in content {
            if prevWasBackslash {
                prevWasBackslash = false
                if ch == "$" {
                    current.append("$")
                    continue
                }
                current.append("\\")
            }

            if ch == "\\" && !inFormula {
                prevWasBackslash = true
                continue
            }

            if ch == "$" {
                if inFormula {
                    if !current.isEmpty {
                        segments.append(.formula(current))
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
        }

        if prevWasBackslash {
            current.append("\\")
        }

        if !current.isEmpty {
            if inFormula {
                segments.append(.text("$\(current)"))
            } else {
                segments.append(.text(current))
            }
        }

        return segments
    }

    // MARK: - Trait changes

    public override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        guard let previousTraitCollection else { return }
        guard traitCollection.hasDifferentColorAppearance(comparedTo: previousTraitCollection) else {
            return
        }
        rebuild()
    }
}
