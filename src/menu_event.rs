// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::{Arc, LazyLock, Mutex};

use crate::MenuId;

/// Describes a menu event emitted when a menu item is activated
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MenuEvent {
    /// Id of the menu item which triggered this event
    pub id: MenuId,
}

/// A receiver that could be used to listen to menu events.
pub type MenuEventReceiver = Receiver<MenuEvent>;

/// A handler for menu events, see [`MenuEvent::set_event_handler`].
pub type MenuEventHandler = Arc<dyn Fn(MenuEvent) + Send + Sync + 'static>;

/// A waker called for every menu event, see [`MenuEvent::set_event_waker`].
pub type MenuEventWaker = Arc<dyn Fn() + Send + Sync + 'static>;

static MENU_CHANNEL: LazyLock<(Sender<MenuEvent>, MenuEventReceiver)> = LazyLock::new(unbounded);
static MENU_EVENT_HANDLER: Mutex<Option<MenuEventHandler>> = Mutex::new(None);
static MENU_EVENT_WAKER: Mutex<Option<MenuEventWaker>> = Mutex::new(None);

impl MenuEvent {
    /// Returns the id of the menu item which triggered this event
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Gets a reference to the event channel's [`MenuEventReceiver`]
    /// which can be used to listen for menu events.
    ///
    /// ## Note
    ///
    /// This will not receive any events while a handler is set with [`MenuEvent::set_event_handler`].
    /// A waker set with [`MenuEvent::set_event_waker`] does not change that.
    pub fn receiver<'a>() -> &'a MenuEventReceiver {
        &MENU_CHANNEL.1
    }

    /// Set a handler to be called for new events.
    ///
    /// This is intended for routing menu events through an event loop, using something like
    /// winit's [`EventLoopProxy`], so that the event loop wakes up on each menu event.
    ///
    /// Only one handler can be set at a time, so this replaces the current handler and returns it.
    /// Passing the returned handler back to this function restores it. Calling this function with
    /// `None` removes the current handler.
    ///
    /// ## Examples
    ///
    /// Route menu events through a winit event loop:
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use muda::MenuEvent;
    /// # use winit::event_loop::EventLoop;
    /// enum UserEvent {
    ///     MenuEvent(MenuEvent),
    /// }
    ///
    /// let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    ///
    /// let proxy = event_loop.create_proxy();
    /// MenuEvent::set_event_handler(Some(Arc::new(move |event| {
    ///     proxy.send_event(UserEvent::MenuEvent(event));
    /// })));
    /// ```
    ///
    /// When this function is called more than once, only the handler from the last call receives
    /// events. To have the replaced handler still receive the events that the new one doesn't
    /// handle, pass them to it:
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use muda::{MenuEvent, MenuItem};
    /// let item = MenuItem::new("Item", true, None);
    /// let item_id = item.id().clone();
    ///
    /// // Take the current handler first, so that the new one can call it.
    /// let previous = MenuEvent::set_event_handler(None);
    /// MenuEvent::set_event_handler(Some(Arc::new(move |event| {
    ///     if event.id == item_id {
    ///         println!("Item activated");
    ///     } else if let Some(previous) = &previous {
    ///         previous(event);
    ///     }
    /// })));
    /// ```
    ///
    /// ## Note
    ///
    /// - While a handler is set, new events are not sent to the channel associated with [`MenuEvent::receiver`].
    /// - The handler is only called for menus created by this version of muda, see
    ///   [multiple versions of muda](crate#note-for-multiple-versions-of-muda).
    ///
    /// [`EventLoopProxy`]: https://docs.rs/winit/latest/winit/event_loop/struct.EventLoopProxy.html
    pub fn set_event_handler(handler: Option<MenuEventHandler>) -> Option<MenuEventHandler> {
        std::mem::replace(&mut *MENU_EVENT_HANDLER.lock().unwrap(), handler)
    }

    /// Set a waker to be called for every new event, before the handler or channel runs.
    ///
    /// This is meant for waking up an event loop on each menu event, so that the loop can
    /// then drain the events, for example with [`MenuEvent::set_event_handler`] forwarding
    /// them through an [`EventLoopProxy`]. Use it together with the handler or the channel:
    /// unlike the handler, the waker does not stop [`MenuEvent::receiver`] from receiving
    /// events, and it runs even while a handler is set.
    ///
    /// Only one waker can be set at a time, so this replaces the current waker and returns
    /// it. Passing the returned waker back to this function restores it. Calling this
    /// function with `None` removes the current waker.
    ///
    /// ## Examples
    ///
    /// Wake a loop through a channel while the events still go to the handler:
    ///
    /// ```
    /// # use std::sync::{mpsc, Arc};
    /// # use muda::MenuEvent;
    /// let (tx, rx) = mpsc::channel::<()>();
    /// let previous = MenuEvent::set_event_waker(Some(Arc::new(move || {
    ///     let _ = tx.send(());
    /// })));
    /// // A menu event now wakes the receiver, e.g. in another thread:
    /// // rx.recv_timeout(Duration::from_secs(1));
    /// MenuEvent::set_event_waker(previous);
    /// ```
    ///
    /// ## Note
    ///
    /// - The waker is only called for menus created by this version of muda, see
    ///   [multiple versions of muda](crate#note-for-multiple-versions-of-muda).
    ///
    /// [`EventLoopProxy`]: https://docs.rs/winit/latest/winit/event_loop/struct.EventLoopProxy.html
    pub fn set_event_waker(waker: Option<MenuEventWaker>) -> Option<MenuEventWaker> {
        std::mem::replace(&mut *MENU_EVENT_WAKER.lock().unwrap(), waker)
    }

    pub(crate) fn send(event: MenuEvent) {
        // Don't hold the locks while running the waker or handler,
        // as they may set a new waker or handler or send another event.
        let waker = MENU_EVENT_WAKER.lock().unwrap().clone();
        let handler = MENU_EVENT_HANDLER.lock().unwrap().clone();
        if let Some(waker) = waker {
            waker();
        }
        if let Some(handler) = handler {
            handler(event);
        } else {
            let _ = MENU_CHANNEL.0.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn waker_runs_next_to_handler_and_channel() {
        static WAKES: AtomicUsize = AtomicUsize::new(0);
        static HANDLED: AtomicUsize = AtomicUsize::new(0);

        let waker: MenuEventWaker = Arc::new(|| {
            WAKES.fetch_add(1, Ordering::SeqCst);
        });
        let handler: MenuEventHandler = Arc::new(|_| {
            HANDLED.fetch_add(1, Ordering::SeqCst);
        });

        let previous_waker = MenuEvent::set_event_waker(Some(waker));
        let previous_handler = MenuEvent::set_event_handler(Some(handler));

        MenuEvent::send(MenuEvent {
            id: MenuId::new("waker-test"),
        });

        assert_eq!(WAKES.load(Ordering::SeqCst), 1);
        assert_eq!(HANDLED.load(Ordering::SeqCst), 1);

        // Without a handler, the event still reaches the channel after waking.
        MenuEvent::set_event_handler(None);
        MenuEvent::send(MenuEvent {
            id: MenuId::new("waker-test-2"),
        });
        assert_eq!(WAKES.load(Ordering::SeqCst), 2);
        // Other tests share this channel, so drain until our event shows up.
        let mut found = false;
        for _ in 0..16 {
            match MenuEvent::receiver().try_recv() {
                Ok(event) if event.id == MenuId::new("waker-test-2") => {
                    found = true;
                    break;
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        assert!(found);

        MenuEvent::set_event_waker(previous_waker);
        MenuEvent::set_event_handler(previous_handler);
    }
}
