//! SYLK spreadsheets (`units\*.slk`). Only the parts the game's files use:
//! `C` records with `X`/`Y` coordinates and a `K` value. The first row holds
//! the column names, the first column the row IDs.

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Slk {
    columns: HashMap<String, usize>,
    rows: HashMap<String, Vec<String>>,
}

impl Slk {
    pub fn parse(text: &str) -> Slk {
        let mut cells: HashMap<usize, Vec<String>> = HashMap::new();
        let (mut x, mut y) = (1, 1);
        for line in text.lines() {
            let fields = split_record(line);
            let Some((kind, fields)) = fields.split_first() else { continue };
            if kind != "C" && kind != "F" {
                continue;
            }
            let mut value = None;
            for field in fields {
                let mut chars = field.chars();
                let tag = chars.next();
                let rest = chars.as_str();
                match tag {
                    Some('X') => x = rest.parse().unwrap_or(x),
                    Some('Y') => y = rest.parse().unwrap_or(y),
                    Some('K') if kind == "C" => value = Some(rest.trim_matches('"').to_string()),
                    _ => {}
                }
            }
            if let (Some(value), true) = (value, x >= 1) {
                let row = cells.entry(y).or_default();
                if row.len() < x {
                    row.resize(x, String::new());
                }
                row[x - 1] = value;
            }
        }
        let mut slk = Slk::default();
        if let Some(header) = cells.remove(&1) {
            slk.columns = header.into_iter().enumerate().map(|(i, name)| (name, i)).collect();
        }
        for row in cells.into_values() {
            if let Some(id) = row.first().filter(|id| !id.is_empty()) {
                slk.rows.insert(id.clone(), row);
            }
        }
        slk
    }

    /// A cell's value, `None` if it's missing, empty, `_` or `-`.
    pub fn get(&self, id: &str, column: &str) -> Option<&str> {
        let value = self.rows.get(id)?.get(*self.columns.get(column)?)?;
        (!matches!(value.as_str(), "" | "_" | "-")).then_some(value.as_str())
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.rows.keys().map(String::as_str)
    }

    pub fn columns(&self) -> impl Iterator<Item = &str> {
        self.columns.keys().map(String::as_str)
    }
}

/// Splits a record on `;`. A doubled `;;` is a literal semicolon.
fn split_record(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut chars = line.trim_end_matches(['\r', '\n']).chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ';' if chars.peek() == Some(&';') => {
                chars.next();
                fields.last_mut().unwrap().push(';');
            }
            ';' => fields.push(String::new()),
            c => fields.last_mut().unwrap().push(c),
        }
    }
    fields
}

/// SLK text from rows of cells, for tests; the first row holds the column names.
#[cfg(test)]
pub(crate) fn sheet(rows: &[&[&str]]) -> String {
    let mut out = String::from("ID;PWXL;N;E\n");
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            out += &format!("C;X{};Y{};K\"{cell}\"\n", x + 1, y + 1);
        }
    }
    out + "E\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "ID;PWXL;N;E\r\nB;X4;Y3;D0\r\n\
                          C;X1;Y1;K\"unitAbilID\"\r\nC;X2;K\"abilList\"\r\nC;X3;K\"heroAbilList\"\r\n\
                          C;X1;Y2;K\"Hamg\"\r\nC;X2;K\"AInv\"\r\nC;X3;K\"AHbz,AHab,AHwe,AHmt\"\r\n\
                          C;X1;Y3;K\"hfoo\"\r\nC;X2;K\"Adef;;x\"\r\nC;X3;K\"_\"\r\nF;X4;Y3;FG0G\r\nC;K1\r\nE\r\n";

    #[test]
    fn parses_cells() {
        let slk = Slk::parse(SAMPLE);
        assert_eq!(slk.get("Hamg", "heroAbilList"), Some("AHbz,AHab,AHwe,AHmt"));
        // Y carries over from the previous record.
        assert_eq!(slk.get("Hamg", "abilList"), Some("AInv"));
        assert_eq!(slk.get("hfoo", "abilList"), Some("Adef;x"));
        // Placeholders read as missing.
        assert_eq!(slk.get("hfoo", "heroAbilList"), None);
        assert_eq!(slk.get("hpea", "abilList"), None);
        assert_eq!(slk.ids().count(), 2);
    }
}
