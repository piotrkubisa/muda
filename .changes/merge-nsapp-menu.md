---
"muda": minor
---

Add `Menu::init_for_nsapp_merged` and `Menu::append_to_nsapp_main_menu` (macOS only). The first keeps the existing NSApplication main menu — the app menu with About, Services, Hide, Quit, and so on — and adds this menu's items to it as extra top-level rows instead of replacing it; the second appends a single item to the current main menu. `Menu::remove_for_nsapp` and `Menu::remove_from_main_menu` take the merged rows back out in that mode.
