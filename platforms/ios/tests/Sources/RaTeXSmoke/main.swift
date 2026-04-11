import Foundation
import RaTeXCore

@main
struct RaTeXSmoke {
    static func main() throws {
        let engine = RaTeXEngine.shared

        let cases: [(String, String)] = [
            ("分式", #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#),
            ("积分", #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#),
            ("矩阵", #"\begin{pmatrix}a&b\\c&d\end{pmatrix}"#),
            ("求和", #"\sum_{n=1}^\infty \frac{1}{n^2} = \frac{\pi^2}{6}"#),
            ("上下标", #"x^2 + y^2 = z^2"#),
            ("mhchem", #"\ce{CO2 + H2O <=> H2CO3}"#),
        ]

        for (label, latex) in cases {
            let dl = try engine.parse(latex, displayMode: true)
            print("[\(label)] width=\(String(format: "%.3f", dl.width))em height=\(String(format: "%.3f", dl.height))em depth=\(String(format: "%.3f", dl.depth))em items=\(dl.items.count)")
        }

        // Error handling: invalid LaTeX should throw
        do {
            _ = try engine.parse(#"\frac{1}"#, displayMode: true)
            throw NSError(domain: "ratex-smoke", code: 1, userInfo: [NSLocalizedDescriptionKey: "expected parse error but got success"])
        } catch {
            print("[错误处理] OK: \(error)")
        }

        print("✅ ratex-smoke OK")
    }
}

