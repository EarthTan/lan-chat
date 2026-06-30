//! UI design tokens — 4px/8px grid, 5-step spacing, 5-step font scale, 1 mono font.
//!
//! This is the single source of truth for visual constants. All UI code references
//! these tokens by name; never use raw pixel values inline.

use eframe::egui::{self, FontFamily, Stroke, TextStyle};

// ── 8px grid ───────────────────────────────────────────────
// 0 / 4 / 8 / 12 / 16 / 24 / 32 / 48
pub mod space {
    pub const NONE: f32 = 0.0;
    pub const XS: f32 = 4.0; // 1 unit
    pub const SM: f32 = 8.0; // 2 units
    pub const MD: f32 = 12.0; // 3 units
    pub const LG: f32 = 16.0; // 4 units
    pub const XL: f32 = 24.0; // 6 units
    pub const XXL: f32 = 32.0; // 8 units
    pub const XXXL: f32 = 48.0; // 12 units
}

// ── Color tokens ───────────────────────────────────────────
pub mod color {
    use eframe::egui::Color32;
    pub const BG: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0a);
    pub const BG_ELEV: Color32 = Color32::from_rgb(0x11, 0x12, 0x11);
    pub const BG_INPUT: Color32 = Color32::from_rgb(0x0d, 0x0e, 0x0d);
    pub const LINE: Color32 = Color32::from_rgb(0x1f, 0x20, 0x1d);
    pub const LINE_SOFT: Color32 = Color32::from_rgb(0x15, 0x16, 0x0f);
    pub const AMBER: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
    pub const AMBER_DIM: Color32 = Color32::from_rgb(0x80, 0x58, 0x00);
    pub const GREEN: Color32 = Color32::from_rgb(0x7c, 0xff, 0xb2);
    pub const TEXT: Color32 = Color32::from_rgb(0xd9, 0xd6, 0xcc);
    pub const MUTED: Color32 = Color32::from_rgb(0x5c, 0x5d, 0x54);
    pub const DANGER: Color32 = Color32::from_rgb(0xff, 0x55, 0x55);
}

// ── Font sizes (baseline 15, slightly larger than original 13) ──
// 12 / 13 / 15 / 18 / 22
pub mod font {
    use eframe::egui::{FontFamily, FontId};
    pub const XS: f32 = 12.0;
    pub const SM: f32 = 13.0;
    pub const BASE: f32 = 15.0;
    pub const LG: f32 = 18.0;
    pub const XL: f32 = 22.0;

    /// The one and only font: JetBrains Mono (with system fallbacks).
    pub fn mono(size: f32) -> FontId {
        FontId::new(size, FontFamily::Monospace)
    }
}

// ── Strok widths ──────────────────────────────────────────
pub mod stroke {
    pub const HAIRLINE: f32 = 1.0;
    pub const THIN: f32 = 1.5;
    pub const MEDIUM: f32 = 2.0;
}

// ── Standard lines ────────────────────────────────────────
pub fn line_horizontal() -> Stroke {
    Stroke::new(stroke::HAIRLINE, color::LINE)
}
pub fn line_horizontal_soft() -> Stroke {
    Stroke::new(stroke::HAIRLINE, color::LINE_SOFT)
}
pub fn line_focused() -> Stroke {
    Stroke::new(stroke::HAIRLINE, color::AMBER)
}

// ── Apply our font sizes as the app's default text styles ──
/// Call this once at startup to register our type scale with egui.
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Small, font::mono(font::SM)),
        (TextStyle::Body, font::mono(font::BASE)),
        (TextStyle::Button, font::mono(font::SM)),
        (TextStyle::Heading, font::mono(font::XL)),
        (TextStyle::Monospace, font::mono(font::BASE)),
    ]
    .into();
    // Reduce the default body size effect
    style.spacing.item_spacing = egui::vec2(space::SM, space::SM);
    style.spacing.button_padding = egui::vec2(space::LG, space::MD);
    style.spacing.window_margin = egui::Margin::same(space::XL);
    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(color::TEXT);
    style.visuals.widgets.noninteractive.bg_fill = color::BG;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(stroke::HAIRLINE, color::TEXT);
    style.visuals.widgets.inactive.bg_fill = color::BG_ELEV;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(stroke::HAIRLINE, color::LINE);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(stroke::HAIRLINE, color::TEXT);
    style.visuals.widgets.hovered.bg_fill = color::BG_ELEV;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(stroke::HAIRLINE, color::AMBER);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(stroke::HAIRLINE, color::AMBER);
    style.visuals.widgets.active.bg_fill = color::AMBER;
    style.visuals.widgets.active.fg_stroke = Stroke::new(stroke::HAIRLINE, color::BG);
    style.visuals.selection.bg_fill = color::AMBER_DIM;
    style.visuals.selection.stroke = Stroke::new(stroke::HAIRLINE, color::AMBER);
    style.visuals.hyperlink_color = color::AMBER;
    style.visuals.extreme_bg_color = color::BG;
    style.visuals.faint_bg_color = color::BG_ELEV;
    ctx.set_style(style);

    // ── CJK fallback font ──────────────────────────────────
    // The egui/eframe default `Monospace` family on Linux/macOS/Windows only
    // covers Latin, so CJK codepoints rendered via cosmic-text end up as
    // mojibake or `U+FFFD`. We bundle a subset of Sarasa Mono SC (OFL-1.1,
    // derived from be5invis/Sarasa-Gothic) as a fallback at the END of both
    // the Monospace and Proportional chains — Latin stays on the system mono
    // for consistency, CJK silently falls through to Sarasa.
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".into(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/sarasa-mono-sc-subset.ttf"
        ))
        .into(),
    );
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("cjk".into());
    }
    ctx.set_fonts(fonts);
}
