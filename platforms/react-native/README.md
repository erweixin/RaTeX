# ratex-react-native

Native LaTeX math rendering for React Native — no WebView, no JavaScript math engine. Formulas are parsed and laid out in Rust (compiled to a native library) and drawn directly onto a native Canvas using KaTeX fonts.

> Chinese documentation: [README.zh.md](./README.zh.md)

## Features

- Renders LaTeX math natively on iOS and Android
- Supports both the **New Architecture** (Fabric / JSI) and the **Old Architecture** (Bridge)
- Measures rendered content size for scroll and dynamic layout
- Error callback for parse failures
- Bundles all required KaTeX fonts — no extra setup
- `InlineTeX` component for mixed text + `$...$` formula strings

## Requirements

| Dependency | Version |
|-----------|---------|
| React Native | ≥ 0.73 |
| React | ≥ 18 |
| iOS | ≥ 14.0 |
| Android | minSdk 21 (Android 5.0+) |

## Installation

```sh
npm install ratex-react-native
```

### iOS — pod install

```sh
cd ios && pod install
```

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

## API

### `<RaTeXView />`

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `latex` | `string` | — | LaTeX math-mode string to render (required) |
| `fontSize` | `number` | `24` | Font size in **dp** (density-independent pixels). The rendered formula scales proportionally. |
| `displayMode` | `boolean` | `true` | `true` = display/block style (`$$...$$`); `false` = inline/text style (`$...$`). |
| `style` | `StyleProp<ViewStyle>` | — | Standard React Native style. Width and height are automatically set from measured content unless overridden. |
| `onError` | `(e: { nativeEvent: { error: string } }) => void` | — | Called when the LaTeX string fails to parse. |
| `onContentSizeChange` | `(e: { nativeEvent: { width: number; height: number } }) => void` | — | Called after layout with the formula's rendered dimensions in dp. Useful for scroll views or dynamic containers. |

### Content size auto-sizing

`RaTeXView` automatically applies the measured `width` and `height` from `onContentSizeChange` to its own style. This means you can use `wrap_content`-style layout without specifying explicit dimensions:

```tsx
<ScrollView horizontal>
  <RaTeXView latex="\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}" fontSize={28} />
</ScrollView>
```

### `<InlineTeX />`

Renders a mixed string of plain text and `$...$` LaTeX formulas as a single inline flow. Text and formula segments are laid out in a flex row with `alignItems: 'center'`, so the formula centerline aligns with the surrounding text automatically — no manual offset required.

**Rendering pipeline:**

1. Each formula is first rendered off-screen (absolutely positioned, `opacity: 0`) to measure its intrinsic width and height via `onContentSizeChange`.
2. Once all formulas are measured, the visible flex row is rendered with each formula at its exact measured size.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `content` | `string` | — | Text string with `$...$` markers for inline LaTeX (required). |
| `fontSize` | `number` | `16` | Font size passed to each formula renderer (dp). |
| `textStyle` | `StyleProp<TextStyle>` | — | Style applied to plain-text segments. |

> `InlineTeX` automatically passes `displayMode={false}` to every formula it renders — `$...$` is always inline style.

## Architecture Support

Supports both **New Architecture** (Fabric / Codegen) and **Old Architecture** (Bridge) — no configuration needed. React Native ≥ 0.73 with `newArchEnabled=true` uses Fabric automatically; older projects fall back to the Bridge manager.

## Font size note

`fontSize` is interpreted as **dp (density-independent pixels)**, not CSS `pt` or raw pixels. On a 3× density screen, a `fontSize={24}` formula renders at 72 physical pixels tall. This matches React Native's standard layout unit.

## License

MIT
