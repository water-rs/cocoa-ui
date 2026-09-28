//! The platform's semantic colors, resolved to concrete components.
//!
//! `UIColor`'s semantic colors are dynamic: what they draw depends on the
//! trait collection in effect at resolution time. [`resolve`] answers under
//! a `UITraitCollection` for one interface style, so the result is for that
//! scheme exactly.
//!
//! # Safety
//!
//! The `unsafe` here reads `UIColor`'s component accessors on a color resolved
//! under a trait collection — documented API, on the calling thread.

use objc2::rc::Retained;
use objc2_foundation::NSString;
use objc2_ui_kit::{UIColor, UITraitCollection, UIUserInterfaceStyle};

use crate::color::Rgba;
use crate::color_scheme::ColorScheme;

/// One of `UIKit`'s semantic colors, or the app's accent color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiColor {
    /// `UIColor.systemBackgroundColor`.
    SystemBackground,
    /// `UIColor.secondarySystemBackgroundColor`.
    SecondarySystemBackground,
    /// `UIColor.tertiarySystemFillColor`.
    TertiarySystemFill,
    /// `UIColor.separatorColor`.
    Separator,
    /// `UIColor.labelColor`.
    Label,
    /// `UIColor.secondaryLabelColor`.
    SecondaryLabel,
    /// The asset catalog's `AccentColor`, or `UIColor.systemBlueColor` when
    /// the app does not define one — the same answer `UIColor.tintColor`
    /// resolves to for a stock app.
    Accent,
    /// `UIColor.whiteColor`.
    White,
    /// `UIColor.systemPurple`.
    SystemPurple,
    /// `UIColor.systemRed`.
    SystemRed,
}

fn native(color: UiColor) -> Retained<UIColor> {
    match color {
        UiColor::SystemBackground => UIColor::systemBackgroundColor(),
        UiColor::SecondarySystemBackground => UIColor::secondarySystemBackgroundColor(),
        UiColor::TertiarySystemFill => UIColor::tertiarySystemFillColor(),
        UiColor::Separator => UIColor::separatorColor(),
        UiColor::Label => UIColor::labelColor(),
        UiColor::SecondaryLabel => UIColor::secondaryLabelColor(),
        UiColor::Accent => UIColor::colorNamed(&NSString::from_str("AccentColor"))
            .unwrap_or_else(UIColor::systemBlueColor),
        UiColor::White => UIColor::whiteColor(),
        UiColor::SystemPurple => UIColor::systemPurpleColor(),
        UiColor::SystemRed => UIColor::systemRedColor(),
    }
}

/// What `color` draws as under `scheme`.
#[must_use]
pub fn resolve(color: UiColor, scheme: ColorScheme) -> Rgba {
    let style = match scheme {
        ColorScheme::Light => UIUserInterfaceStyle::Light,
        ColorScheme::Dark => UIUserInterfaceStyle::Dark,
    };
    let traits = UITraitCollection::traitCollectionWithUserInterfaceStyle(style);
    let resolved = native(color).resolvedColorWithTraitCollection(&traits);
    // SAFETY: the component accessors are valid on any `UIColor`, and the out
    // pointers point at locals that outlive the call.
    unsafe {
        let mut red = 0.0;
        let mut green = 0.0;
        let mut blue = 0.0;
        let mut alpha = 0.0;
        resolved.getRed_green_blue_alpha(
            &raw mut red,
            &raw mut green,
            &raw mut blue,
            &raw mut alpha,
        );
        Rgba::new(red, green, blue, alpha)
    }
}
