use placard_css::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    Flex,
    Grid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackSize {
    Px(f32),
    Percent(f32),
    Fr(f32),
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Px(f32),
    Percent(f32),
}

impl Dimension {
    pub fn resolve(self, basis: f32) -> f32 {
        match self {
            Dimension::Px(v) => v,
            Dimension::Percent(p) => basis * p / 100.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    Normal,
    Number(f32),
    Px(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxSizing {
    ContentBox,
    BorderBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top = 0,
    Right = 1,
    Bottom = 2,
    Left = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub display: Display,
    pub color: Color,
    pub background_color: Color,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_family: Vec<FontFamily>,
    pub line_height: LineHeight,
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,

    pub margin: [Option<f32>; 4],
    pub padding: [f32; 4],
    pub border_width: [f32; 4],
    pub border_color: [Color; 4],
    pub border_style: [BorderStyle; 4],
    pub border_radius: [f32; 4],
    pub text_align: TextAlign,
    pub box_sizing: BoxSizing,

    pub position: Position,
    pub inset: [Option<Dimension>; 4],
    pub z_index: Option<i32>,

    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: Option<AlignItems>,
    pub row_gap: f32,
    pub column_gap: f32,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<Dimension>,

    pub grid_template_columns: Vec<TrackSize>,
    pub grid_template_rows: Vec<TrackSize>,
}

impl ComputedStyle {
    pub fn initial() -> Self {
        Self {
            display: Display::Inline,
            color: Color::rgb(0, 0, 0),
            background_color: Color::rgba(0, 0, 0, 0),
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_family: vec![FontFamily::SansSerif],
            line_height: LineHeight::Normal,
            width: None,
            height: None,
            margin: [Some(0.0); 4],
            padding: [0.0; 4],
            border_width: [0.0; 4],
            border_color: [Color::rgb(0, 0, 0); 4],
            border_style: [BorderStyle::None; 4],
            border_radius: [0.0; 4],
            text_align: TextAlign::Left,
            box_sizing: BoxSizing::ContentBox,

            position: Position::Static,
            inset: [None; 4],
            z_index: None,

            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_self: None,
            row_gap: 0.0,
            column_gap: 0.0,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,

            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
        }
    }

    pub fn inherited_from(parent: &ComputedStyle) -> Self {
        let mut style = Self::initial();
        style.color = parent.color;
        style.font_size = parent.font_size;
        style.font_weight = parent.font_weight;
        style.font_style = parent.font_style;
        style.font_family = parent.font_family.clone();
        style.line_height = parent.line_height;
        style.text_align = parent.text_align;
        style
    }
}
