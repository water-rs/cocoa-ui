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
