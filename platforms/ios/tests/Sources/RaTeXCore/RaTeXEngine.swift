// RaTeXEngine.swift (macOS / test) — calls the C ABI directly (no UIKit dependency).

import Foundation
import CRaTeX

public enum RaTeXError: Error, LocalizedError {
    case parseError(String)

    public var errorDescription: String? {
        if case .parseError(let msg) = self { return "RaTeX: \(msg)" }
        return nil
    }
}

public final class RaTeXEngine {
    public static let shared = RaTeXEngine()
    private init() {}

    public func parse(_ latex: String, displayMode: Bool = true) throws -> DisplayList {
        var opts = RatexOptions(
            struct_size: MemoryLayout<RatexOptions>.size,
            display_mode: displayMode ? 1 : 0
        )
        let result = ratex_parse_and_layout(latex, &opts)
        guard result.error_code == 0, let ptr = result.data else {
            let msg = ratex_get_last_error().map { String(cString: $0) } ?? "unknown error"
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
