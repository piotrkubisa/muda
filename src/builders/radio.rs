// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    MenuId, RadioMenuItem,
};

/// A builder type for [`RadioMenuItem`]
#[derive(Clone, Debug)]
pub struct RadioMenuItemBuilder {
    text: String,
    enabled: bool,
    checked: bool,
    accelerator: Option<MenuAccelerator>,
    id: Option<MenuId>,
    group: String,
}

impl Default for RadioMenuItemBuilder {
    fn default() -> Self {
        Self {
            text: String::new(),
            enabled: true,
            checked: false,
            accelerator: None,
            id: None,
            group: String::new(),
        }
    }
}

impl RadioMenuItemBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the id this radio menu item.
    pub fn id(mut self, id: MenuId) -> Self {
        self.id.replace(id);
        self
    }

    /// Set the text for this radio menu item.
    ///
    /// See [`RadioMenuItem::set_text`] for more info.
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = text.into();
        self
    }

    /// Enable or disable this menu item.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Check or uncheck this menu item.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the group this radio menu item belongs to.
    ///
    /// Activating one radio item unchecks the other items of the same group.
    pub fn group<S: Into<String>>(mut self, group: S) -> Self {
        self.group = group.into();
        self
    }

    /// Set this radio menu item accelerator.
    ///
    /// (Note that setting an accelerator will override any existing [.key_accelerator()](Self::key_accelerator))
    pub fn accelerator<A: TryInto<Accelerator>>(
        mut self,
        accelerator: Option<A>,
    ) -> crate::Result<Self>
    where
        crate::Error: From<<A as TryInto<Accelerator>>::Error>,
    {
        self.accelerator = accelerator
            .map(|a| a.try_into().map(MenuAccelerator::Physical))
            .transpose()?;
        Ok(self)
    }

    /// Set this radio menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.accelerator()](Self::accelerator))
    pub fn key_accelerator<A: TryInto<KeyAccelerator>>(
        mut self,
        accelerator: Option<A>,
    ) -> crate::Result<Self>
    where
        crate::Error: From<<A as TryInto<KeyAccelerator>>::Error>,
    {
        self.accelerator = accelerator
            .map(|a| a.try_into().map(MenuAccelerator::Logical))
            .transpose()?;
        Ok(self)
    }

    /// Build this radio menu item.
    pub fn build(self) -> RadioMenuItem {
        let item = if let Some(id) = self.id {
            RadioMenuItem::with_id(id, self.text, self.enabled, self.checked, None, self.group)
        } else {
            RadioMenuItem::new(self.text, self.enabled, self.checked, None, self.group)
        };

        if let Some(accelerator) = self.accelerator {
            let _ = match accelerator {
                MenuAccelerator::Physical(accelerator) => item.set_accelerator(Some(accelerator)),
                MenuAccelerator::Logical(accelerator) => {
                    item.set_key_accelerator(Some(accelerator))
                }
            };
        }

        item
    }
}
