/// Font families used in KaTeX math rendering.
///
/// Each variant corresponds to a specific OpenType font with pre-extracted metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontId {
    AmsRegular,
    CaligraphicRegular,
    FrakturRegular,
    /// Bold Fraktur — glyphs from `KaTeX_Fraktur-Bold.ttf`; advances from bold `hmtx` (`FRAKTUR_BOLD`).
    FrakturBold,
    MainBold,
    MainBoldItalic,
    MainItalic,
    MainRegular,
    MathBoldItalic,
    MathItalic,
    SansSerifBold,
    SansSerifItalic,
    SansSerifRegular,
    ScriptRegular,
    Size1Regular,
    Size2Regular,
    Size3Regular,
    Size4Regular,
    TypewriterRegular,
}

impl FontId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AmsRegular => "AMS-Regular",
            Self::CaligraphicRegular => "Caligraphic-Regular",
            Self::FrakturRegular => "Fraktur-Regular",
            Self::FrakturBold => "Fraktur-Bold",
            Self::MainBold => "Main-Bold",
            Self::MainBoldItalic => "Main-BoldItalic",
            Self::MainItalic => "Main-Italic",
            Self::MainRegular => "Main-Regular",
            Self::MathBoldItalic => "Math-BoldItalic",
            Self::MathItalic => "Math-Italic",
            Self::SansSerifBold => "SansSerif-Bold",
            Self::SansSerifItalic => "SansSerif-Italic",
            Self::SansSerifRegular => "SansSerif-Regular",
            Self::ScriptRegular => "Script-Regular",
            Self::Size1Regular => "Size1-Regular",
            Self::Size2Regular => "Size2-Regular",
            Self::Size3Regular => "Size3-Regular",
            Self::Size4Regular => "Size4-Regular",
            Self::TypewriterRegular => "Typewriter-Regular",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "AMS-Regular" => Some(Self::AmsRegular),
            "Caligraphic-Regular" => Some(Self::CaligraphicRegular),
            "Fraktur-Regular" => Some(Self::FrakturRegular),
            "Fraktur-Bold" => Some(Self::FrakturBold),
            "Main-Bold" => Some(Self::MainBold),
            "Main-BoldItalic" => Some(Self::MainBoldItalic),
            "Main-Italic" => Some(Self::MainItalic),
            "Main-Regular" => Some(Self::MainRegular),
            "Math-BoldItalic" => Some(Self::MathBoldItalic),
            "Math-Italic" => Some(Self::MathItalic),
            "SansSerif-Bold" => Some(Self::SansSerifBold),
            "SansSerif-Italic" => Some(Self::SansSerifItalic),
            "SansSerif-Regular" => Some(Self::SansSerifRegular),
            "Script-Regular" => Some(Self::ScriptRegular),
            "Size1-Regular" => Some(Self::Size1Regular),
            "Size2-Regular" => Some(Self::Size2Regular),
            "Size3-Regular" => Some(Self::Size3Regular),
            "Size4-Regular" => Some(Self::Size4Regular),
            "Typewriter-Regular" => Some(Self::TypewriterRegular),
            _ => None,
        }
    }
}

impl std::fmt::Display for FontId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
