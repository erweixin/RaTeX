# RaTeX React Native Demo

This folder contains the **RaTeX** React Native demo app for iOS, Android, and **macOS**.

> **Version constraint**: macOS support is provided by [react-native-macos](https://github.com/microsoft/react-native-macos), which must stay on the same minor version as `react-native`.
> This demo is pinned to **`react-native@0.81.6`** + **`react-native-macos@0.81.7`** because that is the newest minor react-native-macos has released.
>
> **Note**: `ratex-react-native` officially supports **React Native ≥ 0.84** (New Architecture only, matching the [supported RN versions](https://reactnative.dev/versions)). This demo runs on 0.81 with the New Architecture enabled and installs with `legacy-peer-deps` (see `.npmrc`) to bypass the peer range; it works because 0.81 already ships full Fabric support, but it is a best-effort setup until react-native-macos catches up.

> **Upgrade path**: when `react-native-macos` releases a new minor (for example `0.84.x`), upgrade `react-native` and `react-native-macos` together, then align `@react-native/*` toolchain packages, Gradle wrapper, and Node engines in one pass, and remove `legacy-peer-deps` from `.npmrc` once ≥ 0.84.

## Prerequisites

- Node **>= 20.19.4** (matches `react-native@0.81.6` engines)
- Xcode (for iOS / macOS)
- Android SDK (for Android)
- Before building Apple platforms for the first time, generate `RaTeX.xcframework` with both iOS and macOS slices:

```bash
# Run from repo root: build platforms/ios/RaTeX.xcframework (iOS + macOS)
./scripts/build-apple-xcframework.sh

# Copy into the local ratex-react-native package (same as release-react-native CI)
rm -rf platforms/react-native/ios/RaTeX.xcframework
cp -R platforms/ios/RaTeX.xcframework platforms/react-native/ios/
```

If the macOS slice is missing, `ratex-react-native` may fail with errors like `Unable to find module dependency: 'RaTeXFFI'`.

## Install dependencies

```bash
cd demo/react-native
npm install
```

### iOS

```bash
cd ios
bundle install          # first time only: install CocoaPods from Gemfile
bundle exec pod install
cd ..
npm run ios
```

### Android

```bash
npm run android
```

### macOS

```bash
# Install CocoaPods dependencies (first time or after native dependency changes)
npm run pods:macos

# Terminal 1: Metro
npm start

# Terminal 2: Build and run macOS app
npm run macos
```

You can also open **`macos/RaTeXDemo.xcworkspace`** in Xcode and run scheme **`RaTeXDemo-macOS`**.

## Tests

```bash
npm test
```

## macOS Rendering Regression Checklist

Run these checks before release when macOS rendering code changes:

1. Inline baseline alignment: verify text mixed with `$...$` formulas keeps a stable baseline in multiple font sizes.
2. Appearance switching: toggle light/dark mode and verify formula/text colors re-render correctly.
3. Window resizing: live-resize the window and confirm formulas remain crisp (no persistent blur after resize).
4. Dynamic content: update formula/text props repeatedly and verify intrinsic size events continue to emit.

## Notes

- JS entry: `index.js`; root component: `App.tsx`.
- `ratex-react-native` is linked locally via `file:../../platforms/react-native`.
- `metro.config.js` uses `blockList` to avoid resolving a second copy of `react-native` from `platforms/react-native/node_modules`.
- To regenerate the `macos/` app template, use `npx react-native-macos-init@2.1.3 --version 0.81.7` (be careful with `--overwrite` since generated files are committed in this repo).

## Fabric Registration (demo only)

- `macos/RaTeXDemo-macOS/RaTeXComponentRegistration.m` injects `RaTeXView` / `RaTeXInlineView` via a custom `RCTAppDependencyProvider` subclass (no swizzling).
- If future RN APIs remove or change `thirdPartyFabricComponents`, migrate to the official third-party Fabric registration flow for that RN version.
- For production integrations, always prefer the official registration mechanism provided by your RN version.
