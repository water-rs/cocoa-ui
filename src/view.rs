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
use objc2_foundation::NSObjectProtocol;

use crate::PlatformView;
use crate::geometry::{Point, Rect};

/// The view's immediate subviews, in back-to-front order.
#[must_use]
pub fn subviews(view: &PlatformView) -> Vec<Retained<PlatformView>> {
    view.subviews().to_vec()
}

/// The view's Objective-C class name.
#[must_use]
pub fn class_name(view: &PlatformView) -> &'static str {
    view.class().name().to_str().unwrap_or_default()
}

/// Converts `point` from `view`'s coordinate space into `to`'s.
#[must_use]
pub fn convert_point(view: &PlatformView, point: Point, to: &PlatformView) -> Point {
    view.convertPoint_toView(point.into(), Some(to)).into()
}

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

/// Whether `view` is inside a window's hierarchy right now.
#[must_use]
pub fn has_window(view: &PlatformView) -> bool {
    view.window().is_some()
}

/// `view`'s bounds in its window's coordinate space.
///
/// On macOS the result keeps `AppKit`'s bottom-left origin; a top-left
/// consumer mirrors it against the content area's height.
#[must_use]
pub fn bounds_in_window(view: &PlatformView) -> Rect {
    view.convertRect_toView(view.bounds(), None).into()
}

/// Lets `view` resize with its superview on both axes.
///
/// The autoresizing mask `UIViewAutoresizing.flexibleWidth |
/// .flexibleHeight`, the same flag pair `NSView` spells `ViewWidthSizable |
/// ViewHeightSizable`.
pub fn set_autoresizing_flexible_size(view: &PlatformView) {
    #[cfg(target_os = "macos")]
    view.setAutoresizingMask(
        objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
            | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    #[cfg(target_os = "ios")]
    view.setAutoresizingMask(
        objc2_ui_kit::UIViewAutoresizing::FlexibleWidth
            | objc2_ui_kit::UIViewAutoresizing::FlexibleHeight,
    );
}

/// Whether `view` posts `NSViewFrameDidChangeNotification` on every frame
/// change — off by default, so a caller watching the notification opts in.
#[cfg(target_os = "macos")]
pub fn set_posts_frame_changed(view: &PlatformView, enabled: bool) {
    view.setPostsFrameChangedNotifications(enabled);
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

/// Removes `view` and everything inside it from the accessibility tree.
#[cfg(target_os = "macos")]
pub fn hide_from_accessibility(view: &PlatformView) {
    use objc2_app_kit::NSAccessibility;
    use objc2_foundation::NSArray;
    view.setAccessibilityElement(false);
    // SAFETY: installing an empty children array on a live view is the
    // documented way to strip its accessibility subtree.
    unsafe { view.setAccessibilityChildren(Some(&NSArray::new())) };
}

/// Removes `view` and everything inside it from the accessibility tree.
#[cfg(target_os = "ios")]
pub fn hide_from_accessibility(view: &PlatformView) {
    use objc2_ui_kit::NSObjectUIAccessibility;
    view.setAccessibilityElementsHidden(true, objc2::MainThreadMarker::from(view));
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

/// Whether `view` clips its subviews to its bounds — `UIView`'s
/// `clipsToBounds`, a `UIKit`-only property.
#[cfg(target_os = "ios")]
pub fn set_clips_to_bounds(view: &PlatformView, clips: bool) {
    view.setClipsToBounds(clips);
}

/// The primary content `view` exposes through `cocoaUiPrimaryContent`.
///
/// The child a kit host view surfaces for chrome like list cells and scroll
/// surfaces; `None` when `view` does not answer the selector.
#[must_use]
pub fn primary_content(view: &PlatformView) -> Option<Retained<PlatformView>> {
    if view.respondsToSelector(objc2::sel!(cocoaUiPrimaryContent)) {
        // SAFETY: every kit class implementing `cocoaUiPrimaryContent`
        // declares it `-> Option<Retained<PlatformView>>`.
        unsafe { objc2::msg_send![view, cocoaUiPrimaryContent] }
    } else {
        None
    }
}

/// Whether `view` sizes itself by its frame rather than by Auto Layout
/// constraints — `true` for views a layout container positions manually.
pub fn set_translates_autoresizing(view: &PlatformView, enabled: bool) {
    view.setTranslatesAutoresizingMaskIntoConstraints(enabled);
}

/// Makes `parent`'s subviews exactly `ordered`, in that z-order, reusing the
/// subview instances already attached.
#[cfg(target_os = "macos")]
pub fn reconcile_subviews(parent: &PlatformView, ordered: &[Retained<PlatformView>]) {
    use objc2_foundation::NSArray;
    parent.setSubviews(&NSArray::from_retained_slice(ordered));
}

/// Makes `parent`'s subviews exactly `ordered`, in that z-order, reusing the
/// subview instances already attached.
#[cfg(target_os = "ios")]
pub fn reconcile_subviews(parent: &PlatformView, ordered: &[Retained<PlatformView>]) {
    use std::ptr;

    for subview in parent.subviews().to_vec() {
        if !ordered
            .iter()
            .any(|wanted| ptr::eq(&raw const **wanted, &raw const *subview))
        {
            subview.removeFromSuperview();
        }
    }
    for (index, child) in ordered.iter().enumerate() {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "a view hierarchy never reaches `NSInteger::MAX` subviews"
        )]
        parent.insertSubview_atIndex(child, index as isize);
    }
}
