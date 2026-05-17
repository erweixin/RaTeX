// RaTeXInlineRNView.swift — ObjC-compatible wrapper around RaTeXInlineView for React Native.

#if os(macOS)
import AppKit
#else
import UIKit
#endif

@objc(RaTeXInlineRNView)
@MainActor
public class RaTeXInlineRNView: PlatformView {

    private let innerView = RaTeXInlineView()
    private var bridgedColor: PlatformColor?
    private var bridgedTextColor: PlatformColor?

    // MARK: - ObjC-bridgeable properties

    @objc public var content: String {
        get { innerView.content }
        set { innerView.content = newValue; invalidateIntrinsicContentSize(); platformSetNeedsLayout() }
    }

    @objc public var fontSize: CGFloat {
        get { innerView.formulaFontSize }
        set { innerView.formulaFontSize = newValue; invalidateIntrinsicContentSize(); platformSetNeedsLayout() }
    }

    @objc public var color: PlatformColor? {
        get { bridgedColor }
        set {
            guard !isSameColor(newValue, bridgedColor) else { return }
            bridgedColor = newValue
            innerView.formulaColor = newValue ?? .black
        }
    }

    @objc public var textColor: PlatformColor? {
        get { bridgedTextColor }
        set {
            guard !isSameColor(newValue, bridgedTextColor) else { return }
            bridgedTextColor = newValue
            innerView.textColor = newValue ?? .black
        }
    }

    @objc public var textFontSize: CGFloat {
        get { innerView.textFontSize }
        set { innerView.textFontSize = newValue; invalidateIntrinsicContentSize(); platformSetNeedsLayout() }
    }

    @objc public var textFontFamily: NSString? {
        get { innerView.textFontFamily as NSString? }
        set { innerView.textFontFamily = newValue as String?; invalidateIntrinsicContentSize(); platformSetNeedsLayout() }
    }

    @objc public var textItalic: Bool {
        get { innerView.textItalic }
        set { innerView.textItalic = newValue; invalidateIntrinsicContentSize(); platformSetNeedsLayout() }
    }

    @objc public var textUnderline: Bool {
        get { innerView.textUnderline }
        set { innerView.textUnderline = newValue; invalidateIntrinsicContentSize(); platformSetNeedsLayout() }
    }

    @objc public var textLineThrough: Bool {
        get { innerView.textLineThrough }
        set { innerView.textLineThrough = newValue; invalidateIntrinsicContentSize(); platformSetNeedsLayout() }
    }

    // MARK: - Event callbacks

    @objc public var onContentSizeChange: ((NSDictionary?) -> Void)? {
        didSet {
            resetContentSizeReporting()
        }
    }

    @objc public func setContentSizeCallback(_ handler: ((CGFloat, CGFloat) -> Void)?) {
        contentSizeCallback = handler
        resetContentSizeReporting()
    }
    private var contentSizeCallback: ((CGFloat, CGFloat) -> Void)?

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

    @objc public func resetContentSizeReporting() {
        innerView.resetContentSizeReporting()
        invalidateIntrinsicContentSize()
        platformSetNeedsLayout()
    }

    // MARK: - Private

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

        if !RaTeXInlineRNView.fontsLoaded {
            RaTeXFontLoader.loadFromBundle(RaTeXInlineRNView.fontsBundle)
            RaTeXInlineRNView.fontsLoaded = true
        }

        innerView.onContentSizeChange = { [weak self] w, h in
            self?.onContentSizeChange?(["width": w, "height": h])
            self?.contentSizeCallback?(w, h)
        }
    }

    private static let fontsBundle: Bundle = {
        let module = Bundle(for: RaTeXInlineRNView.self)
        if let url = module.url(forResource: "RaTeXFonts", withExtension: "bundle"),
           let bundle = Bundle(url: url) {
            return bundle
        }
        return module
    }()

    private static var fontsLoaded = false

    private func isSameColor(_ a: PlatformColor?, _ b: PlatformColor?) -> Bool {
        if a == nil && b == nil { return true }
        guard let a, let b else { return false }
        return a.isEqual(b)
    }
}
