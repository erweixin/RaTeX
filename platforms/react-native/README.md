# ratex-react-native

Native LaTeX math rendering for React Native — no WebView, no JavaScript math engine. Formulas are parsed and laid out in Rust (compiled to a native library) and drawn directly onto a native Canvas using KaTeX fonts.

> Chinese documentation: [README.zh.md](./README.zh.md)

## Features

- Renders LaTeX math natively on iOS, Android, and macOS (via [React Native macOS](https://github.com/microsoft/react-native-macos))
- Built for the **New Architecture** (Fabric / JSI / TurboModules) — the only architecture supported by React Native ≥ 0.84
- Measures rendered content size for scroll and dynamic layout
- Error callback for parse failures
- Bundles all required KaTeX fonts — no extra setup
- Baseline alignment in flex rows and inside `<Text>` via `alignSelf: 'baseline'`
- Sync formula metrics (`getTexMetrics`) for custom text engines
- `InlineTeX` component for mixed text + `$...$` formula strings

## Requirements

| Dependency | Version |
|-----------|---------|
| React Native | ≥ 0.84 |
| React | ≥ 19.2 |
| iOS | ≥ 14.0 |
| macOS | ≥ 13.0 (when using React Native macOS) |
| Android | minSdk 21 (Android 5.0+) |

## Installation

```sh
npm install ratex-react-native
```

### iOS — pod install

```sh
cd ios && pod install
```

### macOS (React Native macOS)

Use the same `ratex-react-native` pod on macOS: from your app’s `macos/` folder run `pod install`, then `npx react-native run-macos`. The `RaTeX.xcframework` vendored by this pod must include a **macOS** slice (see `./scripts/build-apple-xcframework.sh` in the RaTeX repo).

### Android

No additional steps required. The native `.so` libraries are bundled automatically.

## Usage

### Block formula

```tsx
import { RaTeXView } from 'ratex-react-native';

function MathFormula() {
  return (
    <RaTeXView
      latex="\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"
      fontSize={24}
      color="#1E88E5"
      onError={(e) => console.warn('LaTeX error:', e.nativeEvent.error)}
    />
  );
}
```

### Inline formula (mixed text + LaTeX)

```tsx
import { InlineTeX } from 'ratex-react-native';

function Paragraph() {
  return (
    <InlineTeX
      content="The energy–mass relation $E = mc^2$ is a consequence of special relativity."
      fontSize={16}
      textStyle={{ color: '#333' }}
    />
  );
}
```

Use `$...$` delimiters anywhere inside the `content` string. Multiple formulas in one string are supported.

### Shared default color

```tsx
import { RaTeXProvider, InlineTeX, RaTeXView } from 'ratex-react-native';

function Screen() {
  return (
    <RaTeXProvider color="#1E88E5">
      <RaTeXView latex="x + y" />
      <InlineTeX content="Inline math: $E = mc^2$" />
    </RaTeXProvider>
  );
}
```

## API

### `<RaTeXView />`

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `latex` | `string` | — | LaTeX math-mode string to render (required) |
| `fontSize` | `number` | `24` | Font size in **dp** (density-independent pixels). The rendered formula scales proportionally. |
| `displayMode` | `boolean` | `true` | `true` = display/block style (`$$...$$`); `false` = inline/text style (`$...$`). |
| `color` | `ColorValue` | — | Default formula color. Explicit LaTeX colors still take precedence. |
| `style` | `StyleProp<ViewStyle>` | — | Standard React Native style. Width and height are automatically set from measured content unless overridden. |
| `onError` | `(e: { nativeEvent: { error: string } }) => void` | — | Called when the LaTeX string fails to parse. |
| `onContentSizeChange` | `(e: { nativeEvent: { width: number; height: number } }) => void` | — | Called after layout with the formula's **intrinsic (unscaled) content size** in dp. Useful for scroll views or dynamic containers. |

### Content size auto-sizing

`RaTeXView` automatically applies the measured `width` and `height` from `onContentSizeChange` to its own style. This means you can use `wrap_content`-style layout without specifying explicit dimensions:

```tsx
<ScrollView horizontal>
  <RaTeXView latex="\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}" fontSize={28} />
</ScrollView>
```

#### Explicit width/height behavior

If you explicitly provide `style.width` and/or `style.height`, `RaTeXView` will **not** override those values with measurements. Instead, the native view will scale the formula down (never up) to fit the assigned layout size and clip to bounds when necessary.

### Baseline alignment

`RaTeXView` can sit on the text baseline like a glyph — in a flex row and inside `<Text>`:

```tsx
<View style={{ flexDirection: 'row', alignItems: 'baseline' }}>
  <Text>f(x) =</Text>
  <RaTeXView latex={'\\frac{a}{b}'} fontSize={16} displayMode={false} />
</View>

<Text>
  compare y with{' '}
  <RaTeXView latex="y" fontSize={16} displayMode={false}
             style={{ alignSelf: 'baseline' }} />{' '}
  mid-sentence
</Text>
```

### `<InlineTeX />`

Renders a mixed string of plain text and `$...$` LaTeX formulas as a single native text flow. Formulas are embedded with `NSTextAttachment` on iOS/macOS and `ReplacementSpan` on Android, so line wrapping, word breaking, and baseline alignment are handled by the platform text layout engine.

**Rendering pipeline:**

1. `content` is parsed into text and formula segments. Escaped dollars (`\$`) stay as literal text, and unmatched or empty `$` delimiters fall back to plain text.
2. Formula segments are rendered inline with native text attachments/spans and report measured content height for dynamic layout.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `content` | `string` | — | Text string with `$...$` markers for inline LaTeX (required). |
| `fontSize` | `number` | `16` | Font size passed to each formula renderer (dp). |
| `color` | `ColorValue` | — | Default color passed to each inline formula. Explicit LaTeX colors still take precedence. |
| `textStyle` | `StyleProp<TextStyle>` | — | Plain-text style source. Supported fields: `color`, `fontSize`, `fontFamily`, `fontStyle: 'italic'`, and `textDecorationLine` with `underline` / `line-through`. |
| `style` | `StyleProp<ViewStyle>` | — | Standard React Native style for the native inline container. Height is measured automatically unless explicitly provided. |

> `InlineTeX` automatically passes `displayMode={false}` to every formula it renders — `$...$` is always inline style.

### `<RaTeXProvider />`

Provides a default formula color to descendant `RaTeXView` and `InlineTeX` components. Use a component-level `color` prop to override the inherited value.

### `getTexMetrics()`

Synchronous formula ink metrics — TeX's box *depth*: what KaTeX emits as `vertical-align: -depth`, MathML Core's *ink line-descent*. For custom text hosts (TextKit / Spannable engines, markdown renderers) that need the baseline offset as a number. Served from the same parse cache as measure/render — an on-screen formula never re-parses.

```tsx
import { getTexMetrics } from 'ratex-react-native';

// Natural (unscaled) metrics for any formula — no view needed:
const m = getTexMetrics('\\frac{a}{b}', 16, false);
// { width, height, depth } | null — dp; the baseline sits at height - depth

// Drawn metrics on a mounted view (fit scale + centering applied):
const d = ref.current?.getTexMetrics(); // ref: RaTeXViewRef
// { depth, scale, width, height } | null — apply depth directly
```

Both are safe to call from `useLayoutEffect`. 

## Architecture Support

Only the **New Architecture** (Fabric / Codegen / TurboModules) is supported, matching the [officially supported React Native versions](https://reactnative.dev/versions) (≥ 0.84), where it is enabled by default. The legacy Bridge/Paper architecture is not supported.

## Font size note

`fontSize` is interpreted as **dp (density-independent pixels)**, not CSS `pt` or raw pixels. On a 3× density screen, a `fontSize={24}` formula renders at 72 physical pixels tall. This matches React Native's standard layout unit.

## License

MIT
