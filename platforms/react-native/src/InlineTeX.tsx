import React, {useCallback, useContext, useEffect, useMemo, useState} from 'react';
import {StyleSheet} from 'react-native';
import type {ColorValue, StyleProp, TextStyle, ViewStyle} from 'react-native';
import RaTeXInlineViewNativeComponent from './RaTeXInlineViewNativeComponent';
import {RaTeXColorContext} from './RaTeXView';

export interface InlineTeXProps {
  /** Text content with $...$ markers for inline LaTeX formulas. */
  content: string;
  /** Font size for formula rendering (points). Defaults to 16. */
  fontSize?: number;
  /** Default formula color. Explicit LaTeX colors still take precedence. */
  color?: ColorValue;
  /**
   * Style applied to plain-text segments.
   * Supported fields: color, fontSize, fontFamily, fontStyle(italic),
   * textDecorationLine(underline/line-through).
   */
  textStyle?: StyleProp<TextStyle>;
  /** Style applied to the container view. */
  style?: StyleProp<ViewStyle>;
}

/**
 * Renders a mixed string of plain text and `$...$` LaTeX formulas inline.
 *
 * Under the hood this delegates to a native view that uses
 * NSTextAttachment (iOS) / ReplacementSpan (Android) so that formulas
 * participate in the platform's native text layout — line-wrapping,
 * baseline alignment, and word-breaking all happen at character level.
 */
export function InlineTeX({
  content,
  fontSize = 16,
  color,
  textStyle,
  style,
}: InlineTeXProps): React.JSX.Element {
  const inheritedColor = useContext(RaTeXColorContext);
  const resolvedColor = color ?? inheritedColor;

  const flatTextStyle = StyleSheet.flatten(textStyle) as TextStyle | undefined;
  const textColor = flatTextStyle?.color as ColorValue | undefined;
  const textFontSize = flatTextStyle?.fontSize ?? fontSize;
  const textFontFamily =
    typeof flatTextStyle?.fontFamily === 'string' && flatTextStyle.fontFamily.trim().length > 0
      ? flatTextStyle.fontFamily
      : undefined;
  const textItalic = flatTextStyle?.fontStyle === 'italic';
  const rawDecorationLine = flatTextStyle?.textDecorationLine;
  const decorationLine = Array.isArray(rawDecorationLine)
    ? rawDecorationLine.join(' ')
    : rawDecorationLine;
  const textUnderline =
    typeof decorationLine === 'string' && decorationLine.includes('underline');
  const textLineThrough =
    typeof decorationLine === 'string' && decorationLine.includes('line-through');

  const [contentSize, setContentSize] = useState<{
    width: number;
    height: number;
  } | null>(null);

  useEffect(() => {
    setContentSize(null);
  }, [
    content,
    fontSize,
    resolvedColor,
    textColor,
    textFontSize,
    textFontFamily,
    textItalic,
    textUnderline,
    textLineThrough,
  ]);

  const handleContentSizeChange = useCallback(
    (e: {nativeEvent: {width: number; height: number}}) => {
      setContentSize({
        width: e.nativeEvent.width,
        height: e.nativeEvent.height,
      });
    },
    [],
  );

  const flatStyle = StyleSheet.flatten(style) as ViewStyle | undefined;
  const hasHeight = typeof flatStyle?.height === 'number';
  const hasExplicitWidth = flatStyle?.width != null;
  const hasExplicitAlignSelf = flatStyle?.alignSelf != null;

  const estimatedHeight = useMemo(() => {
    const lineHeight = Math.ceil(textFontSize * 1.4);
    return lineHeight;
  }, [textFontSize]);

  const hasRenderableContent = content.trim().length > 0;
  const heightValue = hasRenderableContent
    ? (contentSize ? contentSize.height : estimatedHeight)
    : 0;

  // The native view has no Yoga measure function, so without an explicit width
  // Yoga defaults to 0.  Always stretch to fill the parent (matching how RN's
  // <Text> behaves for wrapping content) unless the caller sets width or
  // alignSelf explicitly.
  const stretchFallback =
    !hasExplicitWidth && !hasExplicitAlignSelf
      ? {alignSelf: 'stretch' as const}
      : null;

  const resolvedStyle = hasHeight
    ? [style, stretchFallback]
    : [style, stretchFallback, {minHeight: heightValue}];

  return (
    <RaTeXInlineViewNativeComponent
      content={content}
      fontSize={fontSize}
      color={resolvedColor}
      textColor={textColor}
      textFontSize={textFontSize}
      textFontFamily={textFontFamily}
      textItalic={textItalic}
      textUnderline={textUnderline}
      textLineThrough={textLineThrough}
      style={resolvedStyle}
      onContentSizeChange={handleContentSizeChange}
    />
  );
}
