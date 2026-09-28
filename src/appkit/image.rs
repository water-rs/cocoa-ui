//! The `AppKit` image view: an `NSImageView` that also names system symbols.
//!
//! # Safety
//!
//! The `unsafe` here defines an `NSImageView` subclass and calls `objc2`
//! bindings marked unsafe because `AppKit` view, image, and symbol APIs are
//! main-thread only — which the `MainThreadOnly` thread kind and
//! [`MainThreadMarker`] constructor guarantee.

use std::cell::RefCell;
use std::fmt;

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility, NSColor, NSImage, NSImageScaling, NSImageSymbolConfiguration, NSImageView,
};
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSObjectProtocol, NSString};

use crate::geometry::Size;
use crate::image::ScaleMode;

/// The named system symbol an [`ImageView`] shows.
///
/// Kept so a new symbol configuration can rebuild the image the way
/// `imageWithSymbolConfiguration:` needs: `NSImageView` does not re-resolve
/// its image, so the configuration creates a fresh `NSImage` from the name.
pub struct ImageViewIvars {
    symbol_name: RefCell<Option<String>>,
}

impl fmt::Debug for ImageViewIvars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageViewIvars")
            .field("symbol_name", &self.symbol_name.borrow())
            .finish()
    }
}

define_class!(
    // SAFETY: `NSImageView` inherits `initWithFrame:` as its designated
    // initializer, which `ImageView::new` calls, and the class does not
    // implement `Drop`.
    #[unsafe(super(NSImageView))]
    #[name = "CocoaUiImageView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ImageViewIvars]
    #[derive(Debug)]
    /// An `NSImageView` used as an image leaf, with system-symbol helpers.
    pub struct ImageView;

    // SAFETY: `NSObjectProtocol` asks nothing of an `NSImageView` subclass.
    unsafe impl NSObjectProtocol for ImageView {}

    impl ImageView {
        // SAFETY: see the module safety note.
        #[unsafe(method_id(symbolName))]
        fn symbol_name_selector(&self) -> Option<Retained<NSString>> {
            self.ivars()
                .symbol_name
                .borrow()
                .as_deref()
                .map(NSString::from_str)
        }
    }
);

impl ImageView {
    /// An empty image view scaled proportionally.
    #[must_use]
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ImageViewIvars {
            symbol_name: RefCell::new(None),
        });
        // SAFETY: `initWithFrame:` is the inherited designated initializer.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: CGRect::ZERO] };
        this.set_scale_mode(ScaleMode::Fit);
        this
    }

    /// How the image scales inside the view's bounds.
    pub fn set_scale_mode(&self, mode: ScaleMode) {
        let scaling = match mode {
            // `NSImageScaling` has no crop-to-fill mode; proportional scaling
            // is the closest the control offers.
            ScaleMode::Fit | ScaleMode::Fill => NSImageScaling::ScaleProportionallyUpOrDown,
            ScaleMode::Stretch => NSImageScaling::ScaleAxesIndependently,
        };
        self.setImageScaling(scaling);
    }

    /// The image the view draws; `None` clears it.
    pub fn set_image(&self, image: Option<&NSImage>) {
        self.ivars().symbol_name.replace(None);
        self.setImage(image);
        self.invalidate_layout();
    }

    /// The named `SF Symbol` as the view's image.
    ///
    /// Answers `false` when the platform catalog has no symbol of that name;
    /// whether an unknown name is a bug is the caller's call.
    pub fn set_system_symbol(&self, name: &str) -> bool {
        let symbol = NSString::from_str(name);
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&symbol, None);
        image.is_some_and(|image| {
            self.ivars().symbol_name.replace(Some(name.to_owned()));
            self.setImage(Some(&image));
            self.invalidate_layout();
            true
        })
    }

    /// Re-renders the current symbol at `size` points and `weight` on the
    /// `NSFontWeight` scale — what a themed font asks of the symbol.
    ///
    /// Answers `false` when no symbol is set or the symbol rejects the
    /// configuration.
    pub fn set_symbol_configuration(&self, size: f64, weight: f64) -> bool {
        let Some(name) = self.ivars().symbol_name.borrow().clone() else {
            return false;
        };
        let configuration =
            NSImageSymbolConfiguration::configurationWithPointSize_weight(size, weight);
        let symbol = NSString::from_str(&name);
        let configured = NSImage::imageWithSystemSymbolName_accessibilityDescription(&symbol, None)
            .and_then(|base| base.imageWithSymbolConfiguration(&configuration));
        configured.is_some_and(|image| {
            self.setImage(Some(&image));
            self.invalidate_layout();
            true
        })
    }

    /// The color a template-rendered symbol draws in.
    pub fn set_tint_color(&self, color: Option<&NSColor>) {
        self.setContentTintColor(color);
    }

    /// Names the view to a screen reader; `None` leaves it unnamed. Setting a
    /// label marks the view an accessibility element.
    pub fn set_accessibility_label(&self, label: Option<&str>) {
        self.setAccessibilityElement(label.is_some());
        self.setAccessibilityLabel(label.map(NSString::from_str).as_deref());
    }

    /// The image's point size, when one is set — `sizeThatFits`'s intrinsic
    /// answer.
    #[must_use]
    pub fn image_size(&self) -> Option<Size> {
        self.image().map(|image| image.size().into())
    }

    /// The view's intrinsic size changed: invalidate it and walk the
    /// superview chain so every ancestor re-runs layout.
    fn invalidate_layout(&self) {
        self.invalidateIntrinsicContentSize();
        crate::view::invalidate_layout(self);
    }
}
