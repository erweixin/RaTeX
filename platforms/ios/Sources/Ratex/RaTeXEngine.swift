// RaTeXEngine.swift — Swift wrapper around the ratex_parse_and_layout C ABI.

import Foundation
import RaTeXFFI

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
    /// - Parameter latex: A LaTeX math-mode string, e.g. `\frac{1}{2}`.
    /// - Returns: A `DisplayList` ready to be drawn.
    /// - Throws: `RaTeXError.parseError` on invalid LaTeX syntax.
    public func parse(_ latex: String) throws -> DisplayList {
        guard let ptr = ratex_parse_and_layout(latex) else {
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
