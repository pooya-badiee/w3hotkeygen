//! The INI-like `.txt` files the game keeps its data in (`units\*func.txt`,
//! `*strings.txt`), and `CustomKeys.txt` itself.

use std::collections::HashMap;

/// The field suffix of the current melee data set, lowercased.
const MELEE: &str = "melee,v1";

/// Sections from one or more files, merged. Section names are rawcodes and
/// stay case-sensitive (`ACvs` and `Acvs` are different abilities); field
/// names are case-insensitive.
#[derive(Debug, Default)]
pub struct Ini {
    sections: HashMap<String, HashMap<String, String>>,
}

impl Ini {
    pub fn parse(text: &str) -> Ini {
        let mut ini = Ini::default();
        ini.add(text);
        ini
    }

    /// Merges another file in. A field set twice keeps the later value, so a
    /// section split over two files gets the fields of both.
    ///
    /// A field suffixed `:melee,V1` (current melee data) overrides the plain
    /// one. Other suffixes (`Art:hd`, `Hotkey:melee,V0`) are skipped: they
    /// belong to other graphics modes or legacy data sets.
    pub fn add(&mut self, text: &str) {
        let mut current: Option<&mut HashMap<String, String>> = None;
        for line in text.trim_start_matches('\u{feff}').lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                current = Some(self.sections.entry(name.trim().to_string()).or_default());
            } else if let (Some(fields), Some((key, value))) = (current.as_deref_mut(), line.split_once('=')) {
                let key = key.trim().to_ascii_lowercase();
                let key = match key.split_once(':') {
                    None => key,
                    Some((field, MELEE)) => format!("{field}:{MELEE}"),
                    Some(_) => continue,
                };
                fields.insert(key, unquote(value.trim()).to_string());
            }
        }
    }

    /// A field's value for current melee, `None` if it's missing or empty.
    pub fn get(&self, section: &str, field: &str) -> Option<&str> {
        let fields = self.sections.get(section)?;
        let field = field.to_ascii_lowercase();
        let value = fields.get(&format!("{field}:{MELEE}")).or_else(|| fields.get(&field))?;
        (!value.is_empty()).then_some(value.as_str())
    }

    /// A comma-separated field as a list (`Trains=hfoo,hrif,hkni`).
    pub fn list(&self, section: &str, field: &str) -> Vec<String> {
        split_list(self.get(section, field).unwrap_or(""))
    }

    pub fn sections(&self) -> impl Iterator<Item = &str> {
        self.sections.keys().map(String::as_str)
    }
}

/// Splits `a,b,c`, dropping blanks and the SLK placeholders `_` and `-`.
pub(crate) fn split_list(value: &str) -> Vec<String> {
    value.split(',').map(str::trim).filter(|s| !matches!(*s, "" | "_" | "-")).map(String::from).collect()
}

/// `"text"` -> `text`. Multi-value strings (`"a","b"`) keep their inner quotes.
fn unquote(value: &str) -> &str {
    match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        Some(inner) if !inner.contains('"') => inner,
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FUNC: &str = "\u{feff}// Call to Arms\r\n[Amil]\r\nButtonpos=1,2\r\nUnButtonpos=2,2\r\n\r\n\
                        [hbar]\t\nTrains=hfoo,hrif,hkni\nRequires=\nArt:hd=x.blp\n";
    const STRINGS: &str = "[Amil]\nName=Call to Arms\nHotkey=C\nHotkey:melee,V0=X\n\
                           Tip:melee,V1=Call\nTip=Call to Arms\n\
                           Ubertip=\"Calls the militia.\"\n[ACvs]\nName=Envenomed\n";

    #[test]
    fn parses_sections_and_fields() {
        let ini = Ini::parse(FUNC);
        assert_eq!(ini.get("Amil", "Buttonpos"), Some("1,2"));
        // Field names are case-insensitive.
        assert_eq!(ini.get("Amil", "unbuttonpos"), Some("2,2"));
        // Trailing whitespace after a header is ignored.
        assert_eq!(ini.list("hbar", "Trains"), ["hfoo", "hrif", "hkni"]);
        // Empty values count as missing; `:` variants are skipped.
        assert_eq!(ini.get("hbar", "Requires"), None);
        assert_eq!(ini.get("hbar", "Art"), None);
        assert!(ini.list("hbar", "Builds").is_empty());
    }

    #[test]
    fn merges_files() {
        let mut ini = Ini::parse(FUNC);
        ini.add(STRINGS);
        assert_eq!(ini.get("Amil", "Buttonpos"), Some("1,2"));
        assert_eq!(ini.get("Amil", "Hotkey"), Some("C"));
        assert_eq!(ini.get("Amil", "Ubertip"), Some("Calls the militia."));
        // `:melee,V1` wins over the plain field, whatever the order.
        assert_eq!(ini.get("Amil", "Tip"), Some("Call"));
        // Section names are case-sensitive.
        assert_eq!((ini.get("ACvs", "Name"), ini.get("Acvs", "Name")), (Some("Envenomed"), None));
        ini.add("[Amil]\nHotkey=V\n");
        assert_eq!(ini.get("Amil", "Hotkey"), Some("V"));
    }
}
