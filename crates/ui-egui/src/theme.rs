//! Design tokens for BimCraft, taken 1:1 from the VectorCraft `Tokens` table
//! (crates/ui-egui/src/theme.rs, storytold, MIT OR Apache-2.0): the Craft look
//! — cool slate greys, flat faces with soft corners, and one accent (the app
//! icon's coral) for what is on, chosen or focused — in the four brightness
//! levels VectorCraft ships.
//!
//! Widget state fills are applied through the adopted `craft-ui` widget
//! palette mechanism (storytold craft-libs, MIT OR Apache-2.0).

use craft_ui::theme::WidgetPalette;
use egui::{Color32, CornerRadius, FontId, Stroke, Style, Visuals};

/// Which of the VectorCraft brightness levels is active. `Charcoal` is the
/// VectorCraft default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    /// VC `Brightness::Dark` — near-black chrome.
    Dark,
    /// VC `Brightness::MediumDark` — charcoal, the default.
    #[default]
    Charcoal,
    /// VC `Brightness::MediumLight`.
    Slate,
    /// VC `Brightness::Light`.
    Light,
}

impl Theme {
    pub const ALL: [Theme; 4] = [Theme::Dark, Theme::Charcoal, Theme::Slate, Theme::Light];
    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Charcoal => "Charcoal",
            Theme::Slate => "Slate",
            Theme::Light => "Light",
        }
    }
    /// Whether light-on-dark (false) or dark-on-light (true).
    pub fn is_light(self) -> bool {
        matches!(self, Theme::Slate | Theme::Light)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub dark: bool,
    pub app_bar: Color32,
    pub panel: Color32,
    pub panel_darker: Color32,
    pub tab_strip: Color32,
    pub input: Color32,
    pub input_border: Color32,
    pub border: Color32,
    pub divider: Color32,
    pub text: Color32,
    pub text_strong: Color32,
    pub text_dim: Color32,
    pub text_disabled: Color32,
    pub icon: Color32,
    pub hover: Color32,
    pub tool_active: Color32,
    pub accent: Color32,
    pub accent_strong: Color32,
    pub flyout_active: Color32,
    pub row_selected: Color32,
    pub button: Color32,
    pub button_border: Color32,
    pub pasteboard: Color32,
    pub viewport_bg: Color32,
    pub selection: Color32,
    pub axis_x: Color32,
    pub axis_y: Color32,
    pub axis_z: Color32,
    pub error: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub radius: f32,
    pub font_size: f32,
}

fn hex(s: u32) -> Color32 {
    Color32::from_rgb((s >> 16) as u8, (s >> 8) as u8, s as u8)
}

/// The Craft look at brightness `b`, straight from the VectorCraft table.
pub fn tokens_for(theme: Theme) -> Tokens {
    // Shared across brightness levels (accents, semantic colors, axes).
    let base = Tokens {
        dark: true,
        app_bar: hex(0x0b0d12),
        panel: hex(0x181b22),
        panel_darker: hex(0x13161c),
        tab_strip: hex(0x0f1116),
        input: hex(0x0f1116),
        input_border: hex(0x2c323e),
        border: hex(0x06070a),
        divider: hex(0x232832),
        text: hex(0xdfe4ec),
        text_strong: hex(0xffffff),
        text_dim: hex(0x969faf),
        text_disabled: hex(0x5a6271),
        icon: hex(0xc9d0dc),
        hover: hex(0x262b35),
        tool_active: hex(0x2f3541),
        accent: hex(0xf06a52),
        accent_strong: hex(0xe8573f),
        flyout_active: hex(0xff8a74),
        row_selected: hex(0x3a2a2c),
        button: hex(0x282d37),
        button_border: hex(0x4a5363),
        pasteboard: hex(0x101217),
        viewport_bg: hex(0x1a1e26),
        selection: hex(0x4f80ff),
        axis_x: hex(0xef6058),
        axis_y: hex(0x45af63),
        axis_z: hex(0x378ef0),
        error: hex(0xe34850),
        warning: hex(0xf0a330),
        danger: hex(0xe34850),
        radius: 4.0,
        font_size: 13.0,
    };
    match theme {
        Theme::Dark => base,
        Theme::Charcoal => Tokens {
            dark: true,
            app_bar: hex(0x1b1f27),
            panel: hex(0x272c36),
            panel_darker: hex(0x21262f),
            tab_strip: hex(0x1e222a),
            input: hex(0x1a1e26),
            input_border: hex(0x3a414e),
            border: hex(0x11141a),
            divider: hex(0x323946),
            text: hex(0xe4e9f1),
            text_dim: hex(0x99a2b2),
            text_disabled: hex(0x626b7a),
            icon: hex(0xccd3de),
            hover: hex(0x353c49),
            tool_active: hex(0x3d4553),
            row_selected: hex(0x4a3236),
            pasteboard: hex(0x1e222a),
            viewport_bg: hex(0x222730),
            button: hex(0x39404d),
            button_border: hex(0x5a6476),
            ..base
        },
        Theme::Slate => Tokens {
            dark: false,
            app_bar: hex(0xc4cad4),
            panel: hex(0xd3d8e0),
            panel_darker: hex(0xc9cfd8),
            tab_strip: hex(0xc4cad4),
            input: hex(0xe9ecf1),
            input_border: hex(0x9aa2b0),
            border: hex(0xa9b0bc),
            divider: hex(0xbcc3ce),
            text: hex(0x1b1f27),
            text_strong: hex(0x000000),
            text_dim: hex(0x4a5160),
            text_disabled: hex(0x7d8594),
            icon: hex(0x272c36),
            hover: hex(0xc5ccd6),
            tool_active: hex(0xb6bec9),
            accent: hex(0xd9472f),
            accent_strong: hex(0xd14a33),
            flyout_active: hex(0xc23e28),
            row_selected: hex(0xe7c4bd),
            pasteboard: hex(0xb9c0cb),
            viewport_bg: hex(0xcfd4dd),
            button: hex(0xc9d0da),
            button_border: hex(0x8a93a2),
            ..base
        },
        Theme::Light => Tokens {
            dark: false,
            app_bar: hex(0xe6e9ee),
            panel: hex(0xf3f5f8),
            panel_darker: hex(0xe9ecf1),
            tab_strip: hex(0xe6e9ee),
            input: hex(0xffffff),
            input_border: hex(0xc3c9d3),
            border: hex(0xd0d5dd),
            divider: hex(0xdde1e8),
            text: hex(0x1b1f27),
            text_strong: hex(0x000000),
            text_dim: hex(0x58606e),
            text_disabled: hex(0x9aa1ad),
            icon: hex(0x2a303b),
            hover: hex(0xe3e7ed),
            tool_active: hex(0xd5dbe3),
            accent: hex(0xd9472f),
            accent_strong: hex(0xd14a33),
            flyout_active: hex(0xc23e28),
            row_selected: hex(0xf8d9d2),
            pasteboard: hex(0xdfe3e9),
            viewport_bg: hex(0xe8ebf0),
            button: hex(0xe3e7ed),
            button_border: hex(0xa4abb8),
            ..base
        },
    }
}

/// Back-compat: the default Charcoal tokens.
pub fn tokens() -> Tokens {
    tokens_for(Theme::Charcoal)
}

pub const CARD_RADIUS: u8 = 8;

pub fn font_ui(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn font_mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Icon-button style shared by the app bar and the tool rail.
pub fn icon_button_style(t: &Tokens) -> craft_ui::buttons::IconButtonStyle {
    use craft_ui::buttons::{ButtonVisuals, IconButtonStyle};
    IconButtonStyle {
        normal: ButtonVisuals { fill: t.app_bar, stroke: Stroke::NONE, icon: t.icon },
        hovered: ButtonVisuals { fill: t.hover, stroke: Stroke::NONE, icon: t.text_strong },
        pressed: Some(ButtonVisuals { fill: t.tool_active, stroke: Stroke::NONE, icon: t.text_strong }),
        selected: ButtonVisuals { fill: t.tool_active, stroke: Stroke::new(1.0, t.accent), icon: t.text_strong },
        corner_radius: CornerRadius::same(6),
        focus_stroke: Stroke::NONE,
    }
}

/// Apply the Craft look to the egui style. Call once per frame start.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let t = tokens_for(theme);
    let mut style = Style { visuals: visuals(&t), ..Style::default() };
    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.interact_size = egui::vec2(24.0, 24.0);
    style.text_styles.insert(egui::TextStyle::Small, font_ui(11.0));
    style.text_styles.insert(egui::TextStyle::Body, font_ui(t.font_size));
    style.text_styles.insert(egui::TextStyle::Button, font_ui(t.font_size));
    style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(15.0));
    style.text_styles.insert(egui::TextStyle::Monospace, font_mono(11.5));
    style.animation_time = 0.08;
    ctx.all_styles_mut(move |s| *s = style.clone());
}

fn visuals(t: &Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.panel;
    v.window_fill = t.panel;
    v.extreme_bg_color = t.input;
    v.faint_bg_color = t.panel_darker;
    v.widgets.noninteractive.bg_fill = t.panel_darker;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text_dim);
    v.widgets.inactive.bg_fill = t.input;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.input_border);
    v.widgets.hovered.bg_fill = t.hover;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.active.bg_fill = t.tool_active;
    v.widgets.active.fg_stroke = Stroke::new(1.0, t.accent);
    v.selection.bg_fill = t.selection.gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0, t.selection);
    v.override_text_color = Some(t.text);
    // Adopted craft-ui mechanism: one palette drives all five widget states.
    WidgetPalette { noninteractive: t.panel_darker, inactive: t.input, hovered: t.hover, active: t.tool_active, open: t.panel }.apply_to(
        &mut v.widgets,
        CornerRadius::same(4),
        Stroke::new(1.0, t.text),
    );
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_match_vectorcraft_values() {
        let c = tokens_for(Theme::Charcoal);
        assert_eq!(c.app_bar, hex(0x1b1f27));
        assert_eq!(c.panel, hex(0x272c36));
        assert_eq!(c.accent, hex(0xf06a52));
        assert_eq!(c.selection, hex(0x4f80ff));
        assert_eq!(c.row_selected, hex(0x4a3236));
        assert_eq!(c.axis_x, hex(0xef6058));
        let d = tokens_for(Theme::Dark);
        assert_eq!(d.app_bar, hex(0x0b0d12));
        let l = tokens_for(Theme::Light);
        assert!(!l.dark);
        assert_eq!(l.panel, hex(0xf3f5f8));
        assert_eq!(l.accent, hex(0xd9472f));
    }
}
