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
        return Self.resolveMacTextFont(
            family: trimmedTextFontFamily,
            size: textFontSize,
            italic: textItalic
        )
        #else
        return Self.resolveIOSTextFont(
            family: trimmedTextFontFamily,
            size: textFontSize,
            italic: textItalic
        )
        #endif
    }

    #if os(macOS)
    private static func resolveMacTextFont(
        family: String?,
        size: CGFloat,
        italic: Bool
    ) -> NSFont {
        guard let family, !family.isEmpty else {
            return italic
                ? NSFontManager.shared.convert(NSFont.systemFont(ofSize: size), toHaveTrait: .italicFontMask)
                : NSFont.systemFont(ofSize: size)
        }

        if let font = bestMacFont(from: macFontNames(for: family), size: size, italic: italic) {
            return font
        }
        if let font = NSFont(name: family, size: size) {
            return italic ? macFontByApplyingItalic(font, size: size) : font
        }
        if let font = bestMacFont(from: fuzzyMacFontNames(matching: family), size: size, italic: italic) {
            return font
        }

        return italic
            ? NSFontManager.shared.convert(NSFont.systemFont(ofSize: size), toHaveTrait: .italicFontMask)
            : NSFont.systemFont(ofSize: size)
    }

    private static func macFontNames(for family: String) -> [String] {
        if let members = NSFontManager.shared.availableMembers(ofFontFamily: family), !members.isEmpty {
            return members.compactMap { $0.first as? String }
        }
        let matchedFamily = NSFontManager.shared.availableFontFamilies.first {
            fontIdentifierMatches($0, family)
        }
        guard let matchedFamily,
              let members = NSFontManager.shared.availableMembers(ofFontFamily: matchedFamily)
        else { return [] }
        return members.compactMap { $0.first as? String }
    }

    private static var fuzzyMacFontNamesCache: [String: [String]] = [:]

    private static func fuzzyMacFontNames(matching query: String) -> [String] {
        if let cached = fuzzyMacFontNamesCache[query] { return cached }
        let result = NSFontManager.shared.availableFonts.filter {
            fontIdentifierMatches($0, query)
        }
        fuzzyMacFontNamesCache[query] = result
        return result
    }

    private static func bestMacFont(from names: [String], size: CGFloat, italic: Bool) -> NSFont? {
        names.reduce(nil as NSFont?) { best, name in
            guard let font = NSFont(name: name, size: size) else { return best }
            guard let currentBest = best else { return font }
            return macFontScore(font, italic: italic) < macFontScore(currentBest, italic: italic)
                ? font : currentBest
        }.map { italic ? macFontByApplyingItalic($0, size: size) : $0 }
    }

    private static func macFontScore(_ font: NSFont, italic: Bool) -> Int {
        var score = isMacItalicFont(font) == italic ? 0 : 1_000
        let name = font.fontName.lowercased()
        if italic {
            if name.contains("italic") || name.contains("oblique") {
                score -= 100
            }
        } else {
            if name.contains("regular") || name.contains("roman") || name.contains("normal") {
                score -= 100
            }
            if name.contains("bold") || name.contains("black") || name.contains("heavy") {
                score += 50
            }
        }
        return score
    }

    private static func isMacItalicFont(_ font: NSFont) -> Bool {
        NSFontManager.shared.traits(of: font).contains(.italicFontMask)
    }

    private static func macFontByApplyingItalic(_ font: NSFont, size: CGFloat) -> NSFont {
        guard !isMacItalicFont(font) else { return font }
        return NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask)
    }
    #else
    private static func resolveIOSTextFont(
        family: String?,
        size: CGFloat,
        italic: Bool
    ) -> UIFont {
        guard let family, !family.isEmpty else {
            return italic ? UIFont.italicSystemFont(ofSize: size) : UIFont.systemFont(ofSize: size)
        }

        if let font = bestUIFont(from: iosFontNames(for: family), size: size, italic: italic) {
            return font
        }
        if let font = UIFont(name: family, size: size) {
            return italic ? uiFontByApplyingItalic(font, size: size) : font
        }
        if let font = bestUIFont(from: fuzzyIOSFontNames(matching: family), size: size, italic: italic) {
            return font
        }

        return italic ? UIFont.italicSystemFont(ofSize: size) : UIFont.systemFont(ofSize: size)
    }

    private static func iosFontNames(for family: String) -> [String] {
        let names = UIFont.fontNames(forFamilyName: family)
        if !names.isEmpty { return names }

        guard let matchedFamily = UIFont.familyNames.first(where: {
            fontIdentifierMatches($0, family)
        }) else { return [] }
        return UIFont.fontNames(forFamilyName: matchedFamily)
    }

    private static var fuzzyIOSFontNamesCache: [String: [String]] = [:]

    private static func fuzzyIOSFontNames(matching query: String) -> [String] {
        if let cached = fuzzyIOSFontNamesCache[query] { return cached }
        let result = UIFont.familyNames
            .flatMap { UIFont.fontNames(forFamilyName: $0) }
            .filter { fontIdentifierMatches($0, query) }
        fuzzyIOSFontNamesCache[query] = result
        return result
    }

    private static func bestUIFont(from names: [String], size: CGFloat, italic: Bool) -> UIFont? {
        names.reduce(nil as UIFont?) { best, name in
            guard let font = UIFont(name: name, size: size) else { return best }
            guard let currentBest = best else { return font }
            return uiFontScore(font, italic: italic) < uiFontScore(currentBest, italic: italic)
                ? font : currentBest
        }.map { italic ? uiFontByApplyingItalic($0, size: size) : $0 }
    }

    private static func uiFontScore(_ font: UIFont, italic: Bool) -> Int {
        var score = isUIItalicFont(font) == italic ? 0 : 1_000
        let name = font.fontName.lowercased()
        if italic {
            if name.contains("italic") || name.contains("oblique") {
                score -= 100
            }
        } else {
            if name.contains("regular") || name.contains("roman") || name.contains("normal") {
                score -= 100
            }
            if name.contains("bold") || name.contains("black") || name.contains("heavy") {
                score += 50
            }
        }
        return score
    }

    private static func isUIItalicFont(_ font: UIFont) -> Bool {
        let traits = font.fontDescriptor.symbolicTraits
        return traits.contains(.traitItalic)
            || font.fontName.lowercased().contains("italic")
            || font.fontName.lowercased().contains("oblique")
    }

    private static func uiFontByApplyingItalic(_ font: UIFont, size: CGFloat) -> UIFont {
        guard !isUIItalicFont(font) else { return font }
        var traits = font.fontDescriptor.symbolicTraits
        traits.insert(.traitItalic)
        if let descriptor = font.fontDescriptor.withSymbolicTraits(traits) {
            return UIFont(descriptor: descriptor, size: size)
        }
        return UIFont.italicSystemFont(ofSize: size)
    }
    #endif

    private static func fontIdentifierMatches(_ lhs: String, _ rhs: String) -> Bool {
        if lhs.caseInsensitiveCompare(rhs) == .orderedSame {
            return true
        }
        let normalizedLhs = normalizedFontIdentifier(lhs)
        let normalizedRhs = normalizedFontIdentifier(rhs)
        if !normalizedLhs.isEmpty && normalizedLhs == normalizedRhs {
            return true
        }
        let compactLhs = compactFontIdentifier(lhs)
        let compactRhs = compactFontIdentifier(rhs)
        return !compactLhs.isEmpty && compactLhs == compactRhs
    }

    private static func normalizedFontIdentifier(_ value: String) -> String {
        value
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(
                of: "[^a-z0-9]+",
                with: "_",
                options: .regularExpression
            )
            .trimmingCharacters(in: CharacterSet(charactersIn: "_"))
    }

    private static func compactFontIdentifier(_ value: String) -> String {
        value
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(
                of: "[^a-z0-9]+",
                with: "",
                options: .regularExpression
            )
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
