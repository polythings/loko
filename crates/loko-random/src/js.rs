use std::convert::Infallible;

use js_sys::{Function, Reflect, Uint8Array};
use rand_core::{TryCryptoRng, TryRng};
use wasm_bindgen::{JsCast, JsValue};

use crate::Random;

/// `crypto.getRandomValues` of the JavaScript host (feature `js`, `wasm32-unknown-unknown`):
/// browsers, Workers, Deno, and Node.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsRandom;

impl Random for JsRandom {
    type Rng = JsRng;

    fn rng(&self) -> JsRng {
        JsRng
    }
}

/// Numbers from `crypto.getRandomValues`; fit for secrets. Panics on a host without Web Crypto,
/// rather than handing out numbers that aren't.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsRng;

fn get_random_values() -> (JsValue, Function) {
    let crypto = Reflect::get(&js_sys::global(), &"crypto".into()).ok().filter(|c| !c.is_undefined());
    let function = crypto
        .as_ref()
        .and_then(|c| Reflect::get(c, &"getRandomValues".into()).ok())
        .and_then(|f| f.dyn_into::<Function>().ok());
    match (crypto, function) {
        (Some(crypto), Some(function)) => (crypto, function),
        _ => panic!("the JavaScript host has no crypto.getRandomValues"),
    }
}

impl TryRng for JsRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let mut bytes = [0u8; 4];
        self.try_fill_bytes(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let mut bytes = [0u8; 8];
        self.try_fill_bytes(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        let (crypto, function) = get_random_values();
        // getRandomValues fills at most 65,536 bytes per call.
        for chunk in dst.chunks_mut(65_536) {
            let array = Uint8Array::new_with_length(u32::try_from(chunk.len()).unwrap_or(u32::MAX));
            function.call1(&crypto, &array).expect("crypto.getRandomValues");
            array.copy_to(chunk);
        }
        Ok(())
    }
}

impl TryCryptoRng for JsRng {}
