from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one replacement target, found {count}")
    p.write_text(text.replace(old, new, 1))


font_loader = "crates/ratex-font-loader/src/lib.rs"
replace_once(
    font_loader,
    '''impl FontLoadPlan {
    pub fn for_display_items(items: &[DisplayItem]) -> Self {
        let mut required = HashSet::new();

        for item in items {
            if let DisplayItem::GlyphPath { font, .. } = item {
                if let Some(font_id) = FontId::parse(font) {
                    required.insert(font_id);
                }
            }
        }

        required.insert(FontId::MainRegular);

        // System Unicode fallbacks are deliberately not part of the initial
        // plan. Renderers load each layer only after the previous font cannot
        // draw a glyph, keeping large emoji TTCs off normal render paths.
        Self {
            required,
            optional: HashSet::new(),
        }
    }

    pub fn required(&self) -> &HashSet<FontId> {
        &self.required
    }

    pub fn all(&self) -> HashSet<FontId> {
        self.required.union(&self.optional).copied().collect()
    }
}
''',
    '''impl FontLoadPlan {
    pub fn for_display_items(items: &[DisplayItem]) -> Self {
        let mut required = HashSet::new();
        let mut optional = HashSet::new();
        let mut needs_optional_unicode_fallbacks = false;

        for item in items {
            if let DisplayItem::GlyphPath {
                font, char_code, ..
            } = item
            {
                if let Some(font_id) = FontId::parse(font) {
                    match font_id {
                        FontId::CjkRegular | FontId::CjkFallback | FontId::EmojiFallback => {
                            required.insert(font_id);
                            needs_optional_unicode_fallbacks = true;
                        }
                        _ => {
                            required.insert(font_id);
                        }
                    }
                    if may_need_runtime_unicode_fallback(font_id, *char_code) {
                        needs_optional_unicode_fallbacks = true;
                    }
                }
            }
        }

        required.insert(FontId::MainRegular);

        if needs_optional_unicode_fallbacks {
            optional.insert(FontId::CjkRegular);
            optional.insert(FontId::EmojiFallback);
            optional.insert(FontId::CjkFallback);
        }

        Self { required, optional }
    }

    /// Build the lazy load plan used by RaTeX's built-in renderers.
    pub fn for_display_items_lazy(items: &[DisplayItem]) -> Self {
        let mut required = HashSet::new();

        for item in items {
            if let DisplayItem::GlyphPath { font, .. } = item {
                if let Some(font_id) = FontId::parse(font) {
                    required.insert(font_id);
                }
            }
        }

        required.insert(FontId::MainRegular);
        Self {
            required,
            optional: HashSet::new(),
        }
    }

    pub fn required(&self) -> &HashSet<FontId> {
        &self.required
    }

    pub fn all(&self) -> HashSet<FontId> {
        self.required.union(&self.optional).copied().collect()
    }
}

fn may_need_runtime_unicode_fallback(font_id: FontId, char_code: u32) -> bool {
    matches!(
        font_id,
        FontId::CjkRegular | FontId::CjkFallback | FontId::EmojiFallback
    ) || (char_code > 0x7f && ratex_font::get_char_metrics(font_id, char_code).is_none())
}
''',
)

replace_once(
    font_loader,
    '''    let plan = FontLoadPlan::for_display_items(items);
    load_fonts_for_plan_parsed(font_dir, &plan)
}
''',
    '''    let plan = FontLoadPlan::for_display_items_lazy(items);
    load_fonts_for_plan_parsed(font_dir, &plan)
}
''',
)

replace_once(
    font_loader,
    '''pub fn load_fonts_for_items(font_dir: &str, items: &[DisplayItem]) -> Result<FontSet, String> {
    let plan = FontLoadPlan::for_display_items(items);
    load_fonts_for_plan(font_dir, &plan)
}

pub fn load_fonts_for_plan(font_dir: &str, plan: &FontLoadPlan) -> Result<FontSet, String> {
''',
    '''pub fn load_fonts_for_items(font_dir: &str, items: &[DisplayItem]) -> Result<FontSet, String> {
    let plan = FontLoadPlan::for_display_items(items);
    load_fonts_for_plan(font_dir, &plan)
}

/// Load the initial font set for built-in renderers without eagerly loading
/// large system CJK/emoji fallback fonts.
pub fn load_fonts_for_items_lazy(
    font_dir: &str,
    items: &[DisplayItem],
) -> Result<FontSet, String> {
    let plan = FontLoadPlan::for_display_items_lazy(items);
    load_fonts_for_plan(font_dir, &plan)
}

pub fn load_fonts_for_plan(font_dir: &str, plan: &FontLoadPlan) -> Result<FontSet, String> {
''',
)

replace_once(
    font_loader,
    '''    #[test]
    fn missing_non_ascii_glyph_defers_unicode_fallbacks() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::MainRegular, '⌘' as u32)]);

        assert!(plan.required.contains(&FontId::MainRegular));
        assert!(plan.optional.is_empty());
    }

    #[test]
    fn explicit_cjk_glyph_requires_primary_cjk_font() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::CjkRegular, '你' as u32)]);

        assert!(plan.required.contains(&FontId::CjkRegular));
        assert!(plan.optional.is_empty());
    }
''',
    '''    #[test]
    fn non_ascii_without_katex_metrics_keeps_legacy_unicode_fallbacks() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::MainRegular, '⌘' as u32)]);

        assert!(plan.required.contains(&FontId::MainRegular));
        assert!(plan.optional.contains(&FontId::CjkRegular));
        assert!(plan.optional.contains(&FontId::EmojiFallback));
        assert!(plan.optional.contains(&FontId::CjkFallback));
        assert!(!plan.required.contains(&FontId::CjkRegular));
    }

    #[test]
    fn explicit_cjk_glyph_keeps_legacy_optional_fallbacks() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::CjkRegular, '你' as u32)]);

        assert!(plan.required.contains(&FontId::CjkRegular));
        assert!(plan.optional.contains(&FontId::EmojiFallback));
        assert!(plan.optional.contains(&FontId::CjkFallback));
    }

    #[test]
    fn lazy_plan_defers_unicode_fallbacks() {
        let missing =
            FontLoadPlan::for_display_items_lazy(&[glyph(FontId::MainRegular, '⌘' as u32)]);
        assert!(missing.required.contains(&FontId::MainRegular));
        assert!(missing.optional.is_empty());

        let explicit =
            FontLoadPlan::for_display_items_lazy(&[glyph(FontId::CjkRegular, '你' as u32)]);
        assert!(explicit.required.contains(&FontId::CjkRegular));
        assert!(explicit.optional.is_empty());
    }
''',
)

replace_once(
    "crates/ratex-pdf/src/renderer.rs",
    "ratex_font_loader::load_fonts_for_items(&options.font_dir, &display_list.items)",
    "ratex_font_loader::load_fonts_for_items_lazy(&options.font_dir, &display_list.items)",
)
replace_once(
    "crates/ratex-cairo/src/lib.rs",
    "ratex_font_loader::load_fonts_for_items(font_dir, &display_list.items)",
    "ratex_font_loader::load_fonts_for_items_lazy(font_dir, &display_list.items)",
)

renderer = "crates/ratex-render/src/renderer.rs"
replace_once(
    renderer,
    "fn raster_glyph_image_to_pixmap(img: &ttf_parser::RasterGlyphImage<'_>) -> Option<Pixmap> {",
    "#[allow(clippy::chunks_exact_to_as_chunks)]\nfn raster_glyph_image_to_pixmap(img: &ttf_parser::RasterGlyphImage<'_>) -> Option<Pixmap> {",
)
replace_once(
    renderer,
    "fn demultiply_rgba(data: &[u8]) -> Vec<u8> {",
    "#[allow(clippy::chunks_exact_to_as_chunks)]\nfn demultiply_rgba(data: &[u8]) -> Vec<u8> {",
)
