use ratex_types::color::Color;
use ratex_types::path_command::PathCommand;

/// A TeX box: the fundamental unit of layout.
///
/// Every mathematical element is represented as a box with three dimensions:
/// - `width`: horizontal extent
/// - `height`: ascent above baseline
/// - `depth`: descent below baseline
///
/// All values are in **em** units relative to the current font size.
#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub content: BoxContent,
    pub color: Color,
}

/// What a LayoutBox contains.
#[derive(Debug, Clone)]
pub enum BoxContent {
    /// Horizontal list of child boxes laid out left-to-right.
    HBox(Vec<LayoutBox>),

    /// Vertical list of child boxes laid out top-to-bottom.
    VBox(Vec<VBoxChild>),

    /// A single glyph character.
    Glyph {
        font_id: ratex_font::FontId,
        char_code: u32,
    },

    /// Filled rectangle from `\rule[<raise>]{width}{height}`.
    /// `thickness` is the ink height; `raise` is the distance (in em) from the baseline
    /// to the bottom edge of the rectangle, positive toward the top of the line.
    Rule {
        thickness: f64,
        raise: f64,
    },

    /// Empty space (kern).
    Kern,

    /// A fraction: numerator over denominator with optional bar.
    Fraction {
        numer: Box<LayoutBox>,
        denom: Box<LayoutBox>,
        numer_shift: f64,
        denom_shift: f64,
        bar_thickness: f64,
        numer_scale: f64,
        denom_scale: f64,
    },

    /// Superscript/subscript layout.
    SupSub {
        base: Box<LayoutBox>,
        sup: Option<Box<LayoutBox>>,
        sub: Option<Box<LayoutBox>>,
        sup_shift: f64,
        sub_shift: f64,
        sup_scale: f64,
        sub_scale: f64,
        /// When true, place scripts centered on the base width (e.g. `\overbrace` / `\underbrace`).
        center_scripts: bool,
        /// Italic correction of the base character (em). Superscript x is offset by this amount
        /// beyond base.width, matching KaTeX's margin-right on italic math symbols.
        italic_correction: f64,
        /// Horizontal kern (em) applied to the subscript: KaTeX uses `margin-left: -base.italic` on
        /// `SymbolNode` bases so subscripts are not pushed out by the base's italic correction.
        sub_h_kern: f64,
    },

    /// A radical (square root).
    Radical {
        body: Box<LayoutBox>,
        index: Option<Box<LayoutBox>>,
        /// Horizontal offset (in em) of the surd/body from the left edge when index is present.
        index_offset: f64,
        /// `scriptscript` size relative to the surrounding math style (for drawing the index).
        index_scale: f64,
        rule_thickness: f64,
        inner_height: f64,
    },

    /// An operator with limits above/below (e.g. \sum_{i=0}^{n}).
    OpLimits {
        base: Box<LayoutBox>,
        sup: Option<Box<LayoutBox>>,
        sub: Option<Box<LayoutBox>>,
        base_shift: f64,
        sup_kern: f64,
        sub_kern: f64,
        slant: f64,
        sup_scale: f64,
        sub_scale: f64,
    },

    /// An accent above or below its base.
    Accent {
        base: Box<LayoutBox>,
        accent: Box<LayoutBox>,
        clearance: f64,
        skew: f64,
        is_below: bool,
        /// KaTeX `accentunder.js`: extra em gap between base bottom and under-accent (e.g. 0.12 for `\\utilde`).
        under_gap_em: f64,
    },

    /// A stretchy delimiter (\left, \right) wrapping inner content.
    LeftRight {
        left: Box<LayoutBox>,
        right: Box<LayoutBox>,
        inner: Box<LayoutBox>,
    },

    /// A matrix/array: rows × columns of cells.
    Array {
        cells: Vec<Vec<LayoutBox>>,
        col_widths: Vec<f64>,
        /// Per-column alignment: b'l', b'c', or b'r'.
        col_aligns: Vec<u8>,
        row_heights: Vec<f64>,
        row_depths: Vec<f64>,
        col_gap: f64,
        offset: f64,
        /// Extra x padding before the first column (= arraycolsep when hskip_before_and_after is true).
        content_x_offset: f64,
        /// For each column boundary (0 = before col 0, ..., num_cols = after last col),
        /// the vertical rule separator type: None = no rule, Some(false) = solid '|', Some(true) = dashed ':'.
        col_separators: Vec<Option<bool>>,
        /// For each row boundary (0 = before row 0, ..., num_rows = after last row),
        /// the list of hlines: false = solid, true = dashed.
        hlines_before_row: Vec<Vec<bool>>,
        /// Thickness of array rules in em.
        rule_thickness: f64,
        /// Gap between consecutive \hline or \hdashline rules (= \doublerulesep, in em).
        double_rule_sep: f64,
        /// Width of the cell grid including `content_x_offset` padding (em); excludes tag column.
        array_inner_width: f64,
        /// Horizontal gap between grid and tag column (em).
        tag_gap_em: f64,
        /// Width reserved for tags; tags are right-aligned in this column (em).
        tag_col_width: f64,
        /// Per-row tag layout; length matches number of rows.
        row_tags: Vec<Option<LayoutBox>>,
        /// When true, tags sit left of the grid (leqno-style).
        tags_left: bool,
    },

    /// An SVG-style path (arrows, braces, etc.).
    SvgPath {
        commands: Vec<PathCommand>,
        fill: bool,
    },

    /// A framed/colored box (fbox, colorbox, fcolorbox).
    /// body is the inner content; padding and border add to the outer dimensions.
    Framed {
        body: Box<LayoutBox>,
        padding: f64,
        border_thickness: f64,
        has_border: bool,
        bg_color: Option<Color>,
        border_color: Color,
    },

    /// A raised/lowered box (raisebox).
    /// shift > 0 moves content up, shift < 0 moves content down.
    RaiseBox {
        body: Box<LayoutBox>,
        shift: f64,
    },

    /// A scaled box (for \scriptstyle, \scriptscriptstyle in inline context).
    /// The child is rendered at child_scale relative to the parent.
    Scaled {
        body: Box<LayoutBox>,
        child_scale: f64,
    },

    /// Actuarial angle \angl{body}: path (horizontal roof + vertical bar) and body share the same baseline.
    Angl {
        path_commands: Vec<PathCommand>,
        body: Box<LayoutBox>,
    },

    /// \overline{body}: body with a horizontal rule drawn above it.
    /// The rule sits `2 * rule_thickness` above the body's top (clearance), and is `rule_thickness` thick.
    Overline {
        body: Box<LayoutBox>,
        rule_thickness: f64,
    },

    /// \underline{body}: body with a horizontal rule drawn below it.
    /// The rule sits `2 * rule_thickness` below the body's bottom (clearance), and is `rule_thickness` thick.
    Underline {
        body: Box<LayoutBox>,
        rule_thickness: f64,
        offset: f64,
    },

    /// Bussproofs-style proof tree with absolutely placed child boxes and inference rules.
    ProofTree {
        children: Vec<PlacedBox>,
        rules: Vec<ProofRule>,
    },

    /// Empty placeholder.
    Empty,
}

#[derive(Debug, Clone)]
pub struct PlacedBox {
    pub box_: LayoutBox,
    pub x: f64,
    pub baseline_y: f64,
}

#[derive(Debug, Clone)]
pub struct ProofRule {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub thickness: f64,
    pub dashed: bool,
}

/// A child element in a vertical box.
#[derive(Debug, Clone)]
pub struct VBoxChild {
    pub kind: VBoxChildKind,
    pub shift: f64,
}

#[derive(Debug, Clone)]
pub enum VBoxChildKind {
    Box(Box<LayoutBox>),
    Kern(f64),
}

impl LayoutBox {
    pub fn new_empty() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            depth: 0.0,
            content: BoxContent::Empty,
            color: Color::BLACK,
        }
    }

    pub fn new_kern(width: f64) -> Self {
        Self {
            width,
            height: 0.0,
            depth: 0.0,
            content: BoxContent::Kern,
            color: Color::BLACK,
        }
    }

    pub fn new_rule(width: f64, height: f64, depth: f64, thickness: f64, raise: f64) -> Self {
        Self {
            width,
            height,
            depth,
            content: BoxContent::Rule { thickness, raise },
            color: Color::BLACK,
        }
    }

    pub fn total_height(&self) -> f64 {
        self.height + self.depth
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Adjust height/depth for a delimiter to match a target size.
    pub fn with_adjusted_delim(mut self, height: f64, depth: f64) -> Self {
        self.height = height;
        self.depth = depth;
        self
    }
}
