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

/// A timing curve [`animate_with`] plays `body`'s animatable changes under.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timing {
    /// A cubic-bezier timing curve over `duration` seconds.
    Bezier {
        /// Seconds the animation runs.
        duration: f64,
        /// The curve's two control points, `[x1, y1, x2, y2]`.
        control_points: [f32; 4],
    },
    /// A physically driven spring.
    Spring {
        /// Spring stiffness.
        stiffness: f64,
        /// Spring damping.
        damping: f64,
    },
}

/// Runs `body` while `timing` plays its animatable changes: a
/// `UIViewPropertyAnimator` on iOS, an `NSAnimationContext` group on macOS.
#[cfg(target_os = "ios")]
pub fn animate_with(timing: Timing, body: impl FnOnce() + 'static) {
    use objc2::MainThreadOnly;
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_core_foundation::{CGPoint, CGVector};
    use objc2_ui_kit::{
        UICubicTimingParameters, UISpringTimingParameters, UITimingCurveProvider, UIViewAnimating,
        UIViewPropertyAnimator,
    };
    use std::cell::RefCell;

    let mtm = objc2::MainThreadMarker::new().expect("animation runs on the main thread");
    // `addAnimations` may evaluate its block once only; the option still
    // guards a double evaluation.
    let body = RefCell::new(Some(body));
    let block = block2::RcBlock::new(move || {
        if let Some(body) = body.borrow_mut().take() {
            body();
        }
    });
    let (duration, parameters): (f64, Retained<ProtocolObject<dyn UITimingCurveProvider>>) =
        match timing {
            Timing::Bezier {
                duration,
                control_points: [x1, y1, x2, y2],
            } => (
                duration,
                ProtocolObject::from_retained(
                    UICubicTimingParameters::initWithControlPoint1_controlPoint2(
                        UICubicTimingParameters::alloc(mtm),
                        CGPoint::new(f64::from(x1), f64::from(y1)),
                        CGPoint::new(f64::from(x2), f64::from(y2)),
                    ),
                ),
            ),
            Timing::Spring { stiffness, damping } => (
                0.0,
                ProtocolObject::from_retained(
                    UISpringTimingParameters::initWithMass_stiffness_damping_initialVelocity(
                        UISpringTimingParameters::alloc(mtm),
                        1.0,
                        stiffness,
                        damping,
                        CGVector::new(0.0, 0.0),
                    ),
                ),
            ),
        };
    let animator = UIViewPropertyAnimator::initWithDuration_timingParameters(
        UIViewPropertyAnimator::alloc(mtm),
        duration,
        &parameters,
    );
    animator.addAnimations(&block);
    animator.startAnimation();
}

/// Runs `body` while `timing` plays its animatable changes.
///
/// A spring has no authored duration in `UIKit`; `NSAnimationContext` wants
/// one, so it is estimated from the stiffness and damping and clamped the
/// way the framework consumer's spring timing expects.
#[cfg(target_os = "macos")]
pub fn animate_with(timing: Timing, body: impl FnOnce() + 'static) {
    match timing {
        Timing::Bezier {
            duration,
            control_points,
        } => animate(duration, Some(control_points), body),
        Timing::Spring { stiffness, damping } => {
            let estimated = 2.0 * (1.0 / stiffness).sqrt() * damping;
            animate(estimated.clamp(0.1, 2.0), None, body);
        }
    }
}
