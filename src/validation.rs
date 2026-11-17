// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::sync::{Arc, Mutex};

use crate::MenuId;

/// A change to apply to a menu item right before the menu that holds it opens.
///
/// The default value changes nothing: every field set to `None` leaves the item as it is.
///
/// See [`set_menu_item_validator`].
#[derive(Debug, Clone, Default)]
pub struct MenuItemValidation {
    /// Enable or disable the item.
    pub enabled: Option<bool>,
    /// Check or uncheck the item. Only applies to items that have a checked state,
    /// like [`CheckMenuItem`](crate::CheckMenuItem).
    pub checked: Option<bool>,
    /// Hide or show the item.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 4:** Unsupported, the field is ignored.
    pub hidden: Option<bool>,
    /// Replace the item's title.
    pub title: Option<String>,
}

/// A handler that maps a menu item id to the [`MenuItemValidation`] to apply to that item
/// right before the menu that holds it opens. See [`set_menu_item_validator`].
pub type MenuItemValidator = Arc<dyn Fn(&MenuId) -> MenuItemValidation + Send + Sync + 'static>;

static MENU_ITEM_VALIDATOR: Mutex<Option<MenuItemValidator>> = Mutex::new(None);

/// Set a handler that is called with each item's [`MenuId`] right before the menu that
/// holds the item opens, so the item can be enabled, checked, hidden, or retitled on the
/// fly. The handler returns a [`MenuItemValidation`]; its default value changes nothing.
///
/// Only one validator can be set at a time, so this replaces the current validator and
/// returns it. Passing the returned validator back to this function restores it. Calling
/// this function with `None` removes the current validator.
///
/// ## Examples
///
/// Disable one item by id and retitle another:
///
/// ```
/// # use std::sync::Arc;
/// # use muda::{set_menu_item_validator, MenuId, MenuItemValidation};
/// let previous = set_menu_item_validator(Some(Arc::new(|id: &MenuId| {
///     if id == "save" {
///         MenuItemValidation {
///             enabled: Some(false),
///             ..MenuItemValidation::default()
///         }
///     } else if id == "title" {
///         MenuItemValidation {
///             title: Some("New title".to_string()),
///             ..MenuItemValidation::default()
///         }
///     } else {
///         MenuItemValidation::default()
///     }
/// })));
/// // Restore the previous validator later:
/// set_menu_item_validator(previous);
/// ```
///
/// ## Platform-specific:
///
/// - **macOS:** Applied from `validateMenuItem:`, once per item each time its menu opens.
/// - **Windows:** Applied when the menu-open message (`WM_INITMENUPOPUP`) arrives.
///   `hidden` is ignored; classic Win32 menus have no hidden state.
/// - **GTK 3 / GTK 4:** Applied when the menu pops up. On GTK 4, `hidden` is ignored.
pub fn set_menu_item_validator(validator: Option<MenuItemValidator>) -> Option<MenuItemValidator> {
    std::mem::replace(&mut *MENU_ITEM_VALIDATOR.lock().unwrap(), validator)
}

/// Runs the validator for `id`, if one is set. Returns `None` when no validator is set.
pub(crate) fn validate_menu_item(id: &MenuId) -> Option<MenuItemValidation> {
    MENU_ITEM_VALIDATOR
        .lock()
        .unwrap()
        .as_ref()
        .map(|validator| validator(id))
}

/// Whether a validator is currently set.
pub(crate) fn validator_is_set() -> bool {
    MENU_ITEM_VALIDATOR.lock().unwrap().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_replaces_and_restores() {
        let first: MenuItemValidator = Arc::new(|_| MenuItemValidation {
            enabled: Some(false),
            ..MenuItemValidation::default()
        });
        let second: MenuItemValidator = Arc::new(|_| MenuItemValidation::default());

        let none = set_menu_item_validator(Some(first));
        assert!(none.is_none());
        assert_eq!(
            validate_menu_item(&MenuId::new("x")).unwrap().enabled,
            Some(false)
        );

        let previous = set_menu_item_validator(Some(second));
        assert!(previous.is_some());
        assert_eq!(validate_menu_item(&MenuId::new("x")).unwrap().enabled, None);

        set_menu_item_validator(previous);
        assert_eq!(
            validate_menu_item(&MenuId::new("x")).unwrap().enabled,
            Some(false)
        );
        set_menu_item_validator(None);
        assert!(validate_menu_item(&MenuId::new("x")).is_none());
    }
}
