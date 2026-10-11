//! A read-only CASC storage for Warcraft III (Reforged), in pure Rust.
//!
//! Warcraft III names every file through a TVFS tree, whose entries point
//! straight at encoded keys, so the encoding table is never needed:
//! `.build.info` → build config → `vfs-root` → local index → `data.###` → BLTE.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::blte;
use crate::index::{self, Location};
use crate::tvfs::{self, Dir};
use crate::{EKey, Error, Source};

/// An open storage over any [`Source`].
pub struct Casc<S> {
    source: S,
    index: HashMap<EKey, Location>,
    /// Encoded keys of every `vfs-N` tree; an entry with one of these keys is a
    /// nested tree (`war3.w3mod`), not a file.
    vfs: HashSet<EKey>,
    root: Arc<Dir>,
    nested: Mutex<HashMap<EKey, Arc<Dir>>>,
}

impl<S: Source> Casc<S> {
    pub async fn open(source: S) -> Result<Self, Error> {
        let build_info = text(source.read(".build.info", 0, None).await?);
        let build_key = active_build_key(&build_info).ok_or(Error::Format(".build.info has no active build"))?;
        let config = text(source.read(&config_path(&build_key), 0, None).await?);
        let (root_key, vfs) = vfs_keys(&config)?;
        let index = index::load(&source).await?;
        let mut casc = Casc { source, index, vfs, root: Arc::default(), nested: Mutex::default() };
        casc.root = Arc::new(casc.load_dir(&root_key).await?);
        Ok(casc)
    }

    /// A whole file, by name, e.g. `war3.w3mod:units\humanunitfunc.txt`.
    /// Names ignore case, and `\` and `/` are the same.
    pub async fn read(&self, name: &str) -> Result<Vec<u8>, Error> {
        let name = tvfs::normalize(name);
        let mut dir = self.root.clone();
        let mut rest = name.as_str();
        loop {
            if let Some(key) = dir.get(rest) {
                return self.read_key(&key).await;
            }
            // `a:b` is file `b` inside the nested tree `a`.
            let (head, tail) = rest.split_once(':').ok_or(Error::NotFound)?;
            let key = dir.get(head).filter(|k| self.vfs.contains(k)).ok_or(Error::NotFound)?;
            dir = self.nested_dir(&key).await?;
            rest = tail;
        }
    }

    /// Names of the files under `prefix`, normalized, e.g. `war3.w3mod:units/`
    /// lists the files in `war3.w3mod` whose path starts with `units/`.
    pub async fn list(&self, prefix: &str) -> Result<Vec<String>, Error> {
        let prefix = tvfs::normalize(prefix);
        let (trees, last) = prefix.rsplit_once(':').map_or(("", prefix.as_str()), |(t, l)| (t, l));
        let mut dir = self.root.clone();
        for tree in trees.split(':').filter(|t| !t.is_empty()) {
            let key = dir.get(tree).filter(|k| self.vfs.contains(k)).ok_or(Error::NotFound)?;
            dir = self.nested_dir(&key).await?;
        }
        let tree_prefix = if trees.is_empty() { String::new() } else { format!("{trees}:") };
        Ok(dir
            .entries()
            .filter(|(name, key)| name.starts_with(last) && !self.vfs.contains(*key))
            .map(|(name, _)| format!("{tree_prefix}{name}"))
            .collect())
    }

    async fn nested_dir(&self, key: &EKey) -> Result<Arc<Dir>, Error> {
        if let Some(dir) = self.nested.lock().unwrap().get(key) {
            return Ok(dir.clone());
        }
        let dir = Arc::new(self.load_dir(key).await?);
        self.nested.lock().unwrap().insert(*key, dir.clone());
        Ok(dir)
    }

    async fn load_dir(&self, key: &EKey) -> Result<Dir, Error> {
        tvfs::parse(&self.read_key(key).await?)
    }

    async fn read_key(&self, key: &EKey) -> Result<Vec<u8>, Error> {
        let at = self.index.get(key).ok_or(Error::NotFound)?;
        let raw = self.source.read(&index::archive_path(at.archive), at.offset, Some(at.size)).await?;
        blte::decode(raw.get(blte::LOCAL_HEADER..).ok_or(Error::Format("data entry too short"))?)
    }
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8_lossy(&bytes).into_owned()
}

/// `.build.info` is a `|`-separated table whose header names the columns,
/// e.g. `Build Key!HEX:16`. One row per region; the active one is used.
fn active_build_key(info: &str) -> Option<String> {
    let mut lines = info.lines().filter(|l| !l.trim().is_empty());
    let header: Vec<&str> = lines.next()?.split('|').map(|c| c.split('!').next().unwrap_or("")).collect();
    let col = |name: &str| header.iter().position(|c| c.eq_ignore_ascii_case(name));
    let (key, active) = (col("Build Key")?, col("Active"));
    let rows: Vec<Vec<&str>> = lines.map(|l| l.split('|').collect()).collect();
    let row = rows
        .iter()
        .find(|r| active.is_some_and(|a| r.get(a) == Some(&"1")))
        .or(rows.first())?;
    row.get(key).map(|k| k.trim().to_ascii_lowercase()).filter(|k| k.len() == 32)
}

fn config_path(key: &str) -> String {
    format!("Data/config/{}/{}/{key}", &key[0..2], &key[2..4])
}

/// The root tree's encoded key and the encoded keys of all `vfs-N` trees.
/// Lines look like `vfs-root = <content key> <encoded key>`.
fn vfs_keys(config: &str) -> Result<(EKey, HashSet<EKey>), Error> {
    let mut root = None;
    let mut all = HashSet::new();
    for line in config.lines() {
        let Some((name, value)) = line.split_once('=') else { continue };
        let name = name.trim();
        if name != "vfs-root" && !(name.starts_with("vfs-") && name[4..].bytes().all(|b| b.is_ascii_digit())) {
            continue;
        }
        let key = value.split_whitespace().nth(1).and_then(parse_ekey).ok_or(Error::Format("bad vfs key in build config"))?;
        if name == "vfs-root" {
            root = Some(key);
        } else {
            all.insert(key);
        }
    }
    Ok((root.ok_or(Error::Format("build config has no vfs-root"))?, all))
}

/// The first 9 bytes of a hex key: what indexes and TVFS store.
fn parse_ekey(hex: &str) -> Option<EKey> {
    let mut key = [0; 9];
    for (i, b) in key.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_active_build() {
        let info = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16\n\
                    eu|0|00000000000000000000000000000000\n\
                    us|1|3A9D8F26806936764D2D9AD526A65E04\n";
        assert_eq!(active_build_key(info).as_deref(), Some("3a9d8f26806936764d2d9ad526a65e04"));
        assert_eq!(config_path("3a9d8f26806936764d2d9ad526a65e04"), "Data/config/3a/9d/3a9d8f26806936764d2d9ad526a65e04");
    }

    #[test]
    fn reads_vfs_keys() {
        let config = "root = aa\nvfs-root = 3ab870666612150386f64ecfa5687094 132daa60e6ad302091b3b3fb6d6c2927\n\
                      vfs-root-size = 1 2\nvfs-1 = 3ab870666612150386f64ecfa5687094 132daa60e6ad302091b3b3fb6d6c2927\n\
                      vfs-2 = dec901636e6c0460e1eda61bb7fa11fa ec9836ae111b50f13e01c5967c2f4c28\n";
        let (root, all) = vfs_keys(config).unwrap();
        assert_eq!(root, [0x13, 0x2d, 0xaa, 0x60, 0xe6, 0xad, 0x30, 0x20, 0x91]);
        assert_eq!(all.len(), 2);
    }
}
