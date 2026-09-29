//! Offscreen test harness: real `AppKit`/`UIKit` objects without a visible
//! window or a running application.
//!
//! Compiled only under `cfg(test)` — nothing here ships. `#[test]` bodies run
//! on the test harness's worker threads, so [`run`] hands them a
//! [`MainThreadMarker`]: the real marker on the main thread, an unchecked one
//! otherwise. That is sound here because a test process never starts
//! `NSApplication`/`UIApplication` — the objects this harness builds are
//! never on screen, never hit-tested and never drawn by the system, and
//! `LOCK` serializes every entry so no two harness users touch `AppKit`
//! state concurrently. A panic in `f` poisons only the test it ran in; the
//! lock is re-entered through `into_inner` so one failing test cannot wedge
//! the rest of the suite.
//!
//! [`attach`] puts a view inside a hidden host that is never ordered in —
//! the smallest environment in which `AppKit`/`UIKit` still run their real
//! `layout`/`layoutSubviews`, display and notification paths — and [`pump`]
//! drains a headless run-loop tick for tests that flush pending work
//! themselves.
//!
//! On `AppKit` the host is a plain `NSView` root rather than an `NSWindow`:
//! `-[NSWindow initWithContentRect:]` throws an `NSInternalInconsistency`
//! exception on any thread that is not `pthread_main`, and `#[test]` bodies
//! never run there. View hierarchies layout and draw identically without a
//! window, so the tests exercise the same code paths.
//!
//! # Safety
//!
//! The single `unsafe` is the unchecked [`MainThreadMarker`]; the module
//! comment above spells out why it is sound in a window-free test process.

use std::sync::{Mutex, MutexGuard};

use objc2::{MainThreadMarker, MainThreadOnly};

/// One harness user at a time: `AppKit`/`UIKit` object state is process-wide
/// and the test harness may run tests on several worker threads at once.
static LOCK: Mutex<()> = Mutex::new(());

/// Takes the harness lock and runs `f` with a [`MainThreadMarker`].
pub fn run<R>(f: impl FnOnce(MainThreadMarker) -> R) -> R {
    let _guard: MutexGuard<'_, ()> = LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: see the module safety note — no application runs in a test
    // process, the objects are never displayed, and LOCK serializes use.
    let mtm =
        MainThreadMarker::new().unwrap_or_else(|| unsafe { MainThreadMarker::new_unchecked() });
    f(mtm)
}

/// Attaches `content` to a hidden host root and returns the host so the
/// caller keeps the hierarchy (and any subview state) alive.
///
/// On `AppKit` the host is a fresh `NSView` — a real `NSWindow` may only be
/// created on the true main thread, which test bodies never occupy. The
/// view-tree layout/display paths the tests exercise behave the same with
/// or without one.
#[cfg(target_os = "macos")]
#[must_use]
pub fn attach(
    mtm: MainThreadMarker,
    content: &crate::PlatformView,
) -> objc2::rc::Retained<objc2_app_kit::NSView> {
    use objc2::{msg_send, rc::Retained};
    // SAFETY: `initWithFrame:` is `NSView`'s plain initializer; `mtm`
    // proves main-thread confinement the way [`run`] arranged.
    let root: Retained<objc2_app_kit::NSView> = unsafe {
        msg_send![
            objc2_app_kit::NSView::alloc(mtm),
            initWithFrame: objc2_core_foundation::CGRect::new(
                objc2_core_foundation::CGPoint::new(0.0, 0.0),
                objc2_core_foundation::CGSize::new(640.0, 480.0),
            )
        ]
    };
    root.addSubview(content);
    root.layoutSubtreeIfNeeded();
    root
}

/// A `UIWindow` with no scene hosting `content`, kept hidden.
///
/// `UIKit` does not need a scene for `layoutSubviews`/`setNeedsDisplay` to
/// run: the window exists so `window`/`safeAreaInsets`-dependent paths see a
/// real window instead of `nil`.
#[cfg(target_os = "ios")]
#[must_use]
pub fn attach(
    mtm: MainThreadMarker,
    content: &crate::PlatformView,
) -> objc2::rc::Retained<objc2_ui_kit::UIWindow> {
    use objc2::{msg_send, rc::Retained};
    // SAFETY: `initWithFrame:` is `UIWindow`'s plain initializer; `mtm`
    // proves main-thread confinement the way [`run`] arranged.
    let window: Retained<objc2_ui_kit::UIWindow> = unsafe {
        msg_send![
            objc2_ui_kit::UIWindow::alloc(mtm),
            initWithFrame: objc2_core_foundation::CGRect::new(
                objc2_core_foundation::CGPoint::new(0.0, 0.0),
                objc2_core_foundation::CGSize::new(390.0, 844.0),
            )
        ]
    };
    window.addSubview(content);
    window
}

/// Runs the current run loop once, draining pending `AppKit`/`UIKit` work —
/// posted notifications, deferred layout, deferred display — without
/// blocking on a window.
pub fn pump() {
    let run_loop = objc2_foundation::NSRunLoop::currentRunLoop();
    let soon = objc2_foundation::NSDate::dateWithTimeIntervalSinceNow(0.05);
    run_loop.runUntilDate(&soon);
}

#[cfg(target_os = "macos")]
#[test]
fn attach_hosts_a_view_offscreen() {
    run(|mtm| {
        let host = crate::appkit::HostView::new(mtm, crate::Rect::new(0.0, 0.0, 320.0, 240.0));
        let root = attach(mtm, &host);
        // SAFETY: `host` is a plain `NSView` subclass — `superview` only
        // walks the responder chain.
        assert!(unsafe { host.superview() }.is_some());
        assert_eq!(root.subviews().len(), 1);
    });
}
