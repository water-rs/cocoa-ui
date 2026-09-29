//! The platform's base view and the free functions on it.
//!
//! `NSView` and `UIView` are never compiled together, so the base is an
//! alias, not a trait: code written against [`PlatformView`] has no `#[cfg]`.
//!
//! # Safety
//!
//! The `unsafe` here calls `AppKit`/`UIKit`'s own hierarchy and frame
//! accessors on views the caller guarantees are alive; all are ordinary
//! main-thread calls.

use objc2::rc::Retained;

use crate::PlatformView;
use crate::geometry::Rect;

/// A +1 reference to `view` as its base class: any `NSView`/`UIView`
/// subclass, including the kit's own classes.
///
/// The caller keeps its own object alive; this retains the base view.
///
/// # Panics
///
/// `view` retains to a non-null object; unreachable for a live view.
#[must_use]
pub fn retain_base<V: AsRef<PlatformView> + ?Sized>(view: &V) -> Retained<PlatformView> {
    // SAFETY: `view.as_ref()` is a live `PlatformView` for the call; the
    // returned `Retained` owns the new +1.
    unsafe { Retained::retain(std::ptr::from_ref(view.as_ref()).cast_mut()) }
        .expect("a live view cannot retain to null")
}

/// The frame in the superview's coordinate space.
#[must_use]
pub fn frame(view: &PlatformView) -> Rect {
    view.frame().into()
}

/// The bounds in the view's own coordinate space.
#[must_use]
pub fn bounds(view: &PlatformView) -> Rect {
    view.bounds().into()
}

/// Moves and resizes `view` in its superview's coordinate space.
pub fn set_frame(view: &PlatformView, frame: Rect) {
    view.setFrame(frame.into());
}

/// Adds `child` above `parent`'s existing subviews.
pub fn add_subview(parent: &PlatformView, child: &PlatformView) {
    parent.addSubview(child);
}

/// Detaches `view` from its superview; does nothing when it has none.
pub fn remove_from_superview(view: &PlatformView) {
    view.removeFromSuperview();
}

/// Shows or hides `view` without detaching it.
pub fn set_hidden(view: &PlatformView, hidden: bool) {
    view.setHidden(hidden);
}

/// Tells `view` and every ancestor that its size may have changed.
///
/// Invalidates intrinsic content size and marks each for layout. Call from a
/// watcher after any imperative change that can alter what the view measures
/// to — a text change, a font change, a swapped child.
pub fn invalidate_layout(view: &PlatformView) {
    view.invalidateIntrinsicContentSize();
    set_needs_layout(view);
    let mut parent = superview(view);
    while let Some(current) = parent {
        current.invalidateIntrinsicContentSize();
        set_needs_layout(&current);
        parent = superview(&current);
    }
}

#[cfg(target_os = "macos")]
fn set_needs_layout(view: &PlatformView) {
    view.setNeedsLayout(true);
}

#[cfg(target_os = "ios")]
fn set_needs_layout(view: &PlatformView) {
    view.setNeedsLayout();
}

#[cfg(target_os = "macos")]
fn superview(view: &PlatformView) -> Option<Retained<PlatformView>> {
    // SAFETY: superview walking is a main-thread read of the view hierarchy.
    unsafe { view.superview() }
}

#[cfg(target_os = "ios")]
fn superview(view: &PlatformView) -> Option<Retained<PlatformView>> {
    view.superview()
}

/// Whether the view lays out right-to-left for its current content.
#[must_use]
#[cfg(target_os = "macos")]
pub fn is_right_to_left(view: &PlatformView) -> bool {
    use objc2_app_kit::NSUserInterfaceLayoutDirection;
    view.userInterfaceLayoutDirection() == NSUserInterfaceLayoutDirection::RightToLeft
}

/// Whether the view lays out right-to-left for its current content and
/// trait environment.
#[must_use]
#[cfg(target_os = "ios")]
pub fn is_right_to_left(view: &PlatformView) -> bool {
    use objc2_ui_kit::UIUserInterfaceLayoutDirection;
    PlatformView::userInterfaceLayoutDirectionForSemanticContentAttribute(
        view.semanticContentAttribute(),
        objc2::MainThreadMarker::from(view),
    ) == UIUserInterfaceLayoutDirection::RightToLeft
}

/// Fades `view` — and everything it contains — toward transparent, `1.0`
/// being fully opaque.
pub fn set_alpha(view: &PlatformView, alpha: f64) {
    #[cfg(target_os = "macos")]
    view.setAlphaValue(alpha);
    #[cfg(target_os = "ios")]
    view.setAlpha(alpha);
}

/// Declares `view` an accessibility element and gives it `label` (and, on
/// `AppKit`, a tooltip of the same text). Pass text already stripped of
/// bidirectional controls.
pub fn set_accessibility_label(view: &PlatformView, label: &str) {
    let label = objc2_foundation::NSString::from_str(label);
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSAccessibility;
        view.setAccessibilityElement(true);
        view.setAccessibilityLabel(Some(&label));
        view.setToolTip(Some(&label));
    }
    #[cfg(target_os = "ios")]
    {
        use objc2_ui_kit::NSObjectUIAccessibility;
        let mtm = objc2::MainThreadMarker::from(view);
        view.setIsAccessibilityElement(true, mtm);
        view.setAccessibilityLabel(Some(&label), mtm);
    }
}

/// Removes `view` and its whole subtree from the accessibility hierarchy:
/// the chrome's own accessible element (say a button) then stands in for it.
pub fn hide_from_accessibility(view: &PlatformView) {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSAccessibility;
        view.setAccessibilityElement(false);
        // SAFETY: an ordinary main-thread `AppKit` accessibility setter;
        // marked unsafe in the bindings.
        unsafe { view.setAccessibilityChildren(Some(&objc2_foundation::NSArray::new())) };
    }
    #[cfg(target_os = "ios")]
    {
        use objc2_ui_kit::NSObjectUIAccessibility;
        view.setAccessibilityElementsHidden(true, objc2::MainThreadMarker::from(view));
    }
}
