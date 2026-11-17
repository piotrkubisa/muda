---
"muda": minor
---

Add `set_menu_item_validator` with `MenuItemValidation` and `MenuItemValidator`: a process-wide handler called with each item's `MenuId` right before its menu opens, which can enable, check, hide, or retitle the item. On macOS it is wired to `validateMenuItem:`, on Windows to `WM_INITMENUPOPUP`, and on GTK when the menu pops up (`hidden` is unsupported on Windows and GTK 4).
