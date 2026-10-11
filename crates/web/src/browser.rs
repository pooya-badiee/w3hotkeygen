//! The browser side: the download and saved keys.

use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

/// Saves `text` as a download named `name`.
pub fn download(name: &str, text: &str) -> Result<(), JsValue> {
    let options = BlobPropertyBag::new();
    options.set_type("text/plain");
    let blob = Blob::new_with_str_sequence_and_options(&Array::of1(&text.into()), &options)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let link: HtmlAnchorElement = document.create_element("a")?.dyn_into()?;
    link.set_href(&url);
    link.set_download(name);
    link.click();
    Url::revoke_object_url(&url)
}

/// Where the last downloaded keys file is kept, so the page opens with it,
/// as the exe opens with the user's file.
const SAVED_KEYS: &str = "w3hk.customkeys";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn saved_keys() -> Option<String> {
    storage()?.get_item(SAVED_KEYS).ok()?
}

pub fn save_keys(text: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(SAVED_KEYS, text);
    }
}
