use js_sys::{Function, Reflect, Uint8Array};
use wasm_bindgen::JsCast;

use crate::Random;

/// `crypto.getRandomValues` of the JavaScript host (feature `js`, `wasm32-unknown-unknown`):
/// browsers, Workers, and Node. Hosts without it fall back to `Math.random`.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsRandom;

impl Random for JsRandom {
    fn next_u64(&self) -> u64 {
        let mut bytes = [0u8; 8];
        self.fill(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    fn fill(&self, dest: &mut [u8]) {
        let crypto = Reflect::get(&js_sys::global(), &"crypto".into()).ok().filter(|c| !c.is_undefined());
        let get_random_values = crypto
            .as_ref()
            .and_then(|c| Reflect::get(c, &"getRandomValues".into()).ok().and_then(|f| f.dyn_into::<Function>().ok()));
        // getRandomValues fills at most 65,536 bytes per call.
        for chunk in dest.chunks_mut(65_536) {
            let filled = match (&crypto, &get_random_values) {
                (Some(crypto), Some(f)) => {
                    let array = Uint8Array::new_with_length(u32::try_from(chunk.len()).unwrap_or(u32::MAX));
                    f.call1(crypto, &array).is_ok() && {
                        array.copy_to(chunk);
                        true
                    }
                }
                _ => false,
            };
            if !filled {
                for b in chunk.iter_mut() {
                    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a byte")]
                    let byte = (js_sys::Math::random() * 256.0) as u8;
                    *b = byte;
                }
            }
        }
    }
}
