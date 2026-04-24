# RaTeX Flutter Desktop Demo

Showcase of native LaTeX math rendering on **macOS**, **Windows**, and **Linux** using the `ratex_flutter` plugin.

## Features

- Sidebar with 16+ preset formulas across 5 categories (Classics, Calculus, Linear Algebra, Physics, Chemistry)
- Adjustable font size (8–96px)
- Display mode toggle (block display vs inline text)
- Custom LaTeX formula input
- Auto dark/light theme
- Adaptive layout (sidebar collapses on narrow windows)
- LaTeX source preview panel

## Prerequisites

- Flutter 3.10+
- Dart 3.0+

For local development (using the plugin from the repo), you also need:
- Rust 1.75+

## Quick Start (pub.dev)

```bash
cd demo/flutter-desktop
flutter pub get
flutter run -d macos    # or -d windows, -d linux
```

## Local Development (from repo)

Build the desktop native library first, then run:

```bash
# 1. Build native library for your platform
cd /path/to/RaTeX
bash platforms/flutter/build-desktop.sh

# 2. Run the demo
cd demo/flutter-desktop
flutter pub get
flutter run -d macos
```

## Project Structure

```
demo/flutter-desktop/
├── lib/main.dart          # Demo app with sidebar + formula rendering
├── pubspec.yaml           # Dependencies + KaTeX font declarations
├── macos/                 # macOS platform files (auto-generated)
├── windows/               # Windows platform files (auto-generated)
├── linux/                 # Linux platform files (auto-generated)
├── android/               # Android platform files (auto-generated)
├── ios/                   # iOS platform files (auto-generated)
└── README.md
```
