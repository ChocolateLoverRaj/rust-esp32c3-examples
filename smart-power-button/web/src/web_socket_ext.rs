// This code was before I used the `ws_stream_wasm` crate. It is no longer needed but I don't want to delete it in case I need it later.
// use std::pin::pin;
// use async_ui_web::__private_macro_only::wasm_bindgen::closure::Closure;
// use futures::future::{Either, select};
// use futures::{pin_mut, select};
// use wasm_bindgen_futures::js_sys::Promise;
// use wasm_bindgen_futures::JsFuture;
// use web_sys::{Event, WebSocket};
// use web_sys::wasm_bindgen::{JsCast, JsValue};
//
// pub trait WebSocketExt {
//     async fn until_event(&self, event: &str) -> Event;
//
//     async fn until_event_or_error(&self, event: &str) -> Result<Event, Event>;
//     async fn until_error(&self) -> Event;
//     async fn until_open(&self) -> Result<Event, Event>;
//     async fn until_message(&self) -> Result<Event, Event>;
// }
//
// impl WebSocketExt for WebSocket {
//     async fn until_event(&self, event: &str) -> Event {
//         match JsFuture::from(Promise::new(&mut |resolve, _reject| {
//             let callback = Closure::wrap(Box::new(move |e: Event| {
//                 resolve.call1(&Default::default(), &e).unwrap();
//             }) as Box<dyn FnMut(_)>);
//             self.add_event_listener_with_callback(event, &callback.as_ref().dyn_ref().unwrap()).unwrap();
//             callback.forget();
//         })).await {
//             Ok(e) => {
//                 e.dyn_into().unwrap()
//             }
//             Err(_) => unreachable!()
//         }
//     }
//
//     async fn until_error(&self) -> Event {
//         self.until_event("error").await
//     }
//
//     async fn until_event_or_error(&self, event: &str) -> Result<Event, Event> {
//         let non_error_event_future = self.until_event(event);
//         let error_event_future = self.until_error();
//         pin_mut!(non_error_event_future);
//         pin_mut!(error_event_future);
//         match select(non_error_event_future,error_event_future).await {
//             Either::Left((other_event, _error_future)) => {
//                 Ok(other_event)
//             }
//             Either::Right((error_event, _other_future)) => {
//                 Err(error_event)
//             }
//         }
//     }
//
//     async fn until_open(&self) -> Result<Event, Event> {
//         self.until_event_or_error("open").await
//     }
//
//     async fn until_message(&self) -> Result<Event, Event> {
//         self.until_event_or_error("message").await
//     }
// }