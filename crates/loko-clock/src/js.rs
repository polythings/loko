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
        // `setTimeout` holds its delay in a signed 32-bit integer and fires at once above it, so a
        // longer sleep waits in steps of at most that.
        const STEP_MS: u128 = i32::MAX as u128;
        let mut left = duration.as_millis();
        loop {
            let step = left.min(STEP_MS);
            timeout(u32::try_from(step).unwrap_or(u32::MAX)).await;
            left -= step;
            if left == 0 {
                break;
            }
        }
    }
}

/// Waits `ms` milliseconds on `setTimeout`, or not at all on a host without timers.
async fn timeout(ms: u32) {
    let ms = JsValue::from_f64(f64::from(ms));
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
