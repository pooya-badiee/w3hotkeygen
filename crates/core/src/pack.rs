//! A flat bundle of named files: the game files the web version ships
//! (`site/public/app/game.pack`), extracted from an install with
//! `cargo run -p w3hk-ui --example pack`.
//!
//! Layout, little endian: `W3HKPACK`, entry count (u32), then per entry the
//! name length (u16), the name (UTF-8), the data length (u32) and the data.

use std::collections::HashMap;

const MAGIC: &[u8; 8] = b"W3HKPACK";

/// Name of the entry holding the game version the files came from.
pub const VERSION: &str = "w3hk:version";

pub fn write<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Vec<u8> {
    let files: Vec<_> = files.into_iter().collect();
    let mut out = MAGIC.to_vec();
    out.extend((files.len() as u32).to_le_bytes());
    for (name, data) in files {
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(name.as_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
    }
    out
}

pub fn read(pack: &[u8]) -> Option<HashMap<String, Vec<u8>>> {
    let rest = pack.strip_prefix(MAGIC)?;
    let (count, mut rest) = take(rest, 4)?;
    let mut out = HashMap::new();
    for _ in 0..u32::from_le_bytes(count.try_into().ok()?) {
        let (len, r) = take(rest, 2)?;
        let (name, r) = take(r, u16::from_le_bytes(len.try_into().ok()?) as usize)?;
        let (len, r) = take(r, 4)?;
        let (data, r) = take(r, u32::from_le_bytes(len.try_into().ok()?) as usize)?;
        out.insert(String::from_utf8(name.to_vec()).ok()?, data.to_vec());
        rest = r;
    }
    rest.is_empty().then_some(out)
}

fn take(data: &[u8], n: usize) -> Option<(&[u8], &[u8])> {
    (data.len() >= n).then(|| data.split_at(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let pack = write([("a\\b.txt", &b"hello"[..]), (VERSION, b"3.0.0")]);
        let files = read(&pack).unwrap();
        assert_eq!(files["a\\b.txt"], b"hello");
        assert_eq!(files[VERSION], b"3.0.0");
        assert!(read(&pack[..pack.len() - 1]).is_none());
        assert!(read(b"nope").is_none());
    }
}
