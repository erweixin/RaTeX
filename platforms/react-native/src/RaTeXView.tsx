import React, {useState, useCallback} from 'react';
import type {StyleProp, ViewStyle} from 'react-native';
import RaTeXViewNativeComponent from './RaTeXViewNativeComponent';

export interface RaTeXViewProps {
  latex: string;
  fontSize?: number;
  /** true (default) = display/block style ($$...$$); false = inline/text style ($...$). */
  displayMode?: boolean;
  style?: StyleProp<ViewStyle>;
  onError?: (e: {nativeEvent: {error: string}}) => void;
  /** Called when content size is measured (e.g. for scroll layout). */
  onContentSizeChange?: (e: {
    nativeEvent: {width: number; height: number};
  }) => void;
}

export function RaTeXView({
  latex,
  fontSize = 24,
  displayMode = true,
  style,
  onError,
  onContentSizeChange,
}: RaTeXViewProps): React.JSX.Element {
  const [contentSize, setContentSize] = useState<{
    width: number;
    height: number;
  } | null>(null);

  const handleContentSizeChange = useCallback(
    (e: {nativeEvent: {width: number; height: number}}) => {
      setContentSize({
        width: e.nativeEvent.width,
        height: e.nativeEvent.height,
      });
      onContentSizeChange?.(e);
    },
    [onContentSizeChange],
  );

  const resolvedStyle = contentSize
    ? [style, {width: contentSize.width, height: contentSize.height}]
    : style;

  return (
    <RaTeXViewNativeComponent
      latex={latex}
      fontSize={fontSize}
      displayMode={displayMode}
      style={resolvedStyle}
      onError={onError}
      onContentSizeChange={handleContentSizeChange}
    />
  );
}
