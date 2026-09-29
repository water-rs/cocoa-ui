//! Menus built from standard actions.
//!
//! # Safety
//!
//! The `unsafe` here creates menu items whose action is sent to the first
//! responder: the item has no target, so `AppKit` walks the responder chain
//! for an object that implements the action and disables the item when none
//! does. No action can reach an object that does not understand it.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use objc2::Message;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSColor, NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{NSMutableAttributedString, NSObject, NSObjectProtocol, NSRange, NSString};

use crate::callback::guarded;
use crate::menu::{Command, KeyModifiers, MenuTreeNode};

/// A menu: a list of items, shown as the menu bar or as a submenu.
///
/// A `Menu` is a handle: clones refer to the same menu.
#[derive(Debug, Clone)]
pub struct Menu {
    menu: Retained<NSMenu>,
}

impl Menu {
    /// An empty menu titled `title`.
    #[must_use]
    pub fn new(mtm: MainThreadMarker, title: &str) -> Self {
        Self {
            menu: NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title)),
        }
    }

    /// Appends `item`, which belongs to this menu from then on.
    pub fn add_item(&self, item: MenuItem) {
        let MenuItem { item } = item;
        self.menu.addItem(&item);
    }

    /// Appends a separator line.
    pub fn add_separator(&self) {
        self.menu
            .addItem(&NSMenuItem::separatorItem(self.menu.mtm()));
    }

    pub(super) fn native(&self) -> &NSMenu {
        &self.menu
    }
}

/// One entry of a [`Menu`].
#[derive(Debug)]
pub struct MenuItem {
    item: Retained<NSMenuItem>,
}

impl MenuItem {
    /// An item titled `title` that performs `action` when chosen, or does
    /// nothing when `action` is `None`.
    ///
    /// `key_equivalent` is the key that chooses the item with ⌘ held (a
    /// lowercase letter, or `""` for none); [`MenuItem::with_key_modifiers`]
    /// changes which modifiers it takes.
    #[must_use]
    pub fn new(
        mtm: MainThreadMarker,
        title: &str,
        action: Option<MenuAction>,
        key_equivalent: &str,
    ) -> Self {
        // SAFETY: see the module safety note.
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                action.map(MenuAction::selector),
                &NSString::from_str(key_equivalent),
            )
        };
        Self { item }
    }

    /// The same item, chosen by its key equivalent with exactly `modifiers`
    /// held instead of ⌘ alone.
    #[must_use]
    pub fn with_key_modifiers(self, modifiers: KeyModifiers) -> Self {
        self.item.setKeyEquivalentModifierMask(modifiers.native());
        self
    }

    /// The same item, opening `submenu` instead of performing an action.
    #[must_use]
    pub fn with_submenu(self, submenu: &Menu) -> Self {
        self.item.setSubmenu(Some(&submenu.menu));
        self
    }
}

/// A standard command a menu item sends to whichever object currently
/// handles it: the focused text view for editing commands, the key window for
/// window commands, the application for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuAction {
    /// Shows the standard About panel.
    About,
    /// Hides the application.
    Hide,
    /// Hides every other application.
    HideOthers,
    /// Shows every hidden application.
    ShowAll,
    /// Quits the application.
    Quit,
    /// Undoes the last change.
    Undo,
    /// Redoes the last undone change.
    Redo,
    /// Cuts the selection to the pasteboard.
    Cut,
    /// Copies the selection to the pasteboard.
    Copy,
    /// Pastes the pasteboard's contents.
    Paste,
    /// Deletes the selection.
    Delete,
    /// Selects everything.
    SelectAll,
    /// Minimizes the key window into the Dock.
    Minimize,
    /// Toggles the key window between its standard and its user size.
    Zoom,
    /// Brings every window of the application to the front.
    BringAllToFront,
}

impl MenuAction {
    fn selector(self) -> Sel {
        match self {
            Self::About => sel!(orderFrontStandardAboutPanel:),
            Self::Hide => sel!(hide:),
            Self::HideOthers => sel!(hideOtherApplications:),
            Self::ShowAll => sel!(unhideAllApplications:),
            Self::Quit => sel!(terminate:),
            Self::Undo => sel!(undo:),
            Self::Redo => sel!(redo:),
            Self::Cut => sel!(cut:),
            Self::Copy => sel!(copy:),
            Self::Paste => sel!(paste:),
            Self::Delete => sel!(delete:),
            Self::SelectAll => sel!(selectAll:),
            Self::Minimize => sel!(miniaturize:),
            Self::Zoom => sel!(zoom:),
            Self::BringAllToFront => sel!(arrangeInFront:),
        }
    }
}

#[cfg(test)]
mod tests {
    use objc2_app_kit::NSEventModifierFlags;

    use super::KeyModifiers;

    #[test]
    fn modifiers_map_to_their_event_flags() {
        assert_eq!(
            (KeyModifiers::COMMAND | KeyModifiers::OPTION).native(),
            NSEventModifierFlags::Command | NSEventModifierFlags::Option
        );
        assert_eq!(
            (KeyModifiers::COMMAND | KeyModifiers::SHIFT).native(),
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift
        );
        assert_eq!(
            KeyModifiers::CONTROL.native(),
            NSEventModifierFlags::Control
        );
        assert_eq!(
            KeyModifiers::empty().native(),
            NSEventModifierFlags::empty()
        );
    }
}

/// An `NSMenuItem` target that runs a Rust callback.
pub struct MenuItemTargetIvars {
    /// Called when the item is chosen.
    action: RefCell<Option<Rc<dyn Fn()>>>,
}

impl fmt::Debug for MenuItemTargetIvars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuItemTargetIvars").finish()
    }
}

define_class!(
    // SAFETY: `NSObject`'s designated initializer is `init`, which
    // `MenuItemTarget::new` calls, and the class does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "CocoaUiMenuItemTarget"]
    #[thread_kind = MainThreadOnly]
    #[ivars = MenuItemTargetIvars]
    #[derive(Debug)]
    /// The target `AppKit` calls when a callback menu item is chosen.
    pub struct MenuItemTarget;

    // SAFETY: `NSObjectProtocol` asks nothing of an `NSObject`.
    unsafe impl NSObjectProtocol for MenuItemTarget {}

    impl MenuItemTarget {
        // SAFETY: `menuItemFired:` is this class's own target action.
        #[unsafe(method(menuItemFired:))]
        fn menu_item_fired(&self, _sender: &NSMenuItem) {
            guarded("MenuItemTarget menuItemFired:", || {
                let handler = self.ivars().action.borrow().clone();
                if let Some(handler) = handler {
                    handler();
                }
            });
        }
    }
);

impl MenuItemTarget {
    fn new(mtm: MainThreadMarker, handler: Rc<dyn Fn()>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MenuItemTargetIvars {
            action: RefCell::new(Some(handler)),
        });
        // SAFETY: `init` is `NSObject`'s designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

impl MenuItem {
    /// An item from a [`Command`]. `action` runs when it is chosen.
    #[must_use]
    pub fn command(mtm: MainThreadMarker, command: &Command, action: Rc<dyn Fn()>) -> Self {
        // SAFETY: see the module safety note.
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&command.label),
                Some(sel!(menuItemFired:)),
                &NSString::from_str(&command.key_equivalent),
            )
        };
        let target = MenuItemTarget::new(mtm, action);
        // SAFETY: `setTarget:` accepts any object; the item holds it weakly,
        // so the target is retained as the item's `representedObject` as well.
        unsafe {
            item.setTarget(Some(
                std::ptr::from_ref::<MenuItemTarget>(target.as_ref())
                    .cast::<AnyObject>()
                    .as_ref()
                    .unwrap_unchecked(),
            ));
        };
        // SAFETY: `setRepresentedObject:` retains its argument.
        unsafe {
            item.setRepresentedObject(Some(
                std::ptr::from_ref::<MenuItemTarget>(target.as_ref())
                    .cast::<AnyObject>()
                    .as_ref()
                    .unwrap_unchecked(),
            ));
        }
        Self::apply_command(&item, command)
    }

    /// An item titled `label` that opens `submenu`.
    #[must_use]
    pub fn submenu(mtm: MainThreadMarker, command: &Command, submenu: &NSMenu) -> Self {
        // SAFETY: see the module safety note.
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(&command.label),
                None,
                &NSString::new(),
            )
        };
        item.setSubmenu(Some(submenu));
        Self::apply_command(&item, command)
    }

    /// Applies a [`Command`]'s presentation to the item: enabled state,
    /// checkmark, key modifiers, symbol image, subtitle and the destructive
    /// style — `AppKit` draws a destructive command with the system-red
    /// title.
    fn apply_command(item: &NSMenuItem, command: &Command) -> Self {
        item.setEnabled(command.enabled);
        item.setState(if command.selected {
            objc2_app_kit::NSControlStateValueOn
        } else {
            objc2_app_kit::NSControlStateValueOff
        });
        item.setKeyEquivalentModifierMask(command.modifiers.native());
        if let Some(symbol) = &command.symbol
            && let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str(symbol),
                None,
            )
        {
            item.setImage(Some(&image));
        }
        if let Some(subtitle) = &command.subtitle {
            item.setSubtitle(Some(&NSString::from_str(subtitle)));
        }
        if command.destructive {
            let title = NSMutableAttributedString::initWithString(
                item.mtm().alloc::<NSMutableAttributedString>(),
                &NSString::from_str(&command.label),
            );
            // SAFETY: `addAttribute:value:range:` takes any attribute-value
            // pair on a live attributed string.
            unsafe {
                title.addAttribute_value_range(
                    objc2_app_kit::NSForegroundColorAttributeName,
                    &NSColor::systemRedColor(),
                    NSRange::new(0, title.length()),
                );
            }
            item.setAttributedTitle(Some(&title));
        }
        Self {
            item: item.retain(),
        }
    }
}

impl Menu {
    /// Replaces the menu's contents with `nodes`.
    pub fn set_nodes(&self, nodes: &[MenuTreeNode]) {
        self.menu.removeAllItems();
        let mtm = self.menu.mtm();
        for node in nodes {
            match node {
                MenuTreeNode::Divider => self.add_separator(),
                MenuTreeNode::Command(command, action) => {
                    self.add_item(MenuItem::command(mtm, command, action.clone()));
                }
                MenuTreeNode::Submenu(command, children) => {
                    let submenu = Self::new(mtm, &command.label);
                    submenu.set_nodes(children);
                    self.add_item(MenuItem::submenu(mtm, command, &submenu.menu));
                }
            }
        }
    }

    /// The raw `NSMenu`.
    #[must_use]
    pub fn menu(&self) -> &NSMenu {
        &self.menu
    }
}
