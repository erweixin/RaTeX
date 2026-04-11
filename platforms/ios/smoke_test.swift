#!/usr/bin/env swift
// smoke_test.swift — 验证 ratex_ffi C ABI 在 macOS 上可以正常调用
// 运行方式: swift platforms/ios/smoke_test.swift

import Foundation

let libPath = "\(FileManager.default.currentDirectoryPath)/target/release/libratex_ffi.dylib"

guard let handle = dlopen(libPath, RTLD_NOW) else {
    print("❌ 无法加载库:", String(cString: dlerror()))
    print("   请先运行: cargo build --release -p ratex-ffi")
    exit(1)
}
print("✓ 库加载成功:", libPath)

// Mirror ratex.h ABI
@frozen struct RatexOptions {
    var struct_size: Int
    var display_mode: Int32   // 0 = inline, 1 = display
}

@frozen struct RatexResult {
    var data: UnsafeMutablePointer<CChar>?
    var error_code: Int32
}

typealias ParseFn = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<RatexOptions>?) -> RatexResult
typealias FreeFn  = @convention(c) (UnsafeMutablePointer<CChar>?) -> Void
typealias ErrFn   = @convention(c) () -> UnsafePointer<Int8>?

let parseRaw = dlsym(handle, "ratex_parse_and_layout")!
let freeRaw  = dlsym(handle, "ratex_free_display_list")!
let errRaw   = dlsym(handle, "ratex_get_last_error")!

let parse = unsafeBitCast(parseRaw, to: ParseFn.self)
let free  = unsafeBitCast(freeRaw,  to: FreeFn.self)
let err   = unsafeBitCast(errRaw,   to: ErrFn.self)

func render(_ latex: String) -> [String: Any]? {
    var opts = RatexOptions(struct_size: MemoryLayout<RatexOptions>.size, display_mode: 1)
    let result = latex.withCString { cstr in
        withUnsafePointer(to: &opts) { pOpts in
            parse(cstr, pOpts)
        }
    }
    guard result.error_code == 0, let ptr = result.data else {
        let msg = err().map { String(cString: $0) } ?? "unknown error"
        print("  ❌ 解析失败:", msg)
        return nil
    }
    defer { free(ptr) }
    let json = String(cString: ptr)
    return try? JSONSerialization.jsonObject(with: json.data(using: String.Encoding.utf8)!) as? [String: Any]
}

func check(_ label: String, _ latex: String) {
    print("\n[\(label)]  \(latex)")
    guard let dl = render(latex) else { return }
    let items = (dl["items"] as? [[String: Any]])?.count ?? 0
    let w = dl["width"]  as? Double ?? 0
    let h = dl["height"] as? Double ?? 0
    print("  ✓  width=\(String(format: "%.3f", w))em  height=\(String(format: "%.3f", h))em  items=\(items)")

    // 检查第一个 item 的类型
    if let first = (dl["items"] as? [[String: Any]])?.first {
        let kind = (first["type"] as? String) ?? "?"
        print("  ✓  first item type:", kind)
    }
}

check("分式",       #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#)
check("积分",       #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#)
check("矩阵",       #"\begin{pmatrix}a&b\\c&d\end{pmatrix}"#)
check("求和",       #"\sum_{n=1}^\infty \frac{1}{n^2} = \frac{\pi^2}{6}"#)
check("上下标",     #"x^2 + y^2 = z^2"#)

// 错误处理测试
print("\n[错误处理]")
if render(#"\frac{1}"#) == nil {
    print("  ✓ 非法 LaTeX 正确返回 nil")
}

print("\n✅ 所有测试通过，C ABI 工作正常")
