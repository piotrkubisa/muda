---
"muda": minor
---

On macOS, menus and menu items can now be constructed in unit tests, which run on worker threads: the constructors allow any thread under `cfg(test)` instead of panicking, and the drop paths skip `NSApplication` access off the main thread. Showing or attaching a menu still requires the main thread, as documented.
