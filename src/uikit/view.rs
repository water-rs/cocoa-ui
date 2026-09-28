//! Free functions on a plain `UIView` that the kit's own wrappers do not
//! own — leaves rendered across the seam are bare platform views.
//!
//! # Safety
//!
//! The `unsafe` here calls `UIKit`'s own frame accessors on a view the
//! caller guarantees is alive; both are ordinary main-thread calls.

use objc2_ui_kit::UIView;

use crate::geometry::Rect;

/// The view's frame in its superview's coordinate space.
#[must_use]
pub fn frame(view: &UIView) -> Rect {
    view.frame().into()
}

/// The view's bounds in its own coordinate space.
#[must_use]
pub fn bounds(view: &UIView) -> Rect {
    view.bounds().into()
}

/// Sets the view's frame in its superview's coordinate space.
pub fn set_frame(view: &UIView, frame: Rect) {
    view.setFrame(frame.into());
}

/// Reports that the view's content size changed: invalidates its intrinsic
/// size and walks the superview chain so every ancestor re-runs layout.
pub fn invalidate_layout_hierarchy(view: &UIView) {
    view.invalidateIntrinsicContentSize();
    view.setNeedsLayout();
    let mut parent = view.superview();
    while let Some(current) = parent {
        current.invalidateIntrinsicContentSize();
        current.setNeedsLayout();
        parent = current.superview();
    }
}

/// Whether the view lays out right-to-left for its current content and
/// trait environment.
#[must_use]
pub fn is_right_to_left(view: &UIView) -> bool {
    use objc2_ui_kit::UIUserInterfaceLayoutDirection;
    UIView::userInterfaceLayoutDirectionForSemanticContentAttribute(
        view.semanticContentAttribute(),
        objc2::MainThreadMarker::from(view),
    ) == UIUserInterfaceLayoutDirection::RightToLeft
}
