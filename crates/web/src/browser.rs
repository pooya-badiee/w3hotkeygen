//! The browser side: reading the picked folder, the download, and saved keys.

use std::io;

use js_sys::{Array, Map, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, File, HtmlAnchorElement, Url};

/// The install folder the user picked: relative path → `File`. Reads slice
/// the file, so the multi-GB archives are never loaded whole.
pub struct FileSource(Map);

impl FileSource {
    pub fn new(files: Map) -> Self {
        FileSource(files)
    }
}

fn js_error(e: JsValue) -> io::Error {
    io::Error::other(e.as_string().unwrap_or_else(|| format!("{e:?}")))
}

impl w3hk_casc::Source for FileSource {
    async fn read(&self, path: &str, offset: u64, len: Option<u64>) -> io::Result<Vec<u8>> {
        let file: File = self
            .0
            .get(&path.into())
            .dyn_into()
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, format!("{path} not found")))?;
        let end = len.map_or(file.size(), |len| (offset + len) as f64);
        let blob = file.slice_with_f64_and_f64(offset as f64, end).map_err(js_error)?;
        let buffer = JsFuture::from(blob.array_buffer()).await.map_err(js_error)?;
        let bytes = Uint8Array::new(&buffer).to_vec();
        if len.is_some_and(|len| bytes.len() as u64 != len) {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, format!("{path} is shorter than expected")));
        }
        Ok(bytes)
    }

    async fn list(&self, dir: &str) -> io::Result<Vec<String>> {
        let prefix = format!("{dir}/");
        let names = self
            .0
            .keys()
            .into_iter()
            .filter_map(|k| k.ok()?.as_string())
            .filter_map(|k| k.strip_prefix(&prefix).filter(|name| !name.contains('/')).map(String::from))
            .collect();
        Ok(names)
    }
}

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

/// Back to the page's folder picker.
pub fn reload() {
    if let Some(w) = web_sys::window() {
        let _ = w.location().reload();
    }
}
