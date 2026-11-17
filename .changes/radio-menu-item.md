---
"muda": minor
---

Add `RadioMenuItem` (and `RadioMenuItemBuilder`) with a `group` parameter: activating one radio item checks it and unchecks the other items of the same group, on all platforms. On GTK 4 the item draws as a checkbox, since a `gio::Menu` has no radio row style.
