// RaTeXEngine.swift — Swift wrapper around the ratex_parse_and_layout C ABI.

import Foundation
import RaTeXFFI
import UIKit

private func ffiColor(from color: UIColor, traitCollection: UITraitCollection? = nil) -> RatexColor {
    let resolved = traitCollection.map { color.resolvedColor(with: $0) } ?? color
    var r: CGFloat = 0
    var g: CGFloat = 0
    var b: CGFloat = 0
    var a: CGFloat = 0
    if resolved.getRed(&r, green: &g, blue: &b, alpha: &a) {
        return RatexColor(r: Float(r), g: Float(g), b: Float(b), a: Float(a))
    }

    let fallbackSpace = CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB()
    guard
        let converted = resolved.cgColor.converted(
            to: fallbackSpace,
            intent: .defaultIntent,
            options: nil
        ),
        let components = converted.components,
        components.count >= 4
    else {
        return RatexColor(r: 0, g: 0, b: 0, a: 1)
    }

    return RatexColor(
        r: Float(components[0]),
        g: Float(components[1]),
        b: Float(components[2]),
        a: Float(components[3])
    )
}

// MARK: - Error type

public enum RaTeXError: Error, LocalizedError {
    case parseError(String)
    case nullResult

    public var errorDescription: String? {
        switch self {
        case .parseError(let msg): return "RaTeX parse error: \(msg)"
        case .nullResult:          return "RaTeX returned null with no error message"
        }
    }
}

// MARK: - Engine

/// Thread-safe entry point for RaTeX rendering.
///
/// ```swift
/// let displayList = try RaTeXEngine.shared.parse(#"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#)
/// ```
public final class RaTeXEngine {
    public static let shared = RaTeXEngine()
    private init() {}

    /// Parse a LaTeX string and return the corresponding `DisplayList`.
    ///
    /// This call is synchronous and CPU-bound; run it on a background queue for
    /// complex formulas.
    ///
    /// - Parameters:
    ///   - latex: A LaTeX math-mode string, e.g. `\frac{1}{2}`.
    ///   - displayMode: `true` (default) for display/block style (`$$...$$`);
    ///     `false` for inline/text style (`$...$`).
    ///   - color: Default formula color. Explicit LaTeX colors still take precedence.
    /// - Returns: A `DisplayList` ready to be drawn.
    /// - Throws: `RaTeXError.parseError` on invalid LaTeX syntax.
    public func parse(
        _ latex: String,
        displayMode: Bool = true,
        color: UIColor = .black
    ) throws -> DisplayList {
        try parse(
            latex,
            displayMode: displayMode,
            color: color,
            traitCollection: nil
        )
    }

    func parse(
        _ latex: String,
        displayMode: Bool,
        color: UIColor,
        traitCollection: UITraitCollection?
    ) throws -> DisplayList {
        var ffiDefaultColor = ffiColor(from: color, traitCollection: traitCollection)
        let result = withUnsafePointer(to: &ffiDefaultColor) { colorPtr in
            var opts = RatexOptions(
                struct_size: MemoryLayout<RatexOptions>.size,
                display_mode: displayMode ? 1 : 0,
                color: colorPtr
            )
            return ratex_parse_and_layout(latex, &opts)
        }
        guard result.error_code == 0, let ptr = result.data else {
            let msg: String
            if let errPtr = ratex_get_last_error() {
                msg = String(cString: errPtr)
            } else {
                msg = "unknown error"
            }
            throw RaTeXError.parseError(msg)
        }
        defer { ratex_free_display_list(ptr) }

        let json = String(cString: ptr)
        do {
            return try JSONDecoder().decode(DisplayList.self, from: Data(json.utf8))
        } catch {
            throw RaTeXError.parseError("JSON decode failed: \(error)")
        }
    }
}
