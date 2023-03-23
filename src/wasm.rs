use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::ParseError;

#[derive(Serialize)]
struct MacBinaryFile {
    name: String,
    #[serde(with = "serde_bytes")]
    data_fork: Vec<u8>,
    rsrc_fork_len: usize,
    resources: Vec<Resource>,
    created: u32,
    modified: u32,
    #[serde(rename = "type")]
    type_: String,
    creator: String,
}

#[derive(Serialize)]
struct Resource {
    #[serde(rename = "type")]
    type_: String,
    id: i16,
    name: Option<String>,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

#[wasm_bindgen]
pub fn parse_macbinary(val: JsValue) -> Result<JsValue, JsValue> {
    let data: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(val)?;
    let file = crate::parse(&data)?;

    let mut resources = Vec::new();
    if let Some(rsrc) = file.resource_fork()? {
        for item in rsrc.resource_types() {
            resources.extend(rsrc.resources(item).map(|resource| Resource {
                type_: item.resource_type().to_string(),
                id: resource.id(),
                name: resource.name(),
                data: resource.data().to_vec(),
            }))
        }
    }

    let res = MacBinaryFile {
        name: file.filename(),
        data_fork: file.data_fork().to_vec(),
        rsrc_fork_len: file.resource_fork_raw().len(),
        resources,
        created: file.created(),
        modified: file.modified(),
        creator: file.file_creator().to_string(),
        type_: file.file_type().to_string(),
    };
    let js = serde_wasm_bindgen::to_value(&res)?;
    Ok(js)
}

impl From<ParseError> for JsValue {
    fn from(err: ParseError) -> JsValue {
        JsValue::from(err.to_string())
    }
}
