//! `UIKit`, the user-interface framework of iOS.
//!
//! An iOS application is started with [`run`] and a set of
//! [`ApplicationHandlers`]. The system then connects a [`WindowScene`] for the
//! application's window, and the scene handler answers with the [`Window`] to
//! show in it, usually rooted in a [`ViewController`] whose [`HostView`] fills
//! the window.
//!
//! The application's `Info.plist` must name `SceneDelegate` as the delegate
//! class of its window scene configuration: that is the class through which
//! the system reaches the scene handler.
//!
//! ```no_run
//! use cocoa_ui::MainThreadMarker;
//! use cocoa_ui::uikit::{self, ApplicationHandlers, ViewController, Window};
//!
//! let mtm = MainThreadMarker::new().expect("main runs on the main thread");
//! uikit::run(
//!     mtm,
//!     ApplicationHandlers::new(|scene| {
//!         let window = Window::new(scene);
//!         let controller = ViewController::new(scene.main_thread());
//!         controller.host_view().set_layout_handler(|_view| {
//!             // Give the subviews their frames here.
//!         });
//!         window.set_root_view_controller(&controller);
//!         window.make_key_and_visible();
//!         window
//!     }),
//! );
//! ```

mod appearance;
mod application;
mod badge;
pub mod button;
mod calendar_view;
mod color_view;
pub mod color_well;
mod date_picker;
pub use color_well::ColorWell;
pub mod colors;
mod host_view;
pub mod image;
mod menu;
mod navigation;
mod search_bar;
mod split;
mod tabs;
mod view_controller;
mod window;

pub use appearance::{ColorSchemeObservation, current_scheme};
pub use application::{ApplicationHandlers, WindowScene, run};
pub use badge::BadgeView;
pub use button::Button;
pub use calendar_view::CalendarView;
pub use color_view::ColorView;
pub use colors::UiColor;
pub use date_picker::{DatePicker, DatePickerMode};
pub use host_view::{HitTest, HostView};
pub use image::ImageView;
pub mod label;
mod picker;
pub use picker::Picker;
pub mod toggle;
pub use label::Label;
mod slider;
pub mod text_field;
pub use slider::Slider;
mod progress;
pub use progress::Progress;
mod scroll;
pub use scroll::ScrollView;
mod stepper;
pub use menu::{elements, menu};
pub use navigation::{
    BarButton, LargeTitle, NavBar, NavContentController, NavPage, NavSearch, NavigationController,
    SearchUpdater, bar_item, bar_item_accessibility_label, first_control,
};
pub use search_bar::SearchBar;
pub use split::{ColumnWidth, SplitColumnController, SplitController};
pub use stepper::Stepper;
pub use tabs::{TabContentController, TabSpec, TabsController};
pub use text_field::{SecureField, TextField};
pub use view_controller::ViewController;
pub use window::Window;
