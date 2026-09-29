//! `AppKit`, the user-interface framework of macOS.
//!
//! A macOS application is one [`Application`] run with
//! [`ApplicationHandlers`]: when it finishes launching, the handler creates
//! [`Window`]s and puts a [`HostView`] in each. The main menu is a [`Menu`] of
//! [`MenuItem`]s whose [`MenuAction`]s travel the responder chain.
//!
//! ```no_run
//! use std::cell::RefCell;
//! use std::rc::Rc;
//!
//! use cocoa_ui::appkit::{
//!     Application, ApplicationHandlers, HostView, Window, WindowStyle,
//! };
//! use cocoa_ui::{MainThreadMarker, Rect};
//!
//! let mtm = MainThreadMarker::new().expect("main runs on the main thread");
//! let windows = Rc::new(RefCell::new(Vec::new()));
//! let launched = Rc::clone(&windows);
//! Application::shared(mtm).run(
//!     ApplicationHandlers::new()
//!         .did_finish_launching(move |mtm| {
//!             let window = Window::new(mtm, Rect::new(0.0, 0.0, 800.0, 600.0), WindowStyle::all());
//!             window.set_title("Hello");
//!             window.set_content_view(&HostView::new(mtm, window.content_rect()));
//!             window.center();
//!             window.make_key_and_order_front();
//!             launched.borrow_mut().push(window);
//!         })
//!         .should_terminate_after_last_window_closed(|_| true),
//! );
//! ```

mod appearance;
mod application;
pub mod button;
pub mod colors;
mod host_view;
pub mod image;
mod menu;
mod view_controller;
mod window;

pub use appearance::ColorSchemeObservation;
pub use application::{ActivationPolicy, Application, ApplicationHandlers};
pub use button::Button;
pub use colors::AppColor;
pub use host_view::{HitTest, HostView};
pub use image::ImageView;
pub mod label;
pub mod toggle;
pub use label::Label;
mod slider;
pub use menu::{KeyModifiers, Menu, MenuAction, MenuItem};
pub use slider::Slider;
mod progress;
pub use progress::Progress;
mod stepper;
pub use stepper::Stepper;
pub mod text_field;
pub use text_field::{SecureField, TextField};
pub use view_controller::ViewController;
pub use window::{Window, WindowStyle};
