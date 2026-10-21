use std::time::{Duration, SystemTime, UNIX_EPOCH};

use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};

use crate::Clock;

/// `Date.now` and `setTimeout` of the JavaScript host (feature `js`, `wasm32-unknown-unknown`):
/// browsers, Workers, and Node.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsClock;

impl Clock for JsClock {
    fn now(&self) -> SystemTime {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "milliseconds since the epoch")]
        let ms = js_sys::Date::now() as u64;
        UNIX_EPOCH + Duration::from_millis(ms)
    }

    async fn sleep(&self, duration: Duration) {
        #[expect(clippy::cast_precision_loss, reason = "timeouts are far below 2^53 ms")]
        let ms = JsValue::from_f64(duration.as_millis() as f64);
        let promise = Promise::new(&mut |resolve, _| {
            let global = js_sys::global();
            match Reflect::get(&global, &"setTimeout".into()).and_then(JsCast::dyn_into::<Function>) {
                Ok(set_timeout) => {
                    let _ = set_timeout.call2(&global, &resolve, &ms);
                }
                // A host without timers: resolve at once.
                Err(_) => {
                    let _ = resolve.call0(&JsValue::NULL);
                }
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}
