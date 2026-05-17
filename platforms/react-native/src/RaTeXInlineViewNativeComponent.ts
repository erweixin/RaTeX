import type {
  Double,
  Float,
  DirectEventHandler,
} from 'react-native/Libraries/Types/CodegenTypes';
import codegenNativeComponent from 'react-native/Libraries/Utilities/codegenNativeComponent';
import type {ColorValue, HostComponent, ViewProps} from 'react-native';

type OnContentSizeChangeEvent = {width: Double; height: Double};

export interface NativeProps extends ViewProps {
  /** Text with $...$ markers for inline LaTeX formulas. */
  content: string;
  /** Font size for formula rendering (points). Defaults to 16. */
  fontSize?: Float;
  /** Formula color. */
  color?: ColorValue;
  /** Plain-text color. */
  textColor?: ColorValue;
  /** Plain-text font size (points). Defaults to fontSize. */
  textFontSize?: Float;
  /** Plain-text font family. */
  textFontFamily?: string;
  /** Plain-text italic style. */
  textItalic?: boolean;
  /** Plain-text underline decoration. */
  textUnderline?: boolean;
  /** Plain-text line-through decoration. */
  textLineThrough?: boolean;
  onContentSizeChange?: DirectEventHandler<OnContentSizeChangeEvent>;
}

export default codegenNativeComponent<NativeProps>(
  'RaTeXInlineView',
) as HostComponent<NativeProps>;
