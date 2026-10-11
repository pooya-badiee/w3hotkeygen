//! Local index files (`Data/data/XXyyyyyyyy.idx`, version 7): where each
//! encoded key sits in the `data.###` archives.

use std::collections::HashMap;

use crate::{EKey, Error, Source};

/// A file's encoded bytes inside an archive, including the 30-byte local
/// header in front of its BLTE data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub archive: u32,
    pub offset: u64,
    pub size: u64,
}

const DIR: &str = "Data/data";
/// Guarded header block (8) + header (16) + padding (8).
const ENTRIES_BLOCK: usize = 0x20;
/// Encoded key (9) + archive and offset (5, big endian) + size (4, little endian).
const ENTRY: usize = 9 + 5 + 4;

pub fn archive_path(archive: u32) -> String {
    format!("{DIR}/data.{archive:03}")
}

/// Every index entry. Each of the 16 buckets can have several versions of its
/// file on disk; only the newest one counts.
pub async fn load(source: &impl Source) -> Result<HashMap<EKey, Location>, Error> {
    let mut newest: HashMap<u8, (u32, String)> = HashMap::new();
    for name in source.list(DIR).await? {
        let Some((bucket, version)) = parse_name(&name) else { continue };
        if newest.get(&bucket).is_none_or(|(v, _)| version > *v) {
            newest.insert(bucket, (version, name));
        }
    }
    if newest.is_empty() {
        return Err(Error::Format("no index files in Data/data"));
    }
    let mut out = HashMap::new();
    for (_, name) in newest.into_values() {
        parse(&source.read(&format!("{DIR}/{name}"), 0, None).await?, &mut out)?;
    }
    Ok(out)
}

/// `0700000008.idx` → bucket 0x07, version 8.
fn parse_name(name: &str) -> Option<(u8, u32)> {
    let stem = name.strip_suffix(".idx").or_else(|| name.strip_suffix(".IDX"))?;
    if stem.len() != 10 {
        return None;
    }
    Some((u8::from_str_radix(&stem[..2], 16).ok()?, u32::from_str_radix(&stem[2..], 16).ok()?))
}

fn parse(data: &[u8], out: &mut HashMap<EKey, Location>) -> Result<(), Error> {
    let bad = Error::Format("bad index file");
    // Header: version 7, then the field sizes this reader assumes.
    let header = data.get(8..24).ok_or(bad.clone())?;
    let version = u16::from_le_bytes([header[0], header[1]]);
    let (size_len, offset_len, key_len, offset_bits) = (header[4], header[5], header[6], header[7]);
    if version != 7 || size_len != 4 || offset_len != 5 || key_len != 9 || !(1..40).contains(&offset_bits) {
        return Err(bad);
    }
    let block = data.get(ENTRIES_BLOCK..ENTRIES_BLOCK + 8).ok_or(bad.clone())?;
    let block_len = u32::from_le_bytes(block[..4].try_into().unwrap()) as usize;
    let start = ENTRIES_BLOCK + 8;
    let entries = data.get(start..start + block_len).ok_or(bad)?;
    let mask = (1u64 << offset_bits) - 1;
    for e in entries.chunks_exact(ENTRY) {
        let key: EKey = e[..9].try_into().unwrap();
        let packed = e[9..14].iter().fold(0u64, |v, &b| v << 8 | b as u64);
        let size = u32::from_le_bytes(e[14..18].try_into().unwrap()) as u64;
        out.insert(key, Location { archive: (packed >> offset_bits) as u32, offset: packed & mask, size });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(parse_name("0700000008.idx"), Some((7, 8)));
        assert_eq!(parse_name("0f0000000a.idx"), Some((15, 10)));
        assert_eq!(parse_name("shmem"), None);
    }

    #[test]
    fn entries() {
        let mut data = vec![0u8; 0x28];
        data[0] = 0x10;
        data[8..16].copy_from_slice(&[7, 0, 0, 0, 4, 5, 9, 30]);
        data[0x20] = ENTRY as u8;
        // Key, then archive 11 at offset 0x3cde6e5e, then size 0x5f68.
        data.extend_from_slice(&[0, 0x0c, 0xa3, 0x80, 0x44, 0x24, 0x63, 0x45, 0x4b]);
        data.extend_from_slice(&[0x02, 0xfc, 0xde, 0x6e, 0x5e, 0x68, 0x5f, 0, 0]);
        let mut out = HashMap::new();
        parse(&data, &mut out).unwrap();
        let at = out[&[0, 0x0c, 0xa3, 0x80, 0x44, 0x24, 0x63, 0x45, 0x4b]];
        assert_eq!(at, Location { archive: 11, offset: 0x3cde6e5e, size: 0x5f68 });
    }
}
