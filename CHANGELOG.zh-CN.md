# 更新日志

本文记录项目的重要变更。[English](CHANGELOG.md)

格式参考 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
版本遵循[语义化版本](https://semver.org/lang/zh-CN/)。

发布时（参见 [RELEASING.md](RELEASING.md)），`scripts/set-version.sh` 会将英文版的
`[Unreleased]` 改为新版本号和日期，并新建 `[Unreleased]` 节。中文版需同步更新。

## [Unreleased]

### 变更

- 依赖：`phf` 升至 0.13，`fancy-regex` 升至 0.20，与常见 GUI 依赖链一致，宿主不再各编译两份。
- `ratex-katex-fonts` 改用 `include_bytes!` 嵌入字体，不再依赖 `rust-embed` 及其 `sha2` 0.11 依赖树。`ttf_bytes` 签名不变。

### 新增

- `ratex-parser` 新增可选的 `mhchem` cargo feature（默认开启），所有依赖它的 crate 均转发该 feature。
  关闭默认 feature 即可构建不含 `\ce` / `\pu` 及 `regex` / `fancy-regex` 依赖的版本。
  详见 README 的“不含 mhchem 的构建”。

### 修复

- `\tag{...}` 的标签按文本模式解析，与 KaTeX（`\text{(...)}`）一致：`\tag{A}` 为直立体，
  数学内容需写在 `$...$` 中，`\tag` 也不能再嵌套在自身参数里。
- 两位及以上的自动公式编号显示全部数字，如 `(10)`，而不再显示为 `(1)`。

## [0.1.15] - 2026-10-16

### 兼容性

- `LayoutOptions` 保留 v0.1.14 的字段结构，`layout(nodes, options)` 签名不变。
  旧代码的完整结构体字面量和穷尽解构仍然有效，无需迁移。宿主字形度量通过新增的
  `layout_with_fallback_metrics` API 传入；C ABI 和 DisplayList JSON 保持不变。

### 新增

- `\slashed` 和 `\centernot` 命令。
- Apple XCFramework 和 Swift 包支持 Mac Catalyst。
- 新增 `layout_with_fallback_metrics(nodes, options, metrics)`，支持宿主提供回退字形
  度量，改善文本间距。用法见 API rustdoc，资源限制见[布局校验说明](docs/STACK_SAFETY.md)。
- CLI 支持配置 PNG/SVG 内边距。
- 新增 `Lexer::next_span()` 和 `TokenSpan`，支持借用式词法标记。

### 修复

- PNG 渲染正确返回损坏大字体的解析错误，不再以字形缺失的图片报告成功。
- PNG 在不同位置复用缓存时，字形偏移一个像素。
- 转义 `##` 的源位置导致 mhchem 空格回归。
- 线条、边框及相关符号的颜色继承问题。
- 网站在字体加载完成前开始渲染。
- 网站字体加载失败或超时后可直接重试，无需刷新页面。
- 定界符缓存不再在每次查询时序列化宿主提供的整张回退字形度量表。
- iOS 因栈展开符号导致链接失败。

### 安全

- 限制宏展开、mhchem 中间数据和布局工作量，覆盖 `\edef`/`\xdef`、`\pmb` 及 Array/CD 补齐。
- 拒绝溢出的 `\char` 值和超大 PNG 内存分配。
- 限制虚线渲染工作量，并裁剪至视口范围。
- 强制 Android ELF 按 16 KB 对齐，并校验发布包。

适用范围与限制见[资源限制说明](docs/EXPANSION_RESOURCE_LIMITS.md)。

### 性能

- 缓存字形掩码和 emoji 位图，共享字体缓冲区，按需加载回退字体。
- 复用内置宏和定界符测量结果，减少 PNG 编码和 SVG 序列化的内存分配。

与 v0.1.14 的最新对比（越低越好）：

| 指标 | v0.1.14 | 未发布版本 | 变化 |
|---|---:|---:|---:|
| PNG 渲染 | 717 μs | 339 μs | −53% |
| 独立 SVG | 938 μs | 403 μs | −57% |
| PNG 端到端耗时 | 1,030 μs | 506 μs | −51% |
| 峰值 RSS | 389.5 MiB | 218.6 MiB | −44% |

测试环境为 x86_64 macOS、Rust 1.95.0，两版使用相同依赖。
Release 模式下测试 100 个公式，每个公式预热一次、测量三次，取五轮交替运行的中位数。
RSS 不含编译过程。这些数据反映缓存预热后的性能；冷启动或字形位置持续变化时，结果可能不同。
