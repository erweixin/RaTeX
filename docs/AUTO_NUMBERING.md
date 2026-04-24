# Auto-numbering in RaTeX

RaTeX supports automatic equation numbering for AMS display environments.
This document covers the design, usage, and platform integration.

## Supported environments

| Environment | Auto-numbered | Notes |
|---|---|---|
| `equation` | Yes | Single row, single column |
| `equation*` | No | |
| `align` | Yes | Multi-row, alternating r/l columns |
| `align*` | No | |
| `gather` | Yes | Multi-row, single centered column |
| `gather*` | No | |
| `alignat` | Yes | Like align but with explicit column count |
| `alignat*` | No | |
| `aligned` | No | Inner environment (no independent number) |
| `split` | No | Inner environment |
| `gathered` | No | Inner environment |
| `alignedat` | No | Inner environment |
| `array` / `matrix` / `cases` / `CD` | No | Non-equation environments |

## Commands

| Command | Effect |
|---|---|
| `\tag{text}` | Override the auto-number with explicit text (wrapped in parentheses) |
| `\tag*{text}` | Override with raw text (no parentheses) |
| `\notag` / `\nonumber` | Suppress auto-numbering on this row |
| `\label{name}` | Associate the current equation number with `name` |
| `\ref{name}` | Render the equation number for `name` |
| `\eqref{name}` | Render `(number)` for `name` |

## Architecture

### Parser layer (`ratex-parser`)

- `ArrayConfig::auto_numbered: bool` controls whether an array environment generates numbers.
- Each environment handler sets `auto_numbered` based on the environment name (non-starred standalone = `true`).
- `extract_trailing_tag_from_last_cell` detects `\tag{...}` (→ `ArrayTag::Explicit`) and `\notag` (→ `ArrayTag::Suppressed`).
- `extract_label_from_last_cell` scans the last cell for `\label{name}` and stores it in `Array::labels`.
- `\label`, `\ref`, `\eqref`, `\notag`, `\nonumber` are registered as function handlers in `functions/label_ref.rs`.

### Layout layer (`ratex-layout`)

- `EquationState` struct holds the counter, collected labels, and external labels.
- `LayoutOptions::equation_state: Option<Rc<RefCell<EquationState>>>` propagates through the layout tree.
- In `layout_array`, `ArrayTag::Auto(true)` rows increment the counter and render the number as `(N)` in the tag column.
- If the row has a `\label`, the mapping is stored in `EquationState::labels`.
- `\ref`/`\eqref` nodes look up their label in `EquationState::external_labels`.

### Why `Rc<RefCell<>>`?

`LayoutOptions` is cloned extensively during layout (for style changes, color changes, etc.). `Rc` keeps all clones pointing to the same state. `RefCell` allows interior mutability so `layout()` can modify the counter and labels without requiring `&mut self` on `LayoutOptions`.

## Usage

### Basic auto-numbering

```rust
use std::cell::RefCell;
use std::rc::Rc;
use ratex_layout::{layout, EquationState, LayoutOptions};
use ratex_parser::parser::parse;

let eq_state = Rc::new(RefCell::new(EquationState::default()));
let opts = LayoutOptions {
    equation_state: Some(eq_state),
    ..LayoutOptions::default()
};
let ast = parse("\\begin{equation} E=mc^2 \\end{equation}").unwrap();
let lbox = layout(&ast, &opts);
```

### Two-pass label/ref workflow

```rust
use std::collections::HashMap;
// Pass 1: collect labels
let state = Rc::new(RefCell::new(EquationState::default()));
let opts = LayoutOptions { equation_state: Some(state.clone()), ..Default::default() };
layout(&parse("\\begin{equation} a=1 \\label{eq:a} \\end{equation}").unwrap(), &opts);
let labels: HashMap<String, usize> = state.borrow().labels.clone();

// Pass 2: resolve \ref
let state2 = Rc::new(RefCell::new(EquationState {
    external_labels: labels,
    ..EquationState::default()
}));
let opts2 = LayoutOptions { equation_state: Some(state2.clone()), ..Default::default() };
layout(&parse("\\ref{eq:a}").unwrap(), &opts2);
```

### Cross-session persistence

To persist numbering across application sessions, serialize `EquationState::labels` and restore it as `EquationState::external_labels` on next launch.

## Platform integration

Platform bindings (FFI, WASM) do NOT currently expose `EquationState`. To enable auto-numbering on a platform:

- **FFI**: Extend `RatexOptions` with fields for `start_counter` and `external_labels` (as JSON). Return collected `labels` in `RatexResult`.
- **WASM**: Extend the JSON input with `startCounter` / `externalLabels` fields. Return `labels` in the output.

The golden test runners (`ratex-render/tests/golden_test.rs`, `ratex-svg/tests/golden_svg.rs`) create and pass `EquationState` to `LayoutOptions`. See those files for reference implementations.

## Test coverage

- **Parser tests** (`ratex-parser`): Verify `Auto(true)` tags, `\label` extraction, `\notag` suppression, `\ref`/`\eqref` parsing.
- **Layout integration tests** (`ratex-layout`): Verify counter increments, label mapping, `\ref` resolution, two-pass workflow.
- **Golden tests** (`ratex-render`, `ratex-svg`): Compare rendered output against KaTeX reference PNGs (auto-numbered cases are at the end of `test_cases.txt`).
