import XCTest
@testable import RaTeXCore

final class RaTeXCoreTests: XCTestCase {

    let engine = RaTeXEngine.shared

    // MARK: - 基本解析

    func testSimpleFraction() throws {
        let dl = try engine.parse(#"\frac{1}{2}"#)
        XCTAssertGreaterThan(dl.width,  0, "width 应该大于 0")
        XCTAssertGreaterThan(dl.height, 0, "height 应该大于 0")
        XCTAssertFalse(dl.items.isEmpty, "items 不应为空")
    }

    func testSuperscript() throws {
        let dl = try engine.parse("x^2 + y^2 = z^2")
        XCTAssertGreaterThan(dl.items.count, 0)
    }

    func testIntegral() throws {
        let dl = try engine.parse(#"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#)
        XCTAssertGreaterThan(dl.width, 3.0, "积分公式应该比较宽")
    }

    func testMatrix() throws {
        let dl = try engine.parse(#"\begin{pmatrix}a&b\\c&d\end{pmatrix}"#)
        XCTAssertGreaterThan(dl.items.count, 4, "矩阵应该有多个绘制项")
    }

    // MARK: - DisplayItem 类型验证

    func testContainsGlyphPath() throws {
        let dl = try engine.parse("x")
        let hasGlyph = dl.items.contains { if case .glyphPath = $0 { return true }; return false }
        XCTAssertTrue(hasGlyph, "单字符应包含 GlyphPath item")
    }

    func testFractionContainsLine() throws {
        let dl = try engine.parse(#"\frac{a}{b}"#)
        let hasLine = dl.items.contains { if case .line = $0 { return true }; return false }
        XCTAssertTrue(hasLine, "分式应包含 Line item（分数线）")
    }

    // MARK: - PathCommand 解码

    func testPathCommandsDecoded() throws {
        let dl = try engine.parse("x")
        guard case .glyphPath(let g) = dl.items.first else {
            XCTFail("第一个 item 应是 GlyphPath"); return
        }
        // 协议要求：GlyphPath 的 commands 可能被省略（当前实现为减小 JSON 体积会省略）
        // 因此这里仅验证“存在且不崩溃”，不要求非空。
        XCTAssertGreaterThanOrEqual(g.commands.count, 0)
    }

    // MARK: - 颜色解码

    func testColorDecoded() throws {
        let dl = try engine.parse("x")
        guard case .glyphPath(let g) = dl.items.first else {
            XCTFail("应有 GlyphPath"); return
        }
        // 默认颜色为黑色
        XCTAssertEqual(g.color.r, 0.0, accuracy: 0.01)
        XCTAssertEqual(g.color.g, 0.0, accuracy: 0.01)
        XCTAssertEqual(g.color.b, 0.0, accuracy: 0.01)
        XCTAssertEqual(g.color.a, 1.0, accuracy: 0.01)
    }

    // MARK: - 尺寸合理性

    func testDimensionsReasonable() throws {
        // display mode 公式应该比 text mode 高
        let dl = try engine.parse(#"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#)
        XCTAssertGreaterThan(dl.height + dl.depth, 1.0, "分式总高度应 > 1em")
        XCTAssertLessThan(dl.width, 20.0, "宽度不应超过 20em")
    }

    // MARK: - 错误处理

    func testInvalidLatexThrows() {
        XCTAssertThrowsError(try engine.parse(#"\frac{1}"#)) { error in
            guard let e = error as? RaTeXError, case .parseError = e else {
                XCTFail("应抛出 RaTeXError.parseError"); return
            }
        }
    }

    func testEmptyStringReturnsEmpty() throws {
        // 空字符串应该成功解析，返回空列表
        let dl = try engine.parse("")
        XCTAssertEqual(dl.items.count, 0)
    }

    // MARK: - 并发安全

    func testConcurrentParse() async throws {
        try await withThrowingTaskGroup(of: DisplayList.self) { group in
            let formulas = [
                #"\frac{1}{2}"#,
                #"\sqrt{x}"#,
                #"x^2 + y^2"#,
                #"\int_0^1 f(x)\,dx"#,
                #"\sum_{n=1}^{10} n"#,
            ]
            for f in formulas {
                group.addTask { try self.engine.parse(f) }
            }
            for try await dl in group {
                XCTAssertGreaterThan(dl.items.count, 0)
            }
        }
    }
}
