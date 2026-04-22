import type {
  Double,
  Float,
  BubblingEventHandler,
  DirectEventHandler,
} from 'react-native/Libraries/Types/CodegenTypes';
import codegenNativeComponent from 'react-native/Libraries/Utilities/codegenNativeComponent';
import type {ColorValue, HostComponent, ViewProps} from 'react-native';

type OnErrorEvent = {error: string};
type OnContentSizeChangeEvent = {width: Double; height: Double};

export interface NativeProps extends ViewProps {
  latex: string;
  fontSize?: Float;
  /** true (default) = display/block style; false = inline/text style. */
  displayMode?: boolean;
  color?: ColorValue;
  onError?: BubblingEventHandler<OnErrorEvent>;
  onContentSizeChange?: DirectEventHandler<OnContentSizeChangeEvent>;
}

export default codegenNativeComponent<NativeProps>(
  'RaTeXView',
) as HostComponent<NativeProps>;
