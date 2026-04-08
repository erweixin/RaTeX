import type {
  Double,
  Float,
  BubblingEventHandler,
  DirectEventHandler,
} from 'react-native/Libraries/Types/CodegenTypes';
import codegenNativeComponent from 'react-native/Libraries/Utilities/codegenNativeComponent';
import type {HostComponent, ViewProps} from 'react-native';

type OnErrorEvent = {error: string};
type OnContentSizeChangeEvent = {width: Double; height: Double};

export interface NativeProps extends ViewProps {
  latex: string;
  fontSize?: Float;
  /** true (default) = display/block style; false = inline/text style. */
  displayMode?: boolean;
  onError?: BubblingEventHandler<OnErrorEvent>;
  onContentSizeChange?: DirectEventHandler<OnContentSizeChangeEvent>;
}

export default codegenNativeComponent<NativeProps>(
  'RaTeXView',
) as HostComponent<NativeProps>;
