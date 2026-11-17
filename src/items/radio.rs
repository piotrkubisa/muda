// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    cell::RefCell,
    collections::HashMap,
    mem,
    rc::{Rc, Weak},
    sync::{Arc, LazyLock, Mutex},
};

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    platform_impl::PlatformMenuItem,
    util, IsMenuItem, MenuEvent, MenuId, MenuItemAction, MenuItemKind, RadioMenuItemBuilder,
    StateCell, WeakStateCell,
};

/// A radio menu item inside a [`Menu`](crate::Menu) or [`Submenu`](crate::Submenu).
///
/// Radio items work in groups: activating one item checks it and unchecks the other
/// items of the same group.
#[derive(Clone)]
pub struct RadioMenuItem {
    pub(crate) id: Arc<MenuId>,
    pub(crate) state: StateCell<RadioMenuItemState>,
    pub(crate) platform: Rc<RefCell<PlatformMenuItem>>,
}

/// Shared state of a [`RadioMenuItem`].
#[derive(Debug, Clone)]
pub(crate) struct RadioMenuItemState {
    pub text: String,
    pub enabled: bool,
    pub checked: bool,
    pub accelerator: Option<MenuAccelerator>,
    pub group: String,
}

/// One radio item's place in its group: handles to its shared state and its platform
/// item, so that activating one item can uncheck the others.
struct RadioGroupEntry {
    state: WeakStateCell<RadioMenuItemState>,
    platform: Weak<RefCell<PlatformMenuItem>>,
}

// SAFETY: the registry is only locked, read, and written on the thread that runs the
// menus' click actions, which is the thread the radio items were created on. The weak
// handles never let the values they point to be used from another thread, and never
// outlive them.
unsafe impl Send for RadioGroupEntry {}

static RADIO_GROUPS: LazyLock<Mutex<HashMap<String, Vec<RadioGroupEntry>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn register_radio(
    group: &str,
    state: &StateCell<RadioMenuItemState>,
    platform: &Rc<RefCell<PlatformMenuItem>>,
) {
    RADIO_GROUPS
        .lock()
        .unwrap()
        .entry(group.to_string())
        .or_default()
        .push(RadioGroupEntry {
            state: state.downgrade(),
            platform: Rc::downgrade(platform),
        });
}

/// Unchecks every radio item of `group` except `except`, in shared state and on screen.
pub(crate) fn uncheck_radio_group(group: &str, except: Option<&StateCell<RadioMenuItemState>>) {
    let mut groups = RADIO_GROUPS.lock().unwrap();
    let Some(entries) = groups.get_mut(group) else {
        return;
    };

    entries.retain(|entry| entry.state.upgrade().is_some());

    for entry in entries.iter() {
        let Some(state) = entry.state.upgrade() else {
            continue;
        };
        if except.is_some_and(|except| state.ptr_eq(except)) {
            continue;
        }

        state.borrow_mut().checked = false;
        if let Some(platform) = entry.platform.upgrade() {
            platform.borrow_mut().set_checked(false);
        }
    }
}

/// Handles a radio item activation: checks the item, unchecks the other items of its
/// group, and emits the menu event. The caller updates the item's own native row.
pub(crate) fn activate_radio(id: &MenuId, state: &WeakStateCell<RadioMenuItemState>, group: &str) {
    let upgraded = state.upgrade();
    if let Some(state) = &upgraded {
        state.borrow_mut().checked = true;
    }
    uncheck_radio_group(group, upgraded.as_ref());
    MenuEvent::send(MenuEvent { id: id.clone() });
}

/// The shared states of the other radio items of `group`. Used from snapshot activation,
/// which runs on any thread; the states are thread-safe with the `snapshot` feature.
#[cfg(feature = "snapshot")]
pub(crate) fn radio_group_states(
    group: &str,
    except: &StateCell<RadioMenuItemState>,
) -> Vec<StateCell<RadioMenuItemState>> {
    let groups = RADIO_GROUPS.lock().unwrap();
    let Some(entries) = groups.get(group) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| entry.state.upgrade())
        .filter(|state| !state.ptr_eq(except))
        .collect()
}

/// The platform items of the other radio items of `group`. Used from snapshot activation;
/// the caller must only touch them on the items' owner thread.
#[cfg(feature = "snapshot")]
pub(crate) fn radio_group_platforms(
    group: &str,
    except: &StateCell<RadioMenuItemState>,
) -> Vec<Weak<RefCell<PlatformMenuItem>>> {
    let groups = RADIO_GROUPS.lock().unwrap();
    let Some(entries) = groups.get(group) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter(|entry| {
            entry
                .state
                .upgrade()
                .is_none_or(|state| !state.ptr_eq(except))
        })
        .map(|entry| entry.platform.clone())
        .collect()
}

impl crate::sealed::Sealed for RadioMenuItem {}
impl IsMenuItem for RadioMenuItem {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::Radio(self.clone())
    }

    fn id(&self) -> &MenuId {
        self.id()
    }

    fn into_id(self) -> MenuId {
        self.into_id()
    }
}

impl RadioMenuItem {
    /// Returns a new [`RadioMenuItemBuilder`].
    pub fn builder() -> RadioMenuItemBuilder {
        RadioMenuItemBuilder::new()
    }

    /// Create a new radio menu item in the group `group`.
    ///
    /// Activating this item unchecks the other radio items of the same group.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this radio menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>, G: AsRef<str>>(
        text: S,
        enabled: bool,
        checked: bool,
        accelerator: Option<Accelerator>,
        group: G,
    ) -> Self {
        Self::new_inner(
            None,
            text.as_ref(),
            enabled,
            checked,
            accelerator.map(MenuAccelerator::Physical),
            group.as_ref(),
        )
    }

    /// Create a new radio menu item with the specified id, in the group `group`.
    ///
    /// Activating this item unchecks the other radio items of the same group.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this radio menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn with_id<I: Into<MenuId>, S: AsRef<str>, G: AsRef<str>>(
        id: I,
        text: S,
        enabled: bool,
        checked: bool,
        accelerator: Option<Accelerator>,
        group: G,
    ) -> Self {
        Self::new_inner(
            Some(id.into()),
            text.as_ref(),
            enabled,
            checked,
            accelerator.map(MenuAccelerator::Physical),
            group.as_ref(),
        )
    }

    fn new_inner(
        id: Option<MenuId>,
        text: &str,
        enabled: bool,
        checked: bool,
        accelerator: Option<MenuAccelerator>,
        group: &str,
    ) -> Self {
        let id = util::next_id(id);
        let state = StateCell::new(RadioMenuItemState {
            text: text.to_string(),
            enabled,
            checked,
            accelerator,
            group: group.to_string(),
        });
        let click = MenuItemAction::Radio(id.clone(), state.downgrade(), group.to_string());
        let platform = Rc::new(RefCell::new(PlatformMenuItem::new(click)));

        register_radio(group, &state, &platform);

        Self {
            id: Arc::new(id),
            state,
            platform,
        }
    }

    /// Returns a unique identifier associated with this radio menu item.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Returns the group this radio menu item belongs to.
    pub fn group(&self) -> String {
        self.state.borrow().group.clone()
    }

    /// Get the text for this radio menu item.
    pub fn text(&self) -> String {
        let text = self.platform.borrow().text();
        text.unwrap_or_else(|| self.state.borrow().text.clone())
    }

    /// Set the text for this radio menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this radio menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        let accelerator = {
            let mut state = self.state.borrow_mut();
            state.text = text.as_ref().to_string();
            state.accelerator.clone()
        };

        self.platform
            .borrow_mut()
            .set_text(text.as_ref(), accelerator.as_ref())
    }

    /// Set the radio menu item's label to a fully custom
    /// [`NSAttributedString`](objc2_foundation::NSAttributedString) (macOS only).
    ///
    /// See [`MenuItem::set_attributed_title`](crate::MenuItem::set_attributed_title).
    #[cfg(target_os = "macos")]
    pub fn set_attributed_title(&self, title: Option<&objc2_foundation::NSAttributedString>) {
        self.platform.borrow_mut().set_attributed_title(title)
    }

    /// Get whether this radio menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        let enabled = self.platform.borrow().is_enabled();
        enabled.unwrap_or_else(|| self.state.borrow().enabled)
    }

    /// Enable or disable this radio menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.state.borrow_mut().enabled = enabled;
        self.platform.borrow_mut().set_enabled(enabled)
    }

    /// Set this radio menu item accelerator.
    ///
    /// (Note that setting an accelerator will override any existing [.set_key_accelerator()](Self::set_key_accelerator))
    pub fn set_accelerator(&self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        self.set_accelerator_inner(accelerator.map(MenuAccelerator::Physical))
    }

    /// Set this radio menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.set_accelerator()](Self::set_accelerator))
    pub fn set_key_accelerator(&self, accelerator: Option<KeyAccelerator>) -> crate::Result<()> {
        self.set_accelerator_inner(accelerator.map(MenuAccelerator::Logical))
    }

    fn set_accelerator_inner(&self, accelerator: Option<MenuAccelerator>) -> crate::Result<()> {
        let text = {
            let mut state = self.state.borrow_mut();
            state.accelerator = accelerator.clone();
            state.text.clone()
        };

        self.platform
            .borrow_mut()
            .set_accelerator(&text, accelerator.as_ref())
    }

    /// Get whether this radio menu item is checked or not.
    pub fn is_checked(&self) -> bool {
        let checked = self.platform.borrow().is_checked();
        checked.unwrap_or_else(|| self.state.borrow().checked)
    }

    /// Check or uncheck this radio menu item.
    ///
    /// Checking a radio item does not uncheck the other items of its group; that happens
    /// when an item is activated.
    pub fn set_checked(&self, checked: bool) {
        self.state.borrow_mut().checked = checked;
        self.platform.borrow_mut().set_checked(checked)
    }

    /// Convert this menu item into its menu ID.
    pub fn into_id(mut self) -> MenuId {
        if let Some(id) = Arc::get_mut(&mut self.id) {
            mem::take(id)
        } else {
            self.id().clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activating_one_radio_item_unchecks_its_group() {
        let a = RadioMenuItem::new("A", true, false, None, "g");
        let b = RadioMenuItem::new("B", true, true, None, "g");
        let other = RadioMenuItem::new("C", true, true, None, "other");

        assert!(b.is_checked());
        assert!(!a.is_checked());

        activate_radio(b.id(), &b.state.downgrade(), "g");

        assert!(b.is_checked());
        assert!(!a.is_checked());
        // A different group is untouched.
        assert!(other.is_checked());

        activate_radio(a.id(), &a.state.downgrade(), "g");
        assert!(a.is_checked());
        assert!(!b.is_checked());
    }
}
