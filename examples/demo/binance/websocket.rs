//! Binance Spot kline WebSocket client.
//!
//! Browser builds use `web-sys::WebSocket`. Native builds stay history-only (no socket).

use egui_charts::data::{FeedStatus, Sink, SubscriptionId};
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use super::types::{kline_stream_url, parse_kline_message};
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

/// Shared handle that owns the live subscription and optional socket.
#[derive(Clone)]
pub struct WsHandle {
    shared: Arc<Mutex<Shared>>,
    #[cfg(target_arch = "wasm32")]
    socket: Rc<RefCell<Option<web_sys::WebSocket>>>,
}

struct Shared {
    generation: u64,
    active: Option<ActiveSub>,
    reconnect_delay_ms: i32,
}

struct ActiveSub {
    id: SubscriptionId,
    sink: Sink,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    symbol: String,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    interval: String,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    generation: u64,
}

impl Default for WsHandle {
    fn default() -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared {
                generation: 0,
                active: None,
                reconnect_delay_ms: 1_000,
            })),
            #[cfg(target_arch = "wasm32")]
            socket: Rc::new(RefCell::new(None)),
        }
    }
}

impl WsHandle {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Begin (or replace) the live kline stream for this subscription.
    pub fn subscribe(
        &self,
        id: SubscriptionId,
        sink: Sink,
        symbol: String,
        interval: String,
    ) {
        {
            let mut state = self.lock();
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            state.reconnect_delay_ms = 1_000;
            sink.set_status(FeedStatus::Connecting);
            state.active = Some(ActiveSub {
                id,
                sink: sink.clone(),
                symbol: symbol.clone(),
                interval: interval.clone(),
                generation,
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ws) = self.socket.borrow_mut().take() {
                let _ = ws.close();
            }
        }
        self.connect(symbol, interval);
    }

    /// Stop the stream when the chart unsubscribes this id.
    pub fn unsubscribe(&self, id: SubscriptionId) {
        {
            let mut state = self.lock();
            let Some(active) = &state.active else {
                return;
            };
            if active.id != id {
                return;
            }
            state.generation = state.generation.wrapping_add(1);
            if let Some(active) = state.active.take() {
                active.sink.set_status(FeedStatus::Disconnected);
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(ws) = self.socket.borrow_mut().take() {
            let _ = ws.close();
        }
    }

    fn connect(&self, symbol: String, interval: String) {
        #[cfg(target_arch = "wasm32")]
        {
            wasm_connect(self, symbol, interval);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (symbol, interval);
            // Native demo keeps REST history; live sockets are browser-only in this phase.
            if let Some(active) = self.lock().active.as_ref() {
                active.sink.set_status(FeedStatus::Live);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn schedule_reconnect(&self) {
        use wasm_bindgen::JsCast;

        let (symbol, interval, delay, generation) = {
            let state = self.lock();
            let Some(active) = &state.active else {
                return;
            };
            (
                active.symbol.clone(),
                active.interval.clone(),
                state.reconnect_delay_ms,
                active.generation,
            )
        };
        let handle = self.clone();
        let Some(window) = web_sys::window() else {
            return;
        };
        let callback = wasm_bindgen::closure::Closure::once(move || {
            let still = handle
                .lock()
                .active
                .as_ref()
                .is_some_and(|a| a.generation == generation);
            if still {
                handle.connect(symbol, interval);
            }
        });
        if let Err(err) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            delay,
        ) {
            log::warn!("binance: reconnect schedule failed: {err:?}");
        }
        callback.forget();
        let mut state = self.lock();
        state.reconnect_delay_ms = (state.reconnect_delay_ms * 2).min(30_000);
    }
}

#[cfg(target_arch = "wasm32")]
fn wasm_connect(handle: &WsHandle, symbol: String, interval: String) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

    let url = kline_stream_url(&symbol, &interval);
    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(err) => {
            log::warn!("binance: websocket open failed: {err:?}");
            handle.schedule_reconnect();
            return;
        }
    };

    let generation = handle
        .lock()
        .active
        .as_ref()
        .map(|a| a.generation)
        .unwrap_or(0);

    {
        let handle = handle.clone();
        let on_open = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let mut state = handle.lock();
            if state
                .active
                .as_ref()
                .is_some_and(|a| a.generation == generation)
            {
                state.reconnect_delay_ms = 1_000;
                if let Some(active) = &state.active {
                    active.sink.set_status(FeedStatus::Live);
                }
            }
        }) as Box<dyn FnMut(_)>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();
    }

    {
        let handle = handle.clone();
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else {
                return;
            };
            let sink = {
                let state = handle.lock();
                let Some(active) = state.active.as_ref() else {
                    return;
                };
                if active.generation != generation || !active.sink.is_active() {
                    return;
                }
                active.sink.clone()
            };

            match parse_kline_message(&text) {
                Ok((bar, _closed, event_time)) => {
                    if let Some(now) = event_time {
                        sink.set_server_time(now);
                    } else {
                        sink.set_server_time(chrono::Utc::now());
                    }
                    // Sink replaces by open-time slot: open candle updates in place.
                    sink.push_bar(bar);
                }
                Err(err) => log::warn!("binance: bad kline message: {err}"),
            }
        }) as Box<dyn FnMut(_)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
    }

    {
        let handle = handle.clone();
        let on_error = Closure::wrap(Box::new(move |_event: ErrorEvent| {
            log::warn!("binance: websocket error");
            let state = handle.lock();
            if state
                .active
                .as_ref()
                .is_some_and(|a| a.generation == generation)
            {
                if let Some(active) = &state.active {
                    active.sink.set_status(FeedStatus::Disconnected);
                }
            }
        }) as Box<dyn FnMut(_)>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();
    }

    {
        let handle = handle.clone();
        let on_close = Closure::wrap(Box::new(move |_event: CloseEvent| {
            let should_reconnect = {
                let state = handle.lock();
                if state
                    .active
                    .as_ref()
                    .is_some_and(|a| a.generation == generation)
                {
                    if let Some(active) = &state.active {
                        active.sink.set_status(FeedStatus::Disconnected);
                    }
                    true
                } else {
                    false
                }
            };
            *handle.socket.borrow_mut() = None;
            if should_reconnect {
                handle.schedule_reconnect();
            }
        }) as Box<dyn FnMut(_)>);
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        on_close.forget();
    }

    *handle.socket.borrow_mut() = Some(ws);
}
