//! The platform's semantic colors, resolved to concrete components.
//!
//! `NSColor`'s semantic colors are dynamic: what they draw depends on the
//! appearance in effect at resolution time. [`resolve`] applies an
//! `NSAppearance` around the read so the answer is for one scheme exactly.
//!
//! # Safety
//!
//! The `unsafe` here asks `NSAppearance` to run a block under a named
//! appearance and reads `NSColor`'s component accessors inside it. Both are
//! documented APIs; the block runs synchronously on the calling thread while
//! that appearance is in effect.

use core::cell::RefCell;

use objc2::rc::Retained;
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSColor, NSColorSpace,
};

use crate::color::Rgba;
use crate::color_scheme::ColorScheme;

/// One of `AppKit`'s semantic colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppColor {
    /// `NSColor.windowBackgroundColor`.
    WindowBackground,
    /// `NSColor.controlBackgroundColor`.
    ControlBackground,
    /// `NSColor.tertiarySystemFill`.
    TertiarySystemFill,
    /// `NSColor.separatorColor`.
    Separator,
    /// `NSColor.labelColor`.
    Label,
    /// `NSColor.secondaryLabelColor`.
    SecondaryLabel,
    /// `NSColor.controlAccentColor`.
    ControlAccent,
    /// `NSColor.alternateSelectedControlTextColor`.
    AlternateSelectedControlText,
    /// `NSColor.selectedContentBackgroundColor`.
    SelectedContentBackground,
    /// `NSColor.systemPurple`.
    SystemPurple,
    /// `NSColor.systemRed`.
    SystemRed,
    /// `NSColor.whiteColor`.
    White,
}

fn native(color: AppColor) -> Retained<NSColor> {
    match color {
        AppColor::WindowBackground => NSColor::windowBackgroundColor(),
        AppColor::ControlBackground => NSColor::controlBackgroundColor(),
        AppColor::TertiarySystemFill => NSColor::tertiarySystemFillColor(),
        AppColor::Separator => NSColor::separatorColor(),
        AppColor::Label => NSColor::labelColor(),
        AppColor::SecondaryLabel => NSColor::secondaryLabelColor(),
        AppColor::ControlAccent => NSColor::controlAccentColor(),
        AppColor::AlternateSelectedControlText => NSColor::alternateSelectedControlTextColor(),
        AppColor::SelectedContentBackground => NSColor::selectedContentBackgroundColor(),
        AppColor::SystemPurple => NSColor::systemPurpleColor(),
        AppColor::SystemRed => NSColor::systemRedColor(),
        AppColor::White => NSColor::whiteColor(),
    }
}

/// What `color` draws as under `scheme`.
///
/// The color is resolved under a named appearance rather than the
/// application's current one, so the answer is exact for either scheme even
/// while the user sees the other.
///
/// A dynamic color that declines to resolve in sRGB — a pattern image, for
/// example — answers `None`; semantic colors always resolve.
///
/// # Panics
///
/// Never in practice: `AppKit` ships appearances under both standard names,
/// and the `expect` only covers a platform that does not.
#[must_use]
pub fn resolve(color: AppColor, scheme: ColorScheme) -> Option<Rgba> {
    // SAFETY: these are the platform's two standard appearance names.
    let name = unsafe {
        match scheme {
            ColorScheme::Light => NSAppearanceNameAqua,
            ColorScheme::Dark => NSAppearanceNameDarkAqua,
        }
    };
    let appearance = NSAppearance::appearanceNamed(name)
        .expect("AppKit ships appearances under both standard names");
    let color = native(color);
    let rgba = RefCell::new(None);
    {
        let slot = &rgba;
        let block = block2::RcBlock::new(move || {
            *slot.borrow_mut() = rgba_of(&color);
        });
        appearance.performAsCurrentDrawingAppearance(&block);
    }
    rgba.into_inner()
}

/// `color` converted into the sRGB space and read as components, using the
/// appearance in effect at this moment.
fn rgba_of(color: &NSColor) -> Option<Rgba> {
    let converted = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
    // SAFETY: the component accessors are valid on a color converted to sRGB,
    // and the out pointers point at locals that outlive the call.
    unsafe {
        let mut red = 0.0;
        let mut green = 0.0;
        let mut blue = 0.0;
        let mut alpha = 0.0;
        converted.getRed_green_blue_alpha(
            &raw mut red,
            &raw mut green,
            &raw mut blue,
            &raw mut alpha,
        );
        Some(Rgba::new(red, green, blue, alpha))
    }
}
