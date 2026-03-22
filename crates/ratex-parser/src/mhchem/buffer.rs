//! Parser buffer (mirrors KaTeX mhchem `buffer` object).

#[derive(Clone, Default, Debug)]
pub struct Buffer {
    pub parenthesis_level: i32,
    pub begins_with_bond: bool,
    pub sb: bool,
    pub a: Option<String>,
    pub b: Option<String>,
    pub p: Option<String>,
    pub o: Option<String>,
    pub q: Option<String>,
    pub d: Option<String>,
    pub d_type: Option<String>,
    pub r: Option<String>,
    pub rdt: Option<String>,
    pub rd: Option<String>,
    pub rqt: Option<String>,
    pub rq: Option<String>,
    pub text_: Option<String>,
    pub rm: Option<String>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            parenthesis_level: 0,
            begins_with_bond: false,
            ..Default::default()
        }
    }

    /// Clear all fields except `parenthesis_level` and `begins_with_bond`.
    pub fn clear_soft(&mut self) {
        self.sb = false;
        self.a = None;
        self.b = None;
        self.p = None;
        self.o = None;
        self.q = None;
        self.d = None;
        self.d_type = None;
        self.r = None;
        self.rdt = None;
        self.rd = None;
        self.rqt = None;
        self.rq = None;
        self.text_ = None;
        self.rm = None;
    }

    pub fn clear_all(&mut self) {
        *self = Self::new();
    }
}
