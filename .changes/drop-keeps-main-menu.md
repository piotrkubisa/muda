---
"muda": minor
---

Add `Menu::remove_from_main_menu` (macOS), which clears the NSApp main menu only when this menu is the current main menu. Dropping a `Menu` already cleared the main menu only in that case, so dropping a context menu never removes the app's menu bar; the new method is the explicit way to take the main menu down.
