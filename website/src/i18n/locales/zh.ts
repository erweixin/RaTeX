import type { TranslationDict } from "../types";

export const zh: TranslationDict = {
  nav: {
    features: "特性",
    demo: "演示",
    math: "数学",
    chemistry: "化学",
    physics: "物理",
    getStarted: "快速上手",
    langEn: "EN",
    langZh: "中文",
  },
  footer: {
    copyright: "© 2026 RaTeX · MIT · 用 Rust 构建",
  },
  gallery: {
    initializing: "初始化中…",
    rendered: "已渲染：",
  },
  heroPlayground: {
    loading: "加载中…",
    enterLatexAbove: "请在上方输入 LaTeX。",
  },
  home: {
    eyebrow: "Rust · WASM · 原生",
    heading: "单一 Rust 排版核心，TeX 级别数学渲染",
    descMain:
      "RaTeX 解析 LaTeX 数学，应用 TeX 排版规则，并输出扁平的显示列表，支持 CoreGraphics、Skia、Canvas 2D 或自定义向量后端——原生 FFI 与 WebAssembly 输出完全一致。",
    alignmentLabel: "对齐性：",
    alignmentBody:
      "RaTeX 致力于在关键方面与 KaTeX 保持一致：CI 针对参考图片运行大型基准测试集，在该语料库上的输出与 KaTeX 广泛可比。",
    alignmentLink: "支持表",
    alignmentSuffix: "以并排方式展示完整基准公式列表与 KaTeX 的对比。",
    whereFitsLabel: "适用场景：",
    whereFitsPrefix: "对于普通网页中的数学公式，DOM 中的",
    whereFitsMid:
      "仍是不错的默认选择。RaTeX 面向原生应用、服务器以及无 WebView 的嵌入场景——从移动端到 WASM 使用同一引擎。",
    tryIt: "立即体验",
    packagesEyebrow: "包",
    packagesHeading: "随处集成",
    packagesDescPrefix:
      "来自同一 Rust 核心的即用 SDK 与 WASM 构建：从 npm、Maven、pub.dev 或 SPM 安装，详见",
    packagesGetStarted: "快速上手",
    packagesDescSuffix: "。服务端 PNG 与 CLI 也在其中。",
    whenToUseHeading: "何时选择 RaTeX",
    whenToUseNative: "原生或服务器",
    whenToUseNativeDesc:
      "在 iOS、Android、Flutter 或 Rust 服务（PNG/SVG 光栅化）上使用相同排版，无需打包浏览器。",
    whenToUseWasm: "WASM 嵌入",
    whenToUseWasmDescPrefix:
      "在 WebAssembly 中运行核心并用 Canvas 绘制；在",
    whenToUseWasmLink: "实时演示",
    whenToUseWasmDescSuffix: "中与 KaTeX 对比输出。",
    whenToUseChem: "化学与单位",
    whenToUseChemDescSuffix:
      "在 mhchem 兼容路径上与普通数学并用（见下方图库）。",
    rustCoreHeading: "Rust 核心",
    rustCoreDesc:
      "单一排版引擎，热路径无 GC：为移动 UI、服务器和 CI 光栅测试提供可预期的计时。",
    shipEverywhereHeading: "随处部署",
    shipEverywhereDesc:
      "为 Swift、Kotlin、Dart 等提供 C ABI，为 Web 提供 WASM；tiny-skia 或自定义光栅化器——显示列表完全一致。",
    mhchemHeading: "mhchem 化学支持",
    mhchemDescPrefix: "内置",
    mhchemDescMid: "和",
    mhchemDescSuffix:
      "，通过 mhchem 兼容路径——反应箭头和物理单位与普通数学在同一流水线中。",
    galleriesEyebrow: "在浏览器中体验",
    galleriesHeading: "基准测试套件图库",
    galleriesDescPrefix:
      "浏览 CI 使用的相同 LaTeX 行，以 RaTeX WASM 在页面上渲染：",
    galleriesDemoPrefix: "与 KaTeX 并排对比，请打开",
    galleriesDemoLink: "交互演示",
    galleriesDemoMid: "；完整基准测试套件在演示页面的",
    galleriesSupportLink: "支持表",
    galleriesDemoSuffix: "中。",
    comparisonHeading: "为什么不用 WebView 方案？",
    comparisonDesc:
      "在浏览器中，KaTeX 和 MathJax 通常以 JavaScript 操作 DOM 运行。对于通过 WebView 嵌入数学的应用外壳，这仍意味着打包浏览器栈。RaTeX 将排版和光栅化保留在 Rust 中，适用于希望避免此路径的宿主。",
    comparisonRuntime: "运行时",
    comparisonMobile: "移动端",
    comparisonOffline: "离线",
    comparisonJsBundle: "JS 包体积（典型）",
    comparisonMemory: "内存模型",
    nativeSdkHeading: "RaTeX 与原生数学 SDK 对比",
    nativeSdkDesc:
      "没有 WebView 时，团队通常会使用 Swift、Objective-C 或 Flutter 库。以下是与常用开源渲染器的高层对比——swiftMath（Swift）、flutter_math_fork / flutter_math（Dart / Flutter）和 iosMath（iOS）——涵盖化学宏、可移植性和引擎架构。第三方 SDK 独立演进；集成时请对比版本。",
    nativeSdkFootnote:
      `*性能取决于工作负载。Swift 使用 ARC；Dart 使用追踪式 GC——对于"无浏览器"嵌入场景，两者都与 RaTeX 的 Rust 核心不同。`,
    capabilityLabel: "能力",
    sameEngineFfi: "相同引擎：原生 FFI + WASM（Web）",
    sameEngineRust: "单一 Rust 核心支持移动端 + 桌面",
    rustLayoutCore: "Rust TeX 排版核心（可预期热路径）",
    ctaHeading: "无需嵌入浏览器引擎，直接发布科学 UI",
    ctaLiveDemo: "实时演示",
    ctaGithubReadme: "GitHub README",
  },
  getStarted: {
    eyebrow: "集成",
    heading: "按平台快速上手",
    intro:
      "每个目标都使用来自 Rust 流水线的相同显示列表。选择您的技术栈，然后在 GitHub 上打开完整指南了解版本控制、字体和原生构建脚本。",
    tryBrowserFirst: "更喜欢先在浏览器中试用公式？",
    liveDemoLink: "实时演示",
    mathGalleryLink: "数学图库",
    jumpTo: "跳转到",
    fullDoc: "完整文档",
    architectureHeading: "架构概述",
    architectureDescPrefix:
      "所有路径共享：词法分析 → 语法分析 → 排版 → 显示列表。原生 UI 和 WASM 将该列表映射到 CoreGraphics、Android Canvas、Flutter",
    architectureDescSuffix: "、Skia 或 Canvas 2D；服务器 crate 使用 tiny-skia 光栅化。",
    architectureLink: "README — 架构",
    platforms: [
      {
        title: "Web (WASM)",
        blurb:
          "将 Rust 编译为 WebAssembly；Canvas 2D 绘制显示列表。使用已发布的 npm 包和可选的 ratex-formula Web 组件。",
        steps: [
          "安装：`npm install ratex-wasm`",
          "从包中加载 KaTeX 字体 CSS 并注册自定义元素或调用编程式 API。",
        ],
      },
      {
        title: "iOS (Swift)",
        blurb:
          "通过 C ABI 构建 Swift / SwiftUI 视图；CoreGraphics 渲染显示列表。通过 GitHub 仓库使用 SPM。",
        steps: [
          "在 Xcode 中：File → Add Package Dependencies → `https://github.com/erweixin/RaTeX`，选择 RaTeX 产品。",
          "使用 `RaTeXFormula` / `RaTeXView`；字体在首次渲染时从包中加载。",
        ],
      },
      {
        title: "Android (Kotlin)",
        blurb:
          "AAR 通过 JNI 链接到相同的原生库；Canvas 绘制字形和规则。已发布到 Maven 坐标。",
        steps: [
          "添加 `implementation(\"io.github.erweixin:ratex-android:…\")`（当前版本见 README）。",
          "在 XML 或 Compose 中放置 `RaTeXView` 并在代码中设置 `latex` / `fontSize`。",
        ],
      },
      {
        title: "Flutter (Dart FFI)",
        blurb:
          "Dart FFI 链接到 `libratex_ffi`；`CustomPainter` 渲染显示列表。预构建的 iOS XCFramework + Android `.so` 在 pub.dev 上。",
        steps: [
          "在 `pubspec.yaml` 中添加 `ratex_flutter` 并运行 `flutter pub get`。",
          "在应用的 `pubspec.yaml` 的 `flutter: fonts:` 节中用 `packages/ratex_flutter/` 资源前缀注册 KaTeX 字体——缺少此步骤字形将静默回退到系统字体。完整声明片段见文档。",
          "使用 `RaTeXWidget(latex: r'…', fontSize: 28)`。",
        ],
      },
      {
        title: "React Native",
        blurb:
          "iOS 和 Android 上的原生视图；JS 打包 UI，Rust 在 `.a` / `.so` 中处理解析/排版。",
        steps: [
          "安装：`npm install ratex-react-native`，然后 `cd ios && pod install`。",
          "使用 `RaTeXView` / `InlineTeX`；字体随包一起发布。",
        ],
      },
      {
        title: "Server / CLI",
        blurb:
          "使用 tiny-skia 光栅化为 PNG（`ratex-render`）或用 `ratex-svg` 导出自包含 SVG——CI 快照、后端或无头服务器——无需浏览器。",
        steps: [
          "PNG：将 LaTeX 通过管道传入标准输入 — `cargo run --release -p ratex-render`。",
          "SVG：添加 `--features cli` — `cargo run --release -p ratex-svg --features cli`，输出基于 `<path>` 的 SVG，无需网络字体依赖。",
        ],
      },
    ],
  },
  demo: {
    eyebrow: "立即体验",
    heading: "演示与基准测试",
    intro: "与生产构建相同的 RaTeX WASM；KaTeX 0.16.45 是这些页面的参考渲染器。",
    suggestedOrderLabel: "建议顺序",
    suggestedOrderDescPrefix: "从",
    suggestedOrderLiveLink: "实时对比",
    suggestedOrderDescMid: "开始测试单个公式，然后打开",
    suggestedOrderTableLink: "支持表",
    suggestedOrderDescSuffix: "扫描全部 1058 行，最后在需要分类浏览时使用图库。",
    howItLoadsLabel: "加载方式：",
    howItLoadsDesc:
      "KaTeX 0.16.45 CSS/JS 来自 jsDelivr。RaTeX 使用本站的 platforms/web/（WASM + 字体）。在 GitHub Pages 上会随部署一起发布；本地请构建 WASM 并使用开发服务器——见",
    howItLoadsGetStartedLink: "快速上手 → Web",
    liveComparisonTitle: "实时对比",
    liveComparisonSubtitle: "RaTeX WASM vs KaTeX 0.16.45",
    liveComparisonBody:
      "编辑一行 LaTeX 并并排比较 RaTeX Canvas 输出与 KaTeX——状态、错误以及与图库相同的 WASM 包。",
    liveComparisonCta: "打开交互演示",
    supportTableTitle: "支持表",
    supportTableSubtitle: "1058 个基准公式",
    supportTableBody:
      "打开全页基准测试：每个基准测试套件行与 KaTeX 0.16.45 对比，批量 IoU 分数以及浏览器中实时的 RaTeX 列——最适合覆盖率和回归分类。",
    supportTableCta: "打开完整支持表",
    galleriesEyebrow: "相同 WASM · 不同界面",
    galleriesHeading: "基准测试套件图库",
    galleriesDesc:
      "与站点导航栏相同的目标——带有源码和 Canvas 的长列表，懒加载，适合抽查多个公式。",
    galleriesOpen: "打开",
    footerText: "在应用中集成 RaTeX：",
    footerLink: "按平台快速上手",
    galleryLabels: {
      math: "数学",
      chemistry: "化学",
      physics: "物理",
    },
    galleryHints: {
      math: "KaTeX 风格分节 · 900+ 行",
      chemistry: "mhchem \\ce",
      physics: "\\pu 及精选",
    },
  },
  demoLive: {
    eyebrow: "立即体验",
    heading: "实时对比",
    desc: "在下方输入 LaTeX，对比 KaTeX（参考）与 RaTeX（Rust → WASM → Canvas）。与图库使用相同的包。",
    inputPlaceholder: "输入 LaTeX…",
    renderBtn: "渲染",
    statusLoading: "加载中…",
    waitingForInput: "等待输入…",
    quickTryLabel: "快速体验",
    activeDevText: "RaTeX 正在积极开发中。发现问题了吗？",
    openIssueLink: "提交 Issue",
    examplesLabel: "示例——点击卡片加载",
  },
  supportTable: {
    eyebrow: "基准测试",
    heading: "公式支持表",
    desc: "RaTeX（Rust + WASM）与 KaTeX 0.16.45 对比 1058 个基准测试套件行（含 mhchem \\ce / \\pu）。离线格使用预计算的墨水 IoU 与 KaTeX 参考 PNG 对比；RaTeX 列由您浏览器中加载的 WASM 实时计算。",
    dataSourceLabel: "数据来源",
    dataSourceDescPrefix:
      "批量离线分数和聚合计数在 CI 运行中重新生成，可能比最新的",
    dataSourceDescMid:
      "滞后几个提交。每行 RaTeX 值始终反映您刚加载的 WASM。如需单个公式验证，请使用",
    dataSourceLiveLink: "实时对比",
    scoreGreat: "分数 ≥ 0.9",
    scoreHigh8: "0.8–0.9",
    scoreMidHi: "0.5–0.8",
    scoreMidLo: "0.3–0.5",
    scoreLow: "< 0.3 或错误",
    scoreAvg: "平均分",
    filterAll: "全部",
    filterGreat: "≥ 0.9",
    filterHigh8: "0.8–0.9",
    filterMidHi: "0.5–0.8",
    filterMidLo: "0.3–0.5",
    filterLow: "< 0.3 / 错误",
    searchPlaceholder: "搜索 LaTeX…",
    initializing: "初始化中…",
    colNum: "#",
    colLatex: "LaTeX 源码",
    colKatex: "KaTeX（参考）",
    colRatex: "RaTeX（WASM）",
    colScore: "分数",
    scoresDesc:
      "分数来自离线基准对比（RaTeX 服务器 PNG 与 KaTeX 参考）。RaTeX 列在本页使用 WASM 按需渲染；字体与图库设置一致。再次提醒：基准流水线输出可能滞后于仓库——见上方说明。",
    offlineIouNote: "离线 IoU 与 KaTeX PNG 对比",
    referencesLabel: "参考",
    liveCompLink: "实时对比",
    inkIou: "墨水覆盖率 IoU 与 KaTeX PNG 对比",
  },
  mathGallery: {
    eyebrow: "图库 · 基准测试套件",
    title: "数学",
    desc1prefix: "每行来自",
    desc1suffix: "——与 CI 光栅测试使用相同的输入。",
    desc2prefix: "各节遵循",
    desc2link: "KaTeX 支持函数",
    desc2suffix:
      "的主题顺序（重音、分隔符、环境……）。每个卡片上方显示源码，下方显示输出；格子使用响应式网格并在滚动时渲染。",
    ariaLabel: "数学公式网格",
  },
  chemGallery: {
    eyebrow: "图库 · mhchem",
    title: "化学",
    desc1prefix: "来自",
    desc1mid: "中使用",
    desc1suffix: "的行，包括混合数学 + 化学。这些反映了基准光栅测试覆盖的路径。",
    desc2prefix: "同时包含",
    desc2mid: "的行也可能出现在",
    desc2link: "物理",
    desc2suffix: "图库中。",
    ariaLabel: "化学公式网格",
  },
  physicsGallery: {
    eyebrow: "图库 · 单位与方程",
    title: "物理",
    desc1prefix: "来自",
    desc1mid: "中的所有",
    desc1suffix: "行，加上一组精选经典公式（如薛定谔方程、麦克斯韦方程）用于视觉冒烟测试。",
    desc2prefix: "对于化学专用",
    desc2mid: "覆盖，请参阅",
    desc2link: "化学",
    desc2suffix: "图库。",
    ariaLabel: "物理公式网格",
  },
};
