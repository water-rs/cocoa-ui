//! `UIMenu` built from the shared menu tree.
//!
//! `elements` turns [`MenuTreeNode`]s into `UIMenuElement`s — dividers
//! become inline groups, commands become `UIAction`s carrying the command's
//! attributes (disabled, destructive, checked) — and `menu` wraps them into
//! a `UIMenu`. Actions run the command's Rust callback.
//!
//! # Safety
//!
//! The `unsafe` here builds `UIAction`s from `RcBlock`s and calls `objc2`/
//! `UIKit` bindings marked unsafe because `UIKit` objects are main-thread
//! only, which [`MainThreadMarker`] guarantees at construction.

use std::rc::Rc;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::NSString;
use objc2_ui_kit::{
    UIAction, UIImage, UIMenu, UIMenuElement, UIMenuElementAttributes, UIMenuElementState,
    UIMenuOptions,
};
use std::ptr::NonNull;

use crate::menu::{Command, MenuTreeNode};

/// A `UIMenu` from a menu tree.
#[must_use]
pub fn menu(mtm: MainThreadMarker, title: &Command, nodes: &[MenuTreeNode]) -> Retained<UIMenu> {
    let image = menu_image(title.symbol.as_deref());
    UIMenu::menuWithTitle_image_identifier_options_children(
        &NSString::from_str(&title.label),
        image.as_deref(),
        None,
        UIMenuOptions::empty(),
        &objc2_foundation::NSArray::from_slice(
            &elements(mtm, nodes)
                .iter()
                .map(|e| &**e)
                .collect::<Vec<_>>(),
        ),
        mtm,
    )
}

/// `UIMenuElement`s from a menu tree — dividers split the list into inline
/// groups.
#[must_use]
pub fn elements(mtm: MainThreadMarker, nodes: &[MenuTreeNode]) -> Vec<Retained<UIMenuElement>> {
    // `MenuTreeNode` is not `Clone`; walk references.
    let mut current: Vec<&MenuTreeNode> = Vec::new();
    let mut group_refs: Vec<Vec<&MenuTreeNode>> = Vec::new();
    for node in nodes {
        if matches!(node, MenuTreeNode::Divider) {
            if !current.is_empty() {
                group_refs.push(std::mem::take(&mut current));
            }
        } else {
            current.push(node);
        }
    }
    if !current.is_empty() {
        group_refs.push(current);
    }
    let mut out: Vec<Retained<UIMenuElement>> = Vec::new();
    let single = group_refs.len() == 1;
    for group in group_refs {
        let children: Vec<Retained<UIMenuElement>> =
            group.iter().filter_map(|node| element(mtm, node)).collect();
        if single {
            out.extend(children);
            continue;
        }
        out.push(
            UIMenu::menuWithTitle_image_identifier_options_children(
                &NSString::new(),
                None,
                None,
                UIMenuOptions::DisplayInline,
                &objc2_foundation::NSArray::from_slice(
                    &children.iter().map(|e| &**e).collect::<Vec<_>>(),
                ),
                mtm,
            )
            .into_super(),
        );
    }
    out
}

fn element(mtm: MainThreadMarker, node: &MenuTreeNode) -> Option<Retained<UIMenuElement>> {
    match node {
        MenuTreeNode::Divider => None,
        MenuTreeNode::Submenu(command, children) => Some(menu(mtm, command, children).into_super()),
        MenuTreeNode::Command(command, action) => {
            Some(command_element(mtm, command, action.clone()))
        }
    }
}

fn command_element(
    mtm: MainThreadMarker,
    command: &Command,
    action: Rc<dyn Fn()>,
) -> Retained<UIMenuElement> {
    let mut attributes = UIMenuElementAttributes::empty();
    if !command.enabled {
        attributes |= UIMenuElementAttributes::Disabled;
    }
    if command.destructive {
        attributes |= UIMenuElementAttributes::Destructive;
    }
    let state = if command.selected {
        UIMenuElementState::On
    } else {
        UIMenuElementState::Off
    };
    let block = RcBlock::new(move |_action: NonNull<UIAction>| {
        action();
    });
    // SAFETY: `actionWithTitle:image:identifier:handler:` retains the block.
    let ui_action = unsafe {
        UIAction::actionWithTitle_image_identifier_handler(
            &NSString::from_str(&command.label),
            menu_image(command.symbol.as_deref()).as_deref(),
            None,
            block2::RcBlock::into_raw(block),
            mtm,
        )
    };
    ui_action.setAttributes(attributes);
    ui_action.setState(state);
    if let Some(subtitle) = &command.subtitle {
        ui_action.setSubtitle(Some(&NSString::from_str(subtitle)));
    }
    if !command.key_equivalent.is_empty() {
        ui_action.setDiscoverabilityTitle(Some(&NSString::from_str(&command.key_equivalent)));
    }
    ui_action.into_super()
}

fn menu_image(symbol: Option<&str>) -> Option<Retained<UIImage>> {
    symbol.and_then(|symbol| UIImage::systemImageNamed(&NSString::from_str(symbol)))
}
