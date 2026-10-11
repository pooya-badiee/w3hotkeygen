//! Scraping absolute paths out of text and binary files (Battle.net
//! databases, JSON configs, registry values).

/// Characters that can't appear in a path and end a match.
const TERMINATORS: &[char] = &['"', '<', '>', '|', '*', '?', ','];

/// Splits `bytes` into runs of printable text. Bytes >= 0x80 are kept so
/// UTF-8 paths survive.
fn text_runs(bytes: &[u8]) -> impl Iterator<Item = String> + '_ {
    bytes
        .split(|&b| b < 0x20 || b == 0x7f)
        .filter(|run| run.len() >= 4)
        .map(|run| String::from_utf8_lossy(run).into_owned())
}

/// Windows absolute paths (`C:\...` or `C:/...`). Handles JSON
/// escaping (`\\`, `\/`) and stray bytes before the drive letter, such as
/// protobuf length prefixes in `product.db`.
pub fn windows_paths(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for run in text_runs(bytes) {
        let run = run.replace("\\\\", "\\").replace("\\/", "/");
        let chars: Vec<char> = run.chars().collect();
        let mut i = 0;
        while i + 2 < chars.len() {
            let is_start = chars[i].is_ascii_alphabetic()
                && chars[i + 1] == ':'
                && matches!(chars[i + 2], '\\' | '/')
                && (i == 0 || !chars[i - 1].is_ascii_alphanumeric());
            if !is_start {
                i += 1;
                continue;
            }
            let end = chars[i..].iter().position(|c| TERMINATORS.contains(c)).map_or(chars.len(), |p| i + p);
            let path: String = chars[i..end].iter().collect();
            let path = path.trim_end().trim_end_matches(['\\', '/']);
            if path.len() > 3 {
                out.push(path.to_string());
            }
            i = end;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_db_style_length_prefixes() {
        // Real product.db: protobuf length byte printed right before the path.
        let blob = b"\x0a\x05agent\x12!C:/Program Files (x86)/Battle.net\x1a\x04enUS\
                     \x22$C:/Program Files (x86)/Warcraft III\x2a\x02xx";
        assert_eq!(
            windows_paths(blob),
            ["C:/Program Files (x86)/Battle.net", "C:/Program Files (x86)/Warcraft III"]
        );
    }

    #[test]
    fn json_escaped_paths() {
        let json = br#"{"DefaultInstallPath": "C:\/Program Files (x86)", "icon_path": "D:\\Games\\Warcraft III\\Warcraft III Launcher.exe"}"#;
        assert_eq!(
            windows_paths(json),
            ["C:/Program Files (x86)", r"D:\Games\Warcraft III\Warcraft III Launcher.exe"]
        );
    }

    #[test]
    fn escaped_path_with_trailing_separator() {
        let reg = br#""InstallPath"="C:\\Program Files (x86)\\Warcraft III\\""#;
        assert_eq!(windows_paths(reg), [r"C:\Program Files (x86)\Warcraft III"]);
    }

    #[test]
    fn ignores_non_paths() {
        assert!(windows_paths(b"geoip-CA? acct:US? http://x.com/a").is_empty());
    }
}
