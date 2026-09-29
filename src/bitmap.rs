//! Pixel capture helpers: RGBA8 bitmap contexts, `CGImage` construction, and
//! the offscreen window an offscreen render attaches its tree to.
//!
//! # Safety
//!
//! `CGBitmapContextCreate` has no `objc2` binding, so the one `extern`
//! declaration lives here, discharged next to the call. Everything else is
//! safe `objc2` API on the caller's own objects, all main-thread.

use objc2::rc::Retained;
#[cfg(target_os = "ios")]
use objc2::runtime::NSObjectProtocol;
use objc2::{AllocAnyThread, MainThreadOnly};
use objc2_core_foundation::{CFData, CFRetained};
use objc2_core_graphics::{
    CGBitmapContextCreateImage, CGColorRenderingIntent, CGColorSpace, CGContext, CGDataProvider,
    CGImage, CGImageAlphaInfo,
};

use crate::geometry::{Rect, Size};

unsafe extern "C" {
    /// `CGBitmapContextCreate` — `objc2-core-graphics` has no binding for it.
    fn CGBitmapContextCreate(
        data: *mut core::ffi::c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: Option<&CGColorSpace>,
        bitmap_info: u32,
    ) -> *mut CGContext;
}

const BITS_PER_COMPONENT: usize = 8;
const BITS_PER_PIXEL: usize = 32;
const BYTES_PER_PIXEL: usize = 4;

/// A `CGContext` drawing premultiplied RGBA8 into `pixels`' storage.
///
/// The context holds a raw pointer into `pixels`' buffer, so callers must not
/// reallocate the buffer until the context is dropped.
///
/// # Panics
///
/// When `pixels` is not exactly `width*height*4` bytes.
#[must_use]
pub fn bitmap_context(
    pixels: &mut [u8],
    width: usize,
    height: usize,
) -> Option<CFRetained<CGContext>> {
    assert_eq!(
        pixels.len(),
        width * height * BYTES_PER_PIXEL,
        "a bitmap context needs exactly width*height*4 bytes"
    );
    // SAFETY: `pixels` is a live `width*height*4` buffer kept alive by the
    // caller for the context's use; the RGB colorspace is a fresh system
    // object.
    let ptr = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            width,
            height,
            BITS_PER_COMPONENT,
            width * BYTES_PER_PIXEL,
            CGColorSpace::new_device_rgb().as_deref(),
            CGImageAlphaInfo::PremultipliedLast.0,
        )
    };
    // SAFETY: CoreGraphics returns a +1 context or null.
    Some(unsafe { CFRetained::from_raw(core::ptr::NonNull::new(ptr)?) })
}

/// A `CGImage` holding a copy of `pixels`' premultiplied RGBA8 content.
///
/// # Panics
///
/// When `pixels` is not exactly `width*height*4` bytes.
#[must_use]
pub fn image_from_rgba(pixels: &[u8], width: usize, height: usize) -> Option<CFRetained<CGImage>> {
    assert_eq!(
        pixels.len(),
        width * height * BYTES_PER_PIXEL,
        "an RGBA8 image needs exactly width*height*4 bytes"
    );
    let data = CFData::from_bytes(pixels);
    let provider = CGDataProvider::with_cf_data(Some(&data))?;
    // SAFETY: `decode` is null; every other argument describes the
    // premultiplied RGBA8 layout `pixels` carries.
    unsafe {
        CGImage::new(
            width,
            height,
            BITS_PER_COMPONENT,
            BITS_PER_PIXEL,
            width * BYTES_PER_PIXEL,
            CGColorSpace::new_device_rgb().as_deref(),
            objc2_core_graphics::CGBitmapInfo(CGImageAlphaInfo::PremultipliedLast.0),
            Some(&provider),
            core::ptr::null(),
            true,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
}

/// Renders `layer`'s committed tree into `context`.
pub fn render_layer(layer: &objc2_quartz_core::CALayer, context: &CGContext) {
    layer.renderInContext(context);
}

/// Draws `image` covering `rect` inside `context`.
pub fn draw_image(context: &CGContext, image: &CGImage, rect: Rect) {
    CGContext::draw_image(Some(context), rect.into(), Some(image));
}

/// The image `context` captured — `CGBitmapContextCreateImage`.
#[must_use]
pub fn context_image(context: &CGContext) -> Option<CFRetained<CGImage>> {
    CGBitmapContextCreateImage(Some(context))
}

/// Scales `context` so one user-space unit is `scale` device pixels.
pub fn scale_to_pixels(context: &CGContext, scale: f64) {
    CGContext::scale_ctm(Some(context), scale, scale);
}

/// Pushes `context`'s flipped layer-render transform — `UIKit`'s layer tree
/// renders top-down into a bottom-up `CG` coordinate space.
#[cfg(target_os = "ios")]
pub fn begin_layer_flip(context: &CGContext, height: f64) {
    CGContext::save_g_state(Some(context));
    CGContext::translate_ctm(Some(context), 0.0, height);
    CGContext::scale_ctm(Some(context), 1.0, -1.0);
}

/// Pops the transform [`begin_layer_flip`] pushed.
#[cfg(target_os = "ios")]
pub fn end_layer_flip(context: &CGContext) {
    CGContext::restore_g_state(Some(context));
}

/// Fills `context`'s whole `rect` with `color`.
pub fn fill(context: &CGContext, color: &objc2_core_graphics::CGColor, rect: Rect) {
    CGContext::set_fill_color_with_color(Some(context), Some(color));
    CGContext::fill_rect(Some(context), rect.into());
}

/// Pushes `context` as `UIKit`'s current graphics context for the layer
/// render, then pops it.
#[cfg(target_os = "ios")]
pub fn with_uikit_context(context: &CGContext, body: impl FnOnce()) {
    // SAFETY: balances the `UIGraphicsPopContext` below on the same thread,
    // the documented pairing for the UIKit context stack.
    unsafe { objc2_ui_kit::UIGraphicsPushContext(context) };
    body();
    // SAFETY: pops the context pushed above.
    unsafe { objc2_ui_kit::UIGraphicsPopContext() };
}

/// A `UIImage` rendering `image` at `scale`, tagged `alwaysTemplate`.
#[cfg(target_os = "ios")]
#[must_use]
pub fn template_image(image: &CGImage, scale: f64) -> Option<Retained<objc2_ui_kit::UIImage>> {
    use objc2_ui_kit::{UIImage, UIImageOrientation, UIImageRenderingMode};
    let ui = UIImage::initWithCGImage_scale_orientation(
        UIImage::alloc(),
        image,
        scale,
        UIImageOrientation::Up,
    );
    Some(ui.imageWithRenderingMode(UIImageRenderingMode::AlwaysTemplate))
}

/// An `NSImage` wrapping `image` at `size` points, tagged as a template.
#[cfg(target_os = "macos")]
#[must_use]
pub fn template_image(image: &CGImage, size: Size) -> Retained<objc2_app_kit::NSImage> {
    let ns = objc2_app_kit::NSImage::initWithCGImage_size(
        objc2_app_kit::NSImage::alloc(),
        image,
        size.into(),
    );
    ns.setTemplate(true);
    ns
}

/// A borderless offscreen `NSWindow` at `(-10_000, -10_000)` for headless
/// capture — it never appears, but attaching a view to it drives `AppKit`'s
/// window-dependent rendering paths.
#[cfg(target_os = "macos")]
#[must_use]
pub fn make_offscreen_window(
    mtm: objc2::MainThreadMarker,
    size: Size,
) -> Retained<objc2_app_kit::NSWindow> {
    use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
    // SAFETY: `initWithContentRect:styleMask:backing:defer:` is `NSWindow`'s
    // designated initializer; `setReleasedWhenClosed:false` keeps the object
    // owned by the `Retained` the caller drops.
    unsafe {
        let window: Retained<NSWindow> = NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            Rect::new(-10_000.0, -10_000.0, size.width, size.height).into(),
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        );
        window.setReleasedWhenClosed(false);
        window
    }
}

/// An offscreen `UIWindow` in `scene` — hidden off-display but attached so
/// `UIView` captures see a real window. Its safe-area insets are zeroed:
/// capture windows never sit under a device notch.
#[cfg(target_os = "ios")]
#[must_use]
pub fn make_offscreen_window(
    mtm: objc2::MainThreadMarker,
    scene: &objc2_ui_kit::UIWindowScene,
    size: Size,
) -> Retained<CaptureWindow> {
    // SAFETY: `initWithWindowScene:` is a designated initializer.
    let window: Retained<CaptureWindow> =
        unsafe { objc2::msg_send![CaptureWindow::alloc(mtm), initWithWindowScene: scene] };
    window.setFrame(Rect::new(-10_000.0, -10_000.0, size.width, size.height).into());
    window
}

#[cfg(target_os = "ios")]
objc2::define_class!(
    // SAFETY: `UIWindow` asks a subclass to initialize through a designated
    // initializer, which `make_offscreen_window` does, and the class holds no
    // ivars and implements no `Drop`.
    #[unsafe(super(objc2_ui_kit::UIWindow))]
    #[name = "CocoaUiCaptureWindow"]
    #[thread_kind = MainThreadOnly]
    /// A capture window reporting zero safe-area insets: it never appears on
    /// a display, so no device notch or home-indicator inset applies.
    pub struct CaptureWindow;

    unsafe impl NSObjectProtocol for CaptureWindow {}

    impl CaptureWindow {
        /// Zero insets — the window is never on a device.
        #[unsafe(method(safeAreaInsets))]
        fn safe_area_insets(&self) -> objc2_ui_kit::UIEdgeInsets {
            objc2_ui_kit::UIEdgeInsets { top: 0.0, left: 0.0, bottom: 0.0, right: 0.0 }
        }
    }
);

/// Recursively forces every `NSTextField` inside `view` to draw — `AppKit`
/// leaves text-field contents out of `cacheDisplay` unless they are marked
/// dirty first.
#[cfg(target_os = "macos")]
pub fn force_text_fields_display(view: &crate::PlatformView) {
    use objc2_app_kit::NSTextField;
    if let Ok(text) = objc2::rc::Retained::downcast::<NSTextField>(crate::view::retain_base(view)) {
        text.setNeedsDisplayInRect(text.bounds());
        text.displayIfNeeded();
    }
    for subview in crate::view::subviews(view) {
        force_text_fields_display(&subview);
    }
}

/// Sends a macOS capture window front without activating the app — the
/// ordering `orderFrontRegardless` implies.
#[cfg(target_os = "macos")]
pub fn show_capture_window(window: &objc2_app_kit::NSWindow) {
    window.orderFrontRegardless();
    window.display();
}

/// Closes a capture window after the bitmap lands.
#[cfg(target_os = "macos")]
pub fn close_capture_window(window: &objc2_app_kit::NSWindow) {
    window.orderOut(None);
}
