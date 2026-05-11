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
  /** Style applied to plain-text segments (fontSize, color are extracted). */
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

  const [contentSize, setContentSize] = useState<{
    width: number;
    height: number;
  } | null>(null);

  useEffect(() => {
    setContentSize(null);
  }, [content, fontSize, resolvedColor, textColor, textFontSize]);

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

  const estimatedHeight = useMemo(() => {
    const lineHeight = Math.ceil(textFontSize * 1.4);
    return lineHeight;
  }, [textFontSize]);

  const heightValue = contentSize
    ? contentSize.height
    : estimatedHeight;

  const resolvedStyle = hasHeight
    ? style
    : [style, {minHeight: heightValue}];

  return (
    <RaTeXInlineViewNativeComponent
      content={content}
      fontSize={fontSize}
      color={resolvedColor}
      textColor={textColor}
      textFontSize={textFontSize}
      style={resolvedStyle}
      onContentSizeChange={handleContentSizeChange}
    />
  );
}
