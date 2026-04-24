use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ratex_font::{get_global_metrics, MathConstants};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;

/// Mutable equation numbering state shared across a single layout pass.
///
/// Controls auto-number generation for non-starred display environments
/// and collects label→number mappings for cross-referencing.
///
/// The counter starts at 1 and auto-increments for each auto-numbered row.
/// `external_labels` should be populated from a prior render pass so that
/// `\ref` / `\eqref` nodes can resolve their targets.
#[derive(Debug, Clone)]
pub struct EquationState {
    /// Current equation number (starts at 1, increments per auto-numbered row).
    pub counter: usize,
    /// Labels collected during this layout pass: label_text → equation_number.
    pub labels: HashMap<String, usize>,
    /// Labels provided from outside (e.g. from a previous render pass) for \ref resolution.
    pub external_labels: HashMap<String, usize>,
}

impl Default for EquationState {
    fn default() -> Self {
        Self {
            counter: 1,
            labels: HashMap::new(),
            external_labels: HashMap::new(),
        }
    }
}

/// Layout options passed through the layout tree.
#[derive(Debug, Clone)]
pub struct LayoutOptions {
    pub style: MathStyle,
    pub color: Color,
    /// When set (e.g. in align/aligned), cap relation spacing to this many mu for consistency.
    pub align_relation_spacing: Option<f64>,
    /// When inside \\left...\\right, the stretch height for \\middle delimiters (second pass only).
    pub leftright_delim_height: Option<f64>,
    /// Extra horizontal kern between glyphs (em), e.g. for `\\url` / `\\href` to match browser tracking.
    pub inter_glyph_kern_em: f64,
    /// Shared equation numbering state (None = auto-numbering disabled).
    pub equation_state: Option<Rc<RefCell<EquationState>>>,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            style: MathStyle::Display,
            color: Color::BLACK,
            align_relation_spacing: None,
            leftright_delim_height: None,
            inter_glyph_kern_em: 0.0,
            equation_state: None,
        }
    }
}

impl LayoutOptions {
    pub fn metrics(&self) -> &'static MathConstants {
        get_global_metrics(self.style.size_index())
    }

    pub fn size_multiplier(&self) -> f64 {
        self.style.size_multiplier()
    }

    pub fn with_style(&self, style: MathStyle) -> Self {
        Self {
            style,
            color: self.color,
            align_relation_spacing: self.align_relation_spacing,
            leftright_delim_height: self.leftright_delim_height,
            inter_glyph_kern_em: self.inter_glyph_kern_em,
            equation_state: self.equation_state.clone(),
        }
    }

    pub fn with_color(&self, color: Color) -> Self {
        Self {
            style: self.style,
            color,
            align_relation_spacing: self.align_relation_spacing,
            leftright_delim_height: self.leftright_delim_height,
            inter_glyph_kern_em: self.inter_glyph_kern_em,
            equation_state: self.equation_state.clone(),
        }
    }

    pub fn with_inter_glyph_kern(&self, em: f64) -> Self {
        Self {
            inter_glyph_kern_em: em,
            equation_state: self.equation_state.clone(),
            ..self.clone()
        }
    }
}
