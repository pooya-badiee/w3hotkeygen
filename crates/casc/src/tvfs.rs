//! TVFS trees: the names of every file. A tree's path table is a prefix tree
//! of name fragments; leaves point through the VFS table and the container
//! file table (CFT) at an encoded key. A leaf can be another tree, which is
//! how `war3.w3mod:units\...` names work.

use std::collections::HashMap;

use crate::{EKey, Error};

/// One tree, flattened: normalized path → encoded key.
#[derive(Debug, Default)]
pub struct Dir(HashMap<String, EKey>);

impl Dir {
    pub fn get(&self, normalized: &str) -> Option<EKey> {
        self.0.get(normalized).copied()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &EKey)> {
        self.0.iter().map(|(name, key)| (name.as_str(), key))
    }
}

/// Lowercase, `/` separators: how [`Dir`] keys are stored.
pub fn normalize(name: &str) -> String {
    name.chars().map(|c| if c == '\\' { '/' } else { c.to_ascii_lowercase() }).collect()
}

const FOLDER: u32 = 0x8000_0000;

struct Tables<'a> {
    data: &'a [u8],
    vfs: (usize, usize),
    cft: (usize, usize),
    cft_offset_len: usize,
}

pub fn parse(data: &[u8]) -> Result<Dir, Error> {
    let bad = Error::Format("bad TVFS tree");
    if data.get(..4) != Some(b"TVFS") || data.get(4) != Some(&1) || data.get(6) != Some(&9) {
        return Err(bad);
    }
    let field = |i: usize| be32(data, 12 + i * 4).map(|v| v as usize);
    let (path_at, path_len) = (field(0)?, field(1)?);
    let tables = Tables {
        data,
        vfs: (field(2)?, field(3)?),
        cft: (field(4)?, field(5)?),
        cft_offset_len: offset_len(field(5)?),
    };
    let mut start = path_at;
    let mut end = path_at + path_len;
    if end > data.len() {
        return Err(bad);
    }
    // The table usually starts with a root folder node.
    if data.get(start) == Some(&0xff) {
        let value = be32(data, start + 1)?;
        if value & FOLDER == 0 {
            return Err(bad);
        }
        end = (start + 1 + (value & !FOLDER) as usize).min(end);
        start += 5;
    }
    let mut dir = HashMap::new();
    walk(&tables, start, end, &mut String::new(), &mut dir)?;
    Ok(Dir(dir))
}

/// How many bytes a CFT offset takes in a VFS span, from the CFT's size.
fn offset_len(cft_len: usize) -> usize {
    match cft_len {
        0x100_0000.. => 4,
        0x1_0000.. => 3,
        0x100.. => 2,
        _ => 1,
    }
}

/// Each entry: optional `0` (separator before), length + name, optional `0`
/// (separator after), then optional `0xff` + node value. A value with the top
/// bit set is a folder whose entries follow; otherwise it's a VFS offset.
/// Fragments without a value are prefixes of the entries after them.
fn walk(t: &Tables, mut at: usize, end: usize, path: &mut String, out: &mut HashMap<String, EKey>) -> Result<(), Error> {
    let bad = || Error::Format("bad TVFS path table");
    let saved = path.len();
    while at < end {
        let byte = |i: usize| t.data.get(i).copied().ok_or_else(bad);
        if byte(at)? == 0 {
            path.push('/');
            at += 1;
        }
        if at < end && byte(at)? != 0xff {
            let len = byte(at)? as usize;
            let name = t.data.get(at + 1..at + 1 + len).ok_or_else(bad)?;
            path.push_str(&normalize(&String::from_utf8_lossy(name)));
            at += 1 + len;
        }
        // A `0`, or anything but `0xff`, after the name means a separator.
        if at < end && byte(at)? == 0 {
            at += 1;
            path.push('/');
            if at < end && byte(at)? != 0xff {
                continue;
            }
        } else if at < end && byte(at)? != 0xff {
            path.push('/');
            continue;
        }
        if at >= end {
            break;
        }
        let value = be32(t.data, at + 1)?;
        at += 5;
        if value & FOLDER != 0 {
            let folder_end = at + (value & !FOLDER) as usize - 4;
            if folder_end > end {
                return Err(bad());
            }
            walk(t, at, folder_end, path, out)?;
            at = folder_end;
        } else if let Some(key) = single_span_key(t, value as usize)? {
            out.insert(path.clone(), key);
        }
        path.truncate(saved);
    }
    Ok(())
}

/// The encoded key of a one-span file. Deleted and multi-span entries are
/// skipped: Warcraft III doesn't use them for anything this app reads.
fn single_span_key(t: &Tables, vfs_offset: usize) -> Result<Option<EKey>, Error> {
    let bad = || Error::Format("bad TVFS span");
    if vfs_offset >= t.vfs.1 {
        return Err(bad());
    }
    let at = t.vfs.0 + vfs_offset;
    if t.data.get(at).copied().ok_or_else(bad)? != 1 {
        return Ok(None);
    }
    // File offset (4), span size (4), CFT offset.
    let field = t.data.get(at + 9..at + 9 + t.cft_offset_len).ok_or_else(bad)?;
    let cft_offset = field.iter().fold(0usize, |v, &b| v << 8 | b as usize);
    if cft_offset + 9 > t.cft.1 {
        return Err(bad());
    }
    let key = t.data.get(t.cft.0 + cft_offset..t.cft.0 + cft_offset + 9).ok_or_else(bad)?;
    Ok(Some(key.try_into().unwrap()))
}

fn be32(data: &[u8], at: usize) -> Result<u32, Error> {
    let bytes = data.get(at..at + 4).ok_or(Error::Format("TVFS too short"))?;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_names() {
        assert_eq!(normalize(r"war3.w3mod:Units\HumanUnitFunc.txt"), "war3.w3mod:units/humanunitfunc.txt");
    }

    #[test]
    fn offset_sizes() {
        assert_eq!(offset_len(0xff), 1);
        assert_eq!(offset_len(0x100), 2);
        assert_eq!(offset_len(0x1_0000), 3);
        assert_eq!(offset_len(0x100_0000), 4);
    }
}
