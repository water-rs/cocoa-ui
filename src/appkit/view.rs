//! Free functions on a plain `NSView` that the kit's own wrappers do not
//! own — leaves rendered across the seam are bare platform views.
//!
//! # Safety
//!
//! The `unsafe` here calls `AppKit`'s own frame accessors on a view the
//! caller guarantees is alive; both are ordinary main-thread calls.

use objc2_app_kit::NSView;

use crate::geometry::Rect;

/// The view's frame in its superview's coordinate space.
#[must_use]
pub fn frame(view: &NSView) -> Rect {
    view.frame().into()
}

/// The view's bounds in its own coordinate space.
#[must_use]
pub fn bounds(view: &NSView) -> Rect {
    view.bounds().into()
}

/// Sets the view's frame in its superview's coordinate space.
pub fn set_frame(view: &NSView, frame: Rect) {
    view.setFrame(frame.into());
}
