export const en = {
  nav: {
    features: "Features",
    demo: "Demo",
    math: "Math",
    chemistry: "Chemistry",
    physics: "Physics",
    getStarted: "Get started",
    langEn: "EN",
    langZh: "中文",
  },
  footer: {
    copyright: "© 2026 RaTeX · MIT · Built with Rust",
  },
  gallery: {
    initializing: "Initializing…",
    rendered: "Rendered:",
  },
  heroPlayground: {
    loading: "Loading…",
    enterLatexAbove: "Enter LaTeX above.",
  },
  home: {
    eyebrow: "Rust · WASM · Native",
    heading: "TeX-quality math from one Rust layout core",
    descMain:
      "RaTeX parses LaTeX math, applies TeX-style rules, and emits a flat display list for CoreGraphics, Skia, Canvas 2D, or your own vector backend—identical output from native FFI and WebAssembly.",
    alignmentLabel: "Alignment:",
    alignmentBody:
      "RaTeX is built to match KaTeX where it matters: CI runs large golden suites with pixel diffs against reference images, and on that corpus output is broadly comparable to KaTeX. The",
    alignmentLink: "support table",
    alignmentSuffix: "walks the full golden list side-by-side with KaTeX.",
    whereFitsLabel: "Where it fits:",
    whereFitsPrefix: "for math inside a normal web page,",
    whereFitsMid:
      "in the DOM remains a great default. RaTeX is aimed at native apps, servers, and embeds without a WebView—same engine from mobile to WASM.",
    tryIt: "Try it",
    packagesEyebrow: "Packages",
    packagesHeading: "Integrate anywhere",
    packagesDescPrefix:
      "Ready-to-use SDKs and WASM builds ship from the same Rust core: install from npm, Maven, pub.dev, or SPM—step-by-step in",
    packagesGetStarted: "Get started",
    packagesDescSuffix: ". Server-side PNG and CLI are covered there too.",
    whenToUseHeading: "When to reach for RaTeX",
    whenToUseNative: "Native or server",
    whenToUseNativeDesc:
      "Ship the same layout on iOS, Android, Flutter, or Rust services (PNG/SVG-style rasterization) without bundling a browser.",
    whenToUseWasm: "WASM in your host",
    whenToUseWasmDescPrefix:
      "Run the core in WebAssembly and draw with Canvas; compare output with KaTeX in the",
    whenToUseWasmLink: "live demo",
    whenToUseWasmDescSuffix: ".",
    whenToUseChem: "Chemistry & units",
    whenToUseChemDescSuffix:
      "on the mhchem-style path next to ordinary math (see galleries below).",
    rustCoreHeading: "Rust core",
    rustCoreDesc:
      "One layout engine, no GC in the hot path: predictable timing for mobile UIs, servers, and CI raster tests.",
    shipEverywhereHeading: "Ship everywhere",
    shipEverywhereDesc:
      "C ABI for Swift, Kotlin, Dart, … WASM for the web; tiny-skia or your own rasterizer—identical display lists.",
    mhchemHeading: "mhchem-style chemistry",
    mhchemDescPrefix: "Built-in",
    mhchemDescMid: "and",
    mhchemDescSuffix:
      "on the mhchem-compatible path—reaction arrows and physical units in the same pipeline as ordinary math.",
    galleriesEyebrow: "Try it in the browser",
    galleriesHeading: "Golden-suite galleries",
    galleriesDescPrefix:
      "Browse the same LaTeX lines CI uses, rendered with RaTeX WASM on the page:",
    galleriesDemoPrefix: "For side-by-side comparison with KaTeX, open the",
    galleriesDemoLink: "interactive demo",
    galleriesDemoMid: "; the full golden suite lives in the",
    galleriesSupportLink: "support table",
    galleriesDemoSuffix: "on the Demo page.",
    comparisonHeading: "Why not a WebView stack?",
    comparisonDesc:
      "In the browser, KaTeX and MathJax typically run as JavaScript against the DOM. For app shells that embed math via WebView, that still means shipping a browser stack. RaTeX keeps layout and rasterization in Rust for hosts that want to avoid that path.",
    comparisonRuntime: "Runtime",
    comparisonMobile: "Mobile",
    comparisonOffline: "Offline",
    comparisonJsBundle: "JS bundle (typical)",
    comparisonMemory: "Memory model",
    nativeSdkHeading: "RaTeX vs native math SDKs",
    nativeSdkDesc:
      "Without a WebView, teams often reach for Swift, Objective-C, or Flutter libraries. Below is a high-level comparison with widely used open-source renderers—swiftMath (Swift), flutter_math_fork / flutter_math (Dart / Flutter), and iosMath (iOS)—on chemistry macros, portability, and engine shape. Third-party SDKs evolve independently; compare versions when you integrate.",
    nativeSdkFootnote:
      "*Performance depends on workload. Swift uses ARC; Dart uses a tracing GC—both differ from RaTeX's Rust core for the same \"no browser\" embedding story.",
    capabilityLabel: "Capability",
    sameEngineFfi: "Same engine: native FFI + WASM (web)",
    sameEngineRust: "Mobile + desktop from one Rust core",
    rustLayoutCore: "TeX layout core in Rust (predictable hot path)",
    ctaHeading: "Ship scientific UI without embedding a browser engine",
    ctaLiveDemo: "Live demo",
    ctaGithubReadme: "GitHub README",
  },
  getStarted: {
    eyebrow: "Integration",
    heading: "Get started by platform",
    intro:
      "Every target consumes the same display list from the Rust pipeline. Pick your stack below, then open the full guide on GitHub for versioning, fonts, and native build scripts.",
    tryBrowserFirst: "Prefer trying formulas in the browser first?",
    liveDemoLink: "Live demo",
    mathGalleryLink: "Math gallery",
    jumpTo: "Jump to",
    fullDoc: "Full documentation",
    architectureHeading: "Architecture overview",
    architectureDescPrefix:
      "All paths share: lexer → parser → layout → display list. Native UIs and WASM map that list to CoreGraphics, Android Canvas, Flutter",
    architectureDescSuffix:
      ", Skia, or Canvas 2D; the server crate rasterizes with tiny-skia.",
    architectureLink: "README — Architecture",
    platforms: [
      {
        title: "Web (WASM)",
        blurb:
          "Rust compiled to WebAssembly; Canvas 2D draws the display list. Use the published npm package and the optional ratex-formula web component.",
        steps: [
          "Install: `npm install ratex-wasm`",
          "Load KaTeX fonts CSS from the package and register the custom element or call the programmatic API.",
        ],
      },
      {
        title: "iOS (Swift)",
        blurb:
          "Swift / SwiftUI views over the C ABI; CoreGraphics renders the display list. SPM from the GitHub repo.",
        steps: [
          "In Xcode: File → Add Package Dependencies → `https://github.com/erweixin/RaTeX`, select the RaTeX product.",
          "Use `RaTeXFormula` / `RaTeXView`; fonts load from the package on first render.",
        ],
      },
      {
        title: "Android (Kotlin)",
        blurb:
          "AAR with JNI into the same native library; Canvas draws glyphs and rules. Published to Maven coordinates.",
        steps: [
          "Add `implementation(\"io.github.erweixin:ratex-android:…\")` (see README for current version).",
          "Place `RaTeXView` in XML or Compose and set `latex` / `fontSize` in code.",
        ],
      },
      {
        title: "Flutter (Dart FFI)",
        blurb:
          "Dart FFI to `libratex_ffi`; `CustomPainter` renders the display list. Prebuilt iOS XCFramework + Android `.so` on pub.dev.",
        steps: [
          "Add `ratex_flutter` to `pubspec.yaml` and run `flutter pub get`.",
          "Register KaTeX fonts in your app's `pubspec.yaml` under `flutter: fonts:` using the `packages/ratex_flutter/` asset prefix — without this step glyphs silently fall back to system fonts. See the full doc for the complete snippet.",
          "Use `RaTeXWidget(latex: r'…', fontSize: 28)`.",
        ],
      },
      {
        title: "React Native",
        blurb:
          "Native views on iOS and Android; JS bundles the UI while Rust handles parse/layout in `.a` / `.so`.",
        steps: [
          "Install: `npm install ratex-react-native` then `cd ios && pod install`.",
          "Use `RaTeXView` / `InlineTeX`; fonts ship with the package.",
        ],
      },
      {
        title: "Server / CLI",
        blurb:
          "Rasterize to PNG with tiny-skia (`ratex-render`) or export to self-contained SVG with `ratex-svg`—CI snapshots, backends, or headless servers—no browser needed.",
        steps: [
          "PNG: pipe LaTeX to stdin — `cargo run --release -p ratex-render`.",
          "SVG: add `--features cli` — `cargo run --release -p ratex-svg --features cli`. Outputs `<path>`-based SVG with no web-font dependency.",
        ],
      },
    ],
  },
  demo: {
    eyebrow: "Try it",
    heading: "Demos & benchmarks",
    intro:
      "Same RaTeX WASM as production builds; KaTeX 0.16.45 is the reference renderer on these pages.",
    suggestedOrderLabel: "Suggested order",
    suggestedOrderDescPrefix: "Start with",
    suggestedOrderLiveLink: "live comparison",
    suggestedOrderDescMid: "for one formula, open the",
    suggestedOrderTableLink: "support table",
    suggestedOrderDescSuffix:
      "to scan all 1058 lines, then use galleries when you want categorized scrolling.",
    howItLoadsLabel: "How it loads:",
    howItLoadsDesc:
      "KaTeX 0.16.45 CSS/JS from jsDelivr. RaTeX uses this site\u2019s platforms/web/ (WASM + fonts). On GitHub Pages that ships with the deployment; locally, build WASM and use the dev server\u2014see",
    howItLoadsGetStartedLink: "Get started \u2192 Web",
    liveComparisonTitle: "Live comparison",
    liveComparisonSubtitle: "RaTeX WASM vs KaTeX 0.16.45",
    liveComparisonBody:
      "Edit one LaTeX line and compare RaTeX canvas output with KaTeX side by side\u2014status, errors, and the same WASM bundle as the galleries.",
    liveComparisonCta: "Open interactive demo",
    supportTableTitle: "Support table",
    supportTableSubtitle: "1058 golden formulas",
    supportTableBody:
      "Opens the full-page benchmark: every golden-suite line vs KaTeX 0.16.45, with batch IoU scores and a live RaTeX column in your browser\u2014best for coverage and regression triage.",
    supportTableCta: "Open full support table",
    galleriesEyebrow: "Same WASM \u00b7 different UI",
    galleriesHeading: "Golden-suite galleries",
    galleriesDesc:
      "Same destinations as the site header\u2014long, lazy-loaded lists with source above and canvas below for spot-checking many formulas.",
    galleriesOpen: "Open",
    footerText: "Integrate RaTeX in apps:",
    footerLink: "Get started by platform",
    galleryLabels: {
      math: "Math",
      chemistry: "Chemistry",
      physics: "Physics",
    },
    galleryHints: {
      math: "KaTeX-style sections \u00b7 900+ lines",
      chemistry: "mhchem \\ce",
      physics: "\\pu and curated",
    },
  },
  demoLive: {
    eyebrow: "Try it",
    heading: "Live comparison",
    desc: "Type LaTeX below and compare KaTeX (reference) with RaTeX (Rust \u2192 WASM \u2192 canvas). Same bundle as the galleries.",
    inputPlaceholder: "Enter LaTeX\u2026",
    renderBtn: "Render",
    statusLoading: "Loading\u2026",
    waitingForInput: "Waiting for input\u2026",
    quickTryLabel: "Quick try",
    activeDevText: "RaTeX is under active development. Found something wrong?",
    openIssueLink: "Open an issue",
    examplesLabel: "Examples \u2014 click a card to load",
  },
  supportTable: {
    eyebrow: "Benchmarks",
    heading: "Formula support table",
    desc: "RaTeX (Rust + WASM) vs KaTeX 0.16.45 across 1058 golden-suite lines (includes mhchem \\ce / \\pu). Offline cells use pre-computed ink IoU vs KaTeX reference PNGs; the RaTeX column is computed live in your browser from the loaded WASM.",
    dataSourceLabel: "Data source",
    dataSourceDescPrefix:
      "Batch offline scores and aggregate counts are regenerated in CI runs and may lag the latest",
    dataSourceDescMid:
      "by a few commits. The per-row RaTeX value always reflects the WASM you just loaded. For a single-formula sanity check, use",
    dataSourceLiveLink: "live comparison",
    scoreGreat: "score \u2265 0.9",
    scoreHigh8: "0.8\u20130.9",
    scoreMidHi: "0.5\u20130.8",
    scoreMidLo: "0.3\u20130.5",
    scoreLow: "< 0.3 or error",
    scoreAvg: "avg score",
    filterAll: "All",
    filterGreat: "\u2265 0.9",
    filterHigh8: "0.8\u20130.9",
    filterMidHi: "0.5\u20130.8",
    filterMidLo: "0.3\u20130.5",
    filterLow: "< 0.3 / err",
    searchPlaceholder: "Search LaTeX\u2026",
    initializing: "Initializing\u2026",
    colNum: "#",
    colLatex: "LaTeX source",
    colKatex: "KaTeX (reference)",
    colRatex: "RaTeX (WASM)",
    colScore: "Score",
    scoresDesc:
      "Scores come from offline golden comparison (RaTeX server PNG vs KaTeX reference). The RaTeX column is rendered on demand with WASM on this page; fonts match the gallery setup. Again: golden pipeline outputs may lag the repo\u2014see the note above.",
    offlineIouNote: "Offline IoU vs KaTeX PNGs",
    referencesLabel: "References",
    liveCompLink: "Live comparison",
    inkIou: "Ink-coverage IoU vs KaTeX PNGs",
  },
  mathGallery: {
    eyebrow: "Gallery \u00b7 Golden suite",
    title: "Math",
    desc1prefix: "One entry per line from",
    desc1suffix: "\u2014 the same inputs used in CI raster tests.",
    desc2prefix: "Sections follow the topic order of",
    desc2link: "KaTeX Supported Functions",
    desc2suffix:
      "(accents, delimiters, environments, \u2026). Each card shows the source above and the output below; cells use a responsive grid and render as you scroll.",
    ariaLabel: "Math formula grid",
  },
  chemGallery: {
    eyebrow: "Gallery \u00b7 mhchem",
    title: "Chemistry",
    desc1prefix: "Lines from",
    desc1mid: "that use",
    desc1suffix:
      ", including mixed math + chemistry. These mirror the paths covered by golden raster tests.",
    desc2prefix: "Rows that also contain",
    desc2mid: "may appear on the",
    desc2link: "Physics",
    desc2suffix: "gallery as well.",
    ariaLabel: "Chemistry formula grid",
  },
  physicsGallery: {
    eyebrow: "Gallery \u00b7 Units & equations",
    title: "Physics",
    desc1prefix: "All",
    desc1mid: "lines from",
    desc1suffix:
      ", plus a short curated set of classic formulas (e.g. Schr\u00f6dinger, Maxwell) for visual smoke tests.",
    desc2prefix: "For chemistry-specific",
    desc2mid: "coverage, see the",
    desc2link: "Chemistry",
    desc2suffix: "gallery.",
    ariaLabel: "Physics formula grid",
  },
};
