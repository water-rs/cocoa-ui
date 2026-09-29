//! A resolved color as plain components.
//!
//! Platform color types stay inside the kit's platform modules; this is the
//! value they resolve to when a specific appearance is applied.

/// A red/green/blue/alpha color, each component in `0.0` to `1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// The red component.
    pub red: f64,
    /// The green component.
    pub green: f64,
    /// The blue component.
    pub blue: f64,
    /// The opacity component.
    pub alpha: f64,
}

impl Rgba {
    /// An sRGB color.
    #[must_use]
    pub const fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// The same color with `alpha` replacing its opacity.
    #[must_use]
    pub const fn with_alpha(self, alpha: f64) -> Self {
        Self { alpha, ..self }
    }
}

/// A `CGColor` in the extended linear sRGB space, `headroom` scaling the
/// color channels by `1.0 + headroom` — the `ResolvedColor` conversion both
/// platforms share.
///
/// # Panics
///
/// When `headroom` is NaN — the channel scale becomes NaN and the color
/// creation traps in Core Graphics.
#[must_use]
pub fn cg_extended_linear(
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
    headroom: f64,
) -> objc2_core_foundation::CFRetained<objc2_core_graphics::CGColor> {
    use objc2_core_graphics::{CGColor, CGColorSpace, kCGColorSpaceExtendedLinearSRGB};
    let scale = 1.0 + headroom;
    let components = [
        red * scale,
        green * scale,
        blue * scale,
        alpha.clamp(0.0, 1.0),
    ];
    // SAFETY: the static is a `CFString` constant exported by Core Graphics.
    let space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceExtendedLinearSRGB }));
    // SAFETY: `components` points at four f64s, the count extended sRGB takes.
    unsafe { CGColor::new(space.as_deref(), components.as_ptr()) }
        .expect("extended sRGB and four components always make a color")
}
