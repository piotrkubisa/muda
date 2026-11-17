---
"muda": minor
---

Add `MenuEvent::set_event_waker`, which sets a process-wide waker called for every menu event in addition to the event handler or channel. Use it to wake an event loop on each menu event while `MenuEvent::receiver` or `set_event_handler` still receives the events.
