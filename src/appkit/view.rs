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

/// Reports that the view's content size changed: invalidates its intrinsic
/// size and walks the superview chain so every ancestor re-runs layout.
pub fn invalidate_layout_hierarchy(view: &NSView) {
    view.invalidateIntrinsicContentSize();
    view.setNeedsLayout(true);
    // SAFETY: superview walking is a main-thread read of the view hierarchy.
    let mut parent = unsafe { view.superview() };
    while let Some(current) = parent {
        current.invalidateIntrinsicContentSize();
        current.setNeedsLayout(true);
        // SAFETY: same walk, one level up.
        parent = unsafe { current.superview() };
    }
}

/// Whether the view lays out right-to-left for its current content.
#[must_use]
pub fn is_right_to_left(view: &NSView) -> bool {
    use objc2_app_kit::NSUserInterfaceLayoutDirection;
    view.userInterfaceLayoutDirection() == NSUserInterfaceLayoutDirection::RightToLeft
}
