//! Colours: a light and a dark palette around one accent, and the colour each signer wears.

use freya::prelude::*;

/// Indigo, the accent before there is a signer to take it from (and the coordinator's).
pub const INDIGO: Color = Color::from_rgb(63, 81, 181);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color,
    pub surface: Color,
    /// A surface set off from `surface`, for fields and nested boxes.
    pub surface_alt: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub accent_soft: Color,
    pub on_accent_soft: Color,
    pub error: Color,
    pub on_error: Color,
    pub error_soft: Color,
    pub on_error_soft: Color,
    pub success: Color,
    pub success_soft: Color,
    pub warn: Color,
    pub hover: Color,
}

impl Palette {
    pub fn new(dark: bool, accent: Color) -> Self {
        let c = |light: u32, darkv: u32| Color::new(if dark { darkv } else { light });
        let bg = c(0xFFF7F7FB, 0xFF121318);
        let error = c(0xFFBA1A1A, 0xFFFFB4AB);
        let accent_soft = mix(accent, bg, if dark { 0.72 } else { 0.84 });
        let error_soft = mix(error, bg, if dark { 0.75 } else { 0.86 });
        Self {
            dark,
            bg,
            surface: c(0xFFFFFFFF, 0xFF1D1E25),
            surface_alt: c(0xFFEEEEF4, 0xFF272832),
            border: c(0xFFD5D6E0, 0xFF3A3B47),
            text: c(0xFF1B1B21, 0xFFE4E3EA),
            text_muted: c(0xFF5C5D6B, 0xFFA5A6B4),
            accent,
            on_accent: readable_on(accent),
            accent_soft,
            on_accent_soft: readable_on(accent_soft),
            error,
            on_error: readable_on(error),
            error_soft,
            on_error_soft: readable_on(error_soft),
            success: c(0xFF2E7D32, 0xFF81C784),
            success_soft: c(0xFFDDF2DE, 0xFF1E3321),
            warn: c(0xFF9A6700, 0xFFFFC857),
            hover: c(0x14000000, 0x14FFFFFF),
        }
    }

    /// The same colours as a Freya theme, for the built-in components (inputs, checkboxes, scroll
    /// bars).
    pub fn freya_theme(&self) -> Theme {
        let mut theme = if self.dark { dark_theme() } else { light_theme() };
        theme.name = if self.dark { "dark" } else { "light" };
        let base = if self.dark { DARK_COLORS } else { LIGHT_COLORS };
        theme.colors = ColorsSheet {
            primary: self.accent,
            secondary: self.accent_soft,
            tertiary: mix(self.accent, self.bg, 0.2),
            error: self.error,
            success: self.success,
            background: self.bg,
            surface_primary: self.border,
            surface_secondary: self.surface_alt,
            surface_tertiary: self.surface,
            border: self.border,
            border_focus: self.accent,
            text_primary: self.text,
            text_secondary: self.text_muted,
            text_placeholder: self.text_muted,
            text_inverse: self.on_accent,
            text_highlight: self.accent,
            ..base
        };
        theme
    }
}

/// The palette in use, provided by [`use_init_palette`].
pub fn use_palette() -> Palette {
    *use_consume::<State<Palette>>().read()
}

/// Provides the palette (light or dark like the system, around the accent `accent(dark)` gives)
/// and the matching Freya theme to the whole app. Call once, at the root.
pub fn use_init_palette(accent: impl FnOnce(bool) -> Color) -> Palette {
    let window = Platform::get().current_window();
    let dark = *window.preferred_theme.read() == PreferredTheme::Dark;
    let palette = Palette::new(dark, accent(dark));
    let mut state = use_provide_context(|| State::create(palette));
    state.set_if_modified(palette);
    let mut theme = use_init_theme(|| palette.freya_theme());
    use_side_effect(move || {
        let p = *state.read();
        theme.set(p.freya_theme());
    });
    palette
}

/// A colour that belongs to one signer, derived from its device id so the signer and the
/// coordinator agree on it without exchanging anything.
pub fn signer_color(device_id: &str, dark: bool) -> Color {
    // the id is hex of a public key: a few leading digits are uniformly distributed
    let digits = &device_id[..device_id.len().min(8)];
    let seed = u64::from_str_radix(digits, 16).unwrap_or(0);
    let hue = (seed % 360) as f32;
    hsl(hue, 0.65, if dark { 0.68 } else { 0.38 })
}

/// A dark shade of `color` that still reads as that hue but keeps enough contrast against white
/// for a camera to pick out QR modules drawn in it (yellows are the worst case).
pub fn qr_ink(color: Color) -> Color {
    let (h, s, l) = to_hsl(color);
    hsl(h, s, l.min(0.26))
}

/// Black or white, whichever reads better on `background`.
pub fn readable_on(background: Color) -> Color {
    let lum = |v: u8| {
        let c = v as f32 / 255.;
        if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let l = 0.2126 * lum(background.r()) + 0.7152 * lum(background.g()) + 0.0722 * lum(background.b());
    if l > 0.4 { Color::from_rgb(20, 20, 24) } else { Color::WHITE }
}

/// `a` moved towards `b` by `t` (0 is `a`, 1 is `b`).
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

fn hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1. - (2. * l - 1.).abs()) * s;
    let hp = (h % 360.) / 60.;
    let x = c * (1. - (hp % 2. - 1.).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = l - c / 2.;
    let v = |f: f32| ((f + m) * 255.).round().clamp(0., 255.) as u8;
    Color::from_rgb(v(r), v(g), v(b))
}

fn to_hsl(color: Color) -> (f32, f32, f32) {
    let (r, g, b) = (color.r() as f32 / 255., color.g() as f32 / 255., color.b() as f32 / 255.);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.;
    if max == min {
        return (0., 0., l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2. - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6. } else { 0. }
    } else if max == g {
        (b - r) / d + 2.
    } else {
        (r - g) / d + 4.
    };
    (h * 60., s, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsl_round_trips() {
        for (h, s, l) in [(0., 0.65, 0.38), (120., 0.65, 0.68), (250., 0.5, 0.2)] {
            let (h2, s2, l2) = to_hsl(hsl(h, s, l));
            assert!((h - h2).abs() < 1.5 && (s - s2).abs() < 0.02 && (l - l2).abs() < 0.01);
        }
    }

    #[test]
    fn signer_colors_follow_the_id() {
        // hue 0x10 = 16
        assert_eq!(signer_color("00000010ab", false), hsl(16., 0.65, 0.38));
        assert_ne!(signer_color("00000010ab", false), signer_color("00000011ab", false));
    }
}
