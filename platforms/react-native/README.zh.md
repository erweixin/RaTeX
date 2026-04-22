# ratex-react-native

React Native 原生 LaTeX 数学公式渲染库——无 WebView，无 JavaScript 数学引擎。公式在 Rust 中完成解析和排版（编译为原生库），直接使用 KaTeX 字体绘制到原生 Canvas 上。

> English documentation: [README.md](./README.md)

## 特性

- 在 iOS 和 Android 上原生渲染 LaTeX 数学公式
- 同时支持**新架构**（Fabric / JSI）和**旧架构**（Bridge）
- 测量渲染内容尺寸，便于滚动视图和动态布局
- 提供解析失败的错误回调
- 内置所有 KaTeX 字体，无需额外配置
- `InlineTeX` 组件支持文字与 `$...$` 公式混排

## 环境要求

| 依赖 | 版本 |
|-----|------|
| React Native | ≥ 0.73 |
| React | ≥ 18 |
| iOS | ≥ 14.0 |
| Android | minSdk 21（Android 5.0+）|

## 安装

```sh
npm install ratex-react-native
```

### iOS — pod install

```sh
cd ios && pod install
```

### Android

无需额外操作，原生 `.so` 库会自动打包。

## 使用方法

### 块级公式

```tsx
import { RaTeXView } from 'ratex-react-native';

function MathFormula() {
  return (
    <RaTeXView
      latex="\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"
      fontSize={24}
      color="#1E88E5"
      onError={(e) => console.warn('LaTeX 错误:', e.nativeEvent.error)}
    />
  );
}
```

### 内联公式（文字与 LaTeX 混排）

```tsx
import { InlineTeX } from 'ratex-react-native';

function Paragraph() {
  return (
    <InlineTeX
      content="质能等价关系 $E = mc^2$ 是狭义相对论的核心结论。"
      fontSize={16}
      textStyle={{ color: '#333' }}
    />
  );
}
```

在 `content` 字符串中用 `$...$` 标记公式，支持一段文字中包含多个公式。

### 共享默认颜色

```tsx
import { RaTeXProvider, InlineTeX, RaTeXView } from 'ratex-react-native';

function Screen() {
  return (
    <RaTeXProvider color="#1E88E5">
      <RaTeXView latex="x + y" />
      <InlineTeX content="内联公式：$E = mc^2$" />
    </RaTeXProvider>
  );
}
```

## API

### `<RaTeXView />`

| 属性 | 类型 | 默认值 | 说明 |
|-----|------|--------|------|
| `latex` | `string` | — | 要渲染的 LaTeX 数学字符串（必填） |
| `fontSize` | `number` | `24` | 字体大小，单位为 **dp**（密度无关像素）。公式整体等比缩放。 |
| `displayMode` | `boolean` | `true` | `true` = 独立块样式（`$$...$$`）；`false` = 行内样式（`$...$`）。 |
| `color` | `ColorValue` | — | 默认公式颜色。显式 LaTeX 颜色仍然优先。 |
| `style` | `StyleProp<ViewStyle>` | — | 标准 React Native 样式。宽高会自动从测量结果设置，也可手动覆盖。 |
| `onError` | `(e: { nativeEvent: { error: string } }) => void` | — | LaTeX 字符串解析失败时调用。 |
| `onContentSizeChange` | `(e: { nativeEvent: { width: number; height: number } }) => void` | — | 排版完成后回调，携带公式的**固有内容尺寸（未缩放）**（dp）。适用于滚动视图或动态容器。 |

### 内容尺寸自适应

`RaTeXView` 会自动将 `onContentSizeChange` 返回的 `width` 和 `height` 应用到自身 `style`，实现类似 `wrap_content` 的自适应布局，无需手动指定尺寸：

```tsx
<ScrollView horizontal>
  <RaTeXView latex="\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}" fontSize={28} />
</ScrollView>
```

#### 显式指定宽高时的行为

如果你在 `style` 中显式指定了 `width` 和/或 `height`，`RaTeXView` **不会**再用测量结果覆盖这些值；原生视图会在绘制阶段把公式**按比例缩小（不会放大）**以适配给定布局尺寸，并在必要时按边界裁剪。

### `<InlineTeX />`

将包含 `$...$` 标记的混合字符串渲染为内联流式布局。文字和公式片段以 flex 行排列，`alignItems: 'center'` 使公式中线与周围文字自动对齐，无需手动计算偏移量。

**渲染流程：**

1. 每个公式先在屏幕外（绝对定位、`opacity: 0`）通过 `onContentSizeChange` 测量其固有宽高。
2. 所有公式测量完毕后，按实测尺寸渲染可见的 flex 行。

| 属性 | 类型 | 默认值 | 说明 |
|-----|------|--------|------|
| `content` | `string` | — | 包含 `$...$` 标记的文字字符串（必填）。 |
| `fontSize` | `number` | `16` | 传给每个公式渲染器的字体大小（dp）。 |
| `color` | `ColorValue` | — | 传给每个行内公式的默认颜色。显式 LaTeX 颜色仍然优先。 |
| `textStyle` | `StyleProp<TextStyle>` | — | 应用于纯文字片段的样式。 |

> `InlineTeX` 会自动对所有公式传入 `displayMode={false}`——`$...$` 始终使用行内样式。

### `<RaTeXProvider />`

为后代 `RaTeXView` 和 `InlineTeX` 提供默认公式颜色。若组件自身传入 `color`，则会覆盖继承值。

## 架构支持

同时支持**新架构**（Fabric / Codegen）和**旧架构**（Bridge），无需任何配置。React Native ≥ 0.73 开启 `newArchEnabled=true` 后自动使用 Fabric；旧项目则回退到 Bridge 管理器。

## fontSize 说明

`fontSize` 单位为 **dp（密度无关像素）**，而非 CSS `pt` 或物理像素。在 3× 屏幕密度的设备上，`fontSize={24}` 的公式渲染高度为 72 物理像素，与 React Native 的标准布局单位一致。

## 许可证

MIT
