use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::ParseError;

#[derive(Serialize, Deserialize)]
struct MacBinaryFile {
    name: String,
    #[serde(with = "serde_bytes")]
    data_fork: Vec<u8>,
    #[serde(with = "serde_bytes")]
    rsrc_fork: Vec<u8>,
}

#[wasm_bindgen]
pub fn parse_macbinary(data: &[u8]) -> Result<JsValue, JsValue> {
    let file = crate::parse(data)?;
    let res = MacBinaryFile {
        name: file.filename(),
        data_fork: file.data_fork().to_vec(),
        rsrc_fork: file.resource_fork_raw().to_vec(),
    };
    let js = serde_wasm_bindgen::to_value(&res)?;
    Ok(js)
}

impl From<ParseError> for JsValue {
    fn from(err: ParseError) -> JsValue {
        JsValue::from(err.to_string())
    }
}
