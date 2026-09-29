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

mod animate;
mod appearance;
mod application;
mod badge;
pub mod button;
mod color_view;
mod color_well;
pub mod colors;
mod control;
mod date_picker;
mod effect_view;
mod host_view;
pub mod image;
mod menu;
mod popup_button;
mod search_field;
mod segmented;
mod source_list;
mod split;
mod toolbar;
mod view_controller;
mod window;

pub use animate::{run_animation, set_animated_alpha};
pub use appearance::ColorSchemeObservation;
pub use application::{ActivationPolicy, Application, ApplicationHandlers};
pub use badge::BadgeView;
pub use button::Button;
pub use color_view::ColorView;
pub use color_well::ColorWell;
pub use colors::AppColor;
pub use control::{activate, first_control};
pub use date_picker::{DatePicker, DatePickerElements};
pub use effect_view::{header_material_view, set_material_background};
pub use host_view::{HitTest, HostView};
pub use image::{ImageView, symbol_image};
pub mod label;
mod picker;
pub use picker::Picker;
pub mod toggle;
pub use label::Label;
mod slider;
pub use crate::menu::{Command, KeyModifiers, MenuTreeNode};
pub use menu::{Menu, MenuAction, MenuItem};
pub use slider::Slider;
mod progress;
pub use progress::Progress;
mod scroll;
pub use scroll::ScrollView;
mod stepper;
pub use stepper::Stepper;
pub mod text_field;
pub use popup_button::PopUpButton;
pub use search_field::SearchField;
pub use segmented::{Segment, SegmentedControl};
pub use source_list::SourceList;
pub use split::{Column, ColumnWidth, SplitViewController};
pub use text_field::{SecureField, TextField};
pub use toolbar::{HostedItem, HostedSearch, ToolbarChild, ToolbarContent, WindowToolbar};
pub use view_controller::ViewController;
pub use window::{Window, WindowStyle};
