use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = WasmInitializer, js_name = finishFirstScreen)]
    pub fn finish_first_screen();
}
