//! Top-level windows.
//!
//! # Safety
//!
//! The `unsafe` here creates a window, keeps it from releasing itself, and
//! defines its delegate class. A window created in code releases itself when
//! closed unless told otherwise, which would free it under the `Retained` this
//! wrapper owns; [`Window::new`] turns that off before anything else touches
//! the window. The delegate's methods have the signatures `NSWindowDelegate`
//! declares, and `AppKit` sends them on the main thread.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use bitflags::bitflags;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSBackingStoreType, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSString};

use crate::appkit::view_controller::ViewController;
use crate::callback::guarded;
use crate::color::Rgba;
use crate::geometry::{Rect, Size};

bitflags! {
    /// The parts of a window's frame. No flags is a borderless window.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WindowStyle: u16 {
        /// A title bar.
        const TITLED = 1 << 0;
        /// A close button.
        const CLOSABLE = 1 << 1;
        /// A minimize button.
        const MINIATURIZABLE = 1 << 2;
        /// Resizable edges and a zoom button.
        const RESIZABLE = 1 << 3;
        /// The content area extends behind the title bar and toolbars.
        const FULL_SIZE_CONTENT_VIEW = 1 << 4;
    }
}

impl WindowStyle {
    fn native(self) -> NSWindowStyleMask {
        [
            (Self::TITLED, NSWindowStyleMask::Titled),
            (Self::CLOSABLE, NSWindowStyleMask::Closable),
            (Self::MINIATURIZABLE, NSWindowStyleMask::Miniaturizable),
            (Self::RESIZABLE, NSWindowStyleMask::Resizable),
            (
                Self::FULL_SIZE_CONTENT_VIEW,
                NSWindowStyleMask::FullSizeContentView,
            ),
        ]
        .into_iter()
        .filter(|(part, _)| self.contains(*part))
        .fold(NSWindowStyleMask::Borderless, |mask, (_, native)| {
            mask | native
        })
    }
}

/// A top-level window.
///
/// The window lives as long as this value: dropping it closes the window.
pub struct Window {
    window: Retained<NSWindow>,
    delegate: Retained<Delegate>,
}

impl Window {
    /// A hidden window whose content area is `content_rect`, in screen
    /// coordinates, with the frame parts in `style`.
    ///
    /// The window draws into a buffer and creates it immediately. Show it
    /// with [`Window::make_key_and_order_front`].
    #[must_use]
    pub fn new(mtm: MainThreadMarker, content_rect: Rect, style: WindowStyle) -> Self {
        // SAFETY: see the module safety note.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                content_rect.into(),
                style.native(),
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: see the module safety note.
        unsafe { window.setReleasedWhenClosed(false) };
        let delegate = Delegate::new(mtm);
        window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        Self { window, delegate }
    }

    /// Sets the text of the title bar.
    pub fn set_title(&self, title: &str) {
        self.window.setTitle(&NSString::from_str(title));
    }

    /// The window's content area, in screen coordinates: its frame without
    /// the title bar and borders.
    #[must_use]
    pub fn content_rect(&self) -> Rect {
        self.window
            .contentRectForFrameRect(self.window.frame())
            .into()
    }

    /// The window's frame, in screen coordinates.
    #[must_use]
    pub fn frame(&self) -> Rect {
        self.window.frame().into()
    }

    /// Moves and resizes the window to `frame`, in screen coordinates,
    /// redrawing it. `animate` asks the system to tween the change.
    pub fn set_frame(&self, frame: Rect, animate: bool) {
        self.window
            .setFrame_display_animate(frame.into(), true, animate);
    }

    /// Makes `view` fill the content area, replacing the view there.
    pub fn set_content_view(&self, view: &NSView) {
        self.window.setContentView(Some(view));
    }

    /// Hands the window's content to `controller`, whose view then fills the
    /// content area and joins the responder chain as a controller.
    ///
    /// A view hierarchy that contains controller-based components — a split
    /// view most importantly — needs this: `NSSplitViewItem` only reaches the
    /// titlebar when the split view lives under the window's view controller.
    pub fn set_content_view_controller(&self, controller: &ViewController) {
        self.window
            .setContentViewController(Some(controller.native()));
    }

    /// Moves the window to the center of its screen, a little above the
    /// middle.
    pub fn center(&self) {
        self.window.center();
    }

    /// Shows the window in front of the application's other windows and gives
    /// it keyboard focus.
    pub fn make_key_and_order_front(&self) {
        self.window.makeKeyAndOrderFront(None);
    }

    /// Shows the window without giving it keyboard focus.
    pub fn order_front(&self) {
        self.window.orderFront(None);
    }

    /// Whether the window is currently hidden.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.window.isVisible()
    }

    /// The smallest content area the user can resize the window to.
    #[must_use]
    pub fn content_min_size(&self) -> Size {
        self.window.contentMinSize().into()
    }

    /// Sets the smallest content area the user can resize the window to.
    pub fn set_content_min_size(&self, size: Size) {
        self.window.setContentMinSize(size.into());
    }

    /// Sets the largest content area the user can resize the window to.
    pub fn set_content_max_size(&self, size: Size) {
        self.window.setContentMaxSize(size.into());
    }

    /// Calls `handler` every time the window is about to close, replacing any
    /// handler set before.
    pub fn on_close(&self, handler: impl Fn() + 'static) {
        self.delegate.ivars().close.replace(Some(Rc::new(handler)));
    }

    /// Calls `handler` every time the window finishes a resize, live or
    /// otherwise, replacing any handler set before.
    pub fn on_resize(&self, handler: impl Fn() + 'static) {
        self.delegate.ivars().resize.replace(Some(Rc::new(handler)));
    }

    /// Calls `handler` every time the window finishes moving, replacing any
    /// handler set before.
    pub fn on_move(&self, handler: impl Fn() + 'static) {
        self.delegate.ivars().moved.replace(Some(Rc::new(handler)));
    }

    /// Calls `handler` after a live (dragged) resize ends, replacing any
    /// handler set before.
    pub fn on_live_resize_end(&self, handler: impl Fn() + 'static) {
        self.delegate
            .ivars()
            .live_resize_end
            .replace(Some(Rc::new(handler)));
    }

    /// Calls `handler` after the window miniaturizes, replacing any handler
    /// set before.
    pub fn on_miniaturized(&self, handler: impl Fn() + 'static) {
        self.delegate
            .ivars()
            .miniaturized
            .replace(Some(Rc::new(handler)));
    }

    /// Calls `handler` after the window returns from the Dock, replacing any
    /// handler set before.
    pub fn on_deminiaturized(&self, handler: impl Fn() + 'static) {
        self.delegate
            .ivars()
            .deminiaturized
            .replace(Some(Rc::new(handler)));
    }

    /// Calls `handler` after the window enters full screen, replacing any
    /// handler set before.
    pub fn on_entered_fullscreen(&self, handler: impl Fn() + 'static) {
        self.delegate
            .ivars()
            .entered_fullscreen
            .replace(Some(Rc::new(handler)));
    }

    /// Calls `handler` after the window leaves full screen, replacing any
    /// handler set before.
    pub fn on_exited_fullscreen(&self, handler: impl Fn() + 'static) {
        self.delegate
            .ivars()
            .exited_fullscreen
            .replace(Some(Rc::new(handler)));
    }

    /// Whether the window is collapsed into the Dock.
    #[must_use]
    pub fn is_miniaturized(&self) -> bool {
        self.window.isMiniaturized()
    }

    /// Collapses the window into the Dock.
    pub fn miniaturize(&self) {
        self.window.miniaturize(None);
    }

    /// Brings the window back from the Dock.
    pub fn deminiaturize(&self) {
        self.window.deminiaturize(None);
    }

    /// Whether the window is drawing full screen.
    #[must_use]
    pub fn is_fullscreen(&self) -> bool {
        self.window
            .styleMask()
            .contains(NSWindowStyleMask::FullScreen)
    }

    /// Moves the window in or out of full screen.
    pub fn toggle_fullscreen(&self) {
        self.window.toggleFullScreen(None);
    }

    /// Closes the window, calling the [`Window::on_close`] handler first.
    pub fn close(&self) {
        self.window.close();
    }

    /// How transparent the window and everything in it is, `0.0` to `1.0`.
    pub fn set_alpha_value(&self, alpha: f64) {
        self.window.setAlphaValue(alpha);
    }

    /// Whether the window sees the mouse move even when no button is held.
    ///
    /// Views that track hover need this on: `AppKit` only delivers
    /// `mouseMoved:` events to windows that asked for them.
    pub fn set_accepts_mouse_moved_events(&self, accepts: bool) {
        self.window.setAcceptsMouseMovedEvents(accepts);
    }

    /// The color drawn behind the window's content, `None` for the default.
    pub fn set_background_color(&self, color: Rgba) {
        let native = objc2_app_kit::NSColor::colorWithSRGBRed_green_blue_alpha(
            color.red,
            color.green,
            color.blue,
            color.alpha,
        );
        self.window.setBackgroundColor(Some(&native));
    }

    /// Whether the window treats itself as opaque for compositing.
    pub fn set_opaque(&self, opaque: bool) {
        self.window.setOpaque(opaque);
    }

    /// Whether the window draws a shadow.
    pub fn set_has_shadow(&self, has_shadow: bool) {
        self.window.setHasShadow(has_shadow);
    }

    /// Draws every part of the window marked as needing display, now rather
    /// than at the end of this turn of the run loop.
    pub fn display_if_needed(&self) {
        self.window.displayIfNeeded();
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // The close handler belongs to the value being dropped, so it does not
        // hear about this close.
        self.window.setDelegate(None);
        self.window.close();
    }
}

type Handler = Rc<dyn Fn()>;

#[derive(Default)]
struct DelegateIvars {
    close: RefCell<Option<Handler>>,
    resize: RefCell<Option<Handler>>,
    moved: RefCell<Option<Handler>>,
    live_resize_end: RefCell<Option<Handler>>,
    miniaturized: RefCell<Option<Handler>>,
    deminiaturized: RefCell<Option<Handler>>,
    entered_fullscreen: RefCell<Option<Handler>>,
    exited_fullscreen: RefCell<Option<Handler>>,
}

impl DelegateIvars {
    fn fire(slot: &RefCell<Option<Handler>>) {
        // Cloned out of the cell so the handler may replace itself.
        let handler = slot.borrow().clone();
        if let Some(handler) = handler {
            handler();
        }
    }
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements, and the class does
    // not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "CocoaUiWindowDelegate"]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateIvars]
    struct Delegate;

    // SAFETY: `NSObjectProtocol` asks nothing of an `NSObject` subclass.
    unsafe impl NSObjectProtocol for Delegate {}

    // SAFETY: see the module safety note.
    unsafe impl NSWindowDelegate for Delegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            guarded("windowWillClose:", || {
                DelegateIvars::fire(&self.ivars().close);
            });
        }

        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, _notification: &NSNotification) {
            guarded("windowDidResize:", || {
                DelegateIvars::fire(&self.ivars().resize);
            });
        }

        #[unsafe(method(windowDidMove:))]
        fn window_did_move(&self, _notification: &NSNotification) {
            guarded("windowDidMove:", || {
                DelegateIvars::fire(&self.ivars().moved);
            });
        }

        #[unsafe(method(windowDidEndLiveResize:))]
        fn window_did_end_live_resize(&self, _notification: &NSNotification) {
            guarded("windowDidEndLiveResize:", || {
                DelegateIvars::fire(&self.ivars().live_resize_end);
            });
        }

        #[unsafe(method(windowDidMiniaturize:))]
        fn window_did_miniaturize(&self, _notification: &NSNotification) {
            guarded("windowDidMiniaturize:", || {
                DelegateIvars::fire(&self.ivars().miniaturized);
            });
        }

        #[unsafe(method(windowDidDeminiaturize:))]
        fn window_did_deminiaturize(&self, _notification: &NSNotification) {
            guarded("windowDidDeminiaturize:", || {
                DelegateIvars::fire(&self.ivars().deminiaturized);
            });
        }

        #[unsafe(method(windowDidEnterFullScreen:))]
        fn window_did_enter_full_screen(&self, _notification: &NSNotification) {
            guarded("windowDidEnterFullScreen:", || {
                DelegateIvars::fire(&self.ivars().entered_fullscreen);
            });
        }

        #[unsafe(method(windowDidExitFullScreen:))]
        fn window_did_exit_full_screen(&self, _notification: &NSNotification) {
            guarded("windowDidExitFullScreen:", || {
                DelegateIvars::fire(&self.ivars().exited_fullscreen);
            });
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DelegateIvars::default());
        // SAFETY: `init` is `NSObject`'s designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

#[cfg(test)]
mod tests {
    use objc2_app_kit::NSWindowStyleMask;

    use super::WindowStyle;

    #[test]
    fn style_maps_to_its_mask() {
        assert_eq!(
            WindowStyle::all().native(),
            NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::FullSizeContentView
        );
        assert_eq!(
            (WindowStyle::TITLED | WindowStyle::RESIZABLE).native(),
            NSWindowStyleMask::Titled | NSWindowStyleMask::Resizable
        );
        assert_eq!(WindowStyle::empty().native(), NSWindowStyleMask::Borderless);
    }
}
