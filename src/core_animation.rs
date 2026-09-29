//! Core Animation, the compositor both frameworks draw through.

use objc2_quartz_core::CATransaction;

/// Commits the current Core Animation transaction to the render server now,
/// rather than at the end of this turn of the run loop.
///
/// After this returns, every layer change made so far has been handed to the
/// compositor, which is the point at which a frame can be said to have been
/// submitted.
pub fn flush_transaction() {
    CATransaction::flush();
}

/// Runs `body` inside an `NSAnimationContext` group.
///
/// Animatable property changes `body` makes animate over `duration` seconds
/// and, when `control_points` is given, the cubic timing function they
/// describe.
#[cfg(target_os = "macos")]
pub fn animate(duration: f64, control_points: Option<[f32; 4]>, body: impl FnOnce() + 'static) {
    use std::cell::RefCell;
    use std::ptr::NonNull;

    use objc2_app_kit::NSAnimationContext;
    use objc2_quartz_core::CAMediaTimingFunction;

    // The changes block is `Fn`, and `NSAnimationContext` may evaluate it
    // once only; the option still guards a double evaluation.
    let body = RefCell::new(Some(body));
    let block = block2::RcBlock::new(move |context: NonNull<NSAnimationContext>| {
        // SAFETY: the group passes a live context for the block's duration.
        let context = unsafe { context.as_ref() };
        context.setDuration(duration);
        context.setAllowsImplicitAnimation(true);
        if let Some([x1, y1, x2, y2]) = control_points {
            let timing = CAMediaTimingFunction::functionWithControlPoints(x1, y1, x2, y2);
            context.setTimingFunction(Some(&timing));
        }
        if let Some(body) = body.borrow_mut().take() {
            body();
        }
    });
    NSAnimationContext::runAnimationGroup(&block);
}

/// Runs `body` while a cross-fade of `duration` seconds plays on `view`'s
/// layer: the view's new content dissolves in over the old.
#[cfg(target_os = "macos")]
pub fn cross_dissolve(view: &objc2_app_kit::NSView, duration: f64, body: impl FnOnce()) {
    use objc2_foundation::ns_string;
    use objc2_quartz_core::{CAMediaTiming, CATransition, kCATransitionFade};

    view.setWantsLayer(true);
    if let Some(layer) = view.layer() {
        let transition = CATransition::new();
        // SAFETY: `kCATransitionFade` is a `CATransitionType` constant Core
        // Animation exports.
        transition.setType(unsafe { kCATransitionFade });
        transition.setDuration(duration);
        layer.addAnimation_forKey(&transition, Some(ns_string!("crossDissolve")));
    }
    body();
}

/// Runs `body` while a cross-fade of `duration` seconds plays on `view`: the
/// view's new content dissolves in over the old.
#[cfg(target_os = "ios")]
pub fn cross_dissolve(view: &objc2_ui_kit::UIView, duration: f64, body: impl FnOnce() + 'static) {
    use objc2_ui_kit::{UIView, UIViewAnimationOptions};

    // `UIView.transition` may evaluate its animations block more than once;
    // the body still runs a single time.
    let body = std::cell::RefCell::new(Some(body));
    let block = block2::RcBlock::new(move || {
        if let Some(body) = body.borrow_mut().take() {
            body();
        }
    });
    UIView::transitionWithView_duration_options_animations_completion(
        view,
        duration,
        UIViewAnimationOptions::TransitionCrossDissolve,
        Some(&block),
        None,
    );
}
