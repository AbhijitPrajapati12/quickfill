//! The saved items and settings, persisted as DPAPI-encrypted JSON.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::crypto;

pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+Space";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "default_true")]
    pub codes_enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Data {
    pub version: u32,
    pub items: Vec<Item>,
    pub settings: Settings,
}

impl Default for Data {
    fn default() -> Self {
        let item = |label: &str, value: &str, code: &str| Item {
            id: new_id(),
            label: label.into(),
            value: value.into(),
            code: Some(code.into()),
        };
        Data {
            version: 1,
            items: vec![
                item("GitHub", "https://github.com/", ":gh"),
                item("LinkedIn", "https://www.linkedin.com/in/", ":li"),
                item("X", "https://x.com/", ":x"),
            ],
            settings: Settings {
                hotkey: DEFAULT_HOTKEY.into(),
                autostart: false,
                codes_enabled: true,
            },
        }
    }
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Loads the data file. Returns `(data, first_run)`. A file that can't be read
/// is moved aside to `data.bin.bad` rather than silently overwritten.
pub fn load(path: &Path) -> (Data, bool) {
    if !path.exists() {
        return (Data::default(), true);
    }
    let parsed = fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| crypto::unprotect(&bytes))
        .and_then(|json| serde_json::from_slice::<Data>(&json).map_err(|e| e.to_string()));
    match parsed {
        Ok(data) => (data, false),
        Err(e) => {
            eprintln!("QuickFill: couldn't read {}: {e}", path.display());
            let _ = fs::rename(path, path.with_extension("bin.bad"));
            (Data::default(), true)
        }
    }
}

/// Saves atomically: write a temp file, then rename over the real one.
pub fn save(path: &Path, data: &Data) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec(data).map_err(|e| e.to_string())?;
    let encrypted = crypto::protect(&json)?;
    let tmp = path.with_extension("bin.tmp");
    fs::write(&tmp, encrypted).map_err(|e| format!("Couldn't write data file: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("Couldn't replace data file: {e}"))
}

/// Trims fields, turns blank codes into `None` and fills in missing ids.
pub fn normalize(items: Vec<Item>) -> Vec<Item> {
    items
        .into_iter()
        .map(|mut item| {
            item.label = item.label.trim().to_string();
            item.code = item
                .code
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty());
            if item.id.is_empty() {
                item.id = new_id();
            }
            item
        })
        .collect()
}

/// Checks labels and short codes. Codes must be 2–16 characters without spaces,
/// unique, and no code may be the start of another (it would always fire first).
pub fn validate(items: &[Item]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for item in items {
        if item.label.is_empty() {
            return Err("Every item needs a label.".into());
        }
        let Some(code) = &item.code else { continue };
        let len = code.chars().count();
        if !(2..=16).contains(&len) {
            return Err(format!("{}: short code must be 2–16 characters.", item.label));
        }
        if code.chars().any(char::is_whitespace) {
            return Err(format!("{}: short code can't contain spaces.", item.label));
        }
        if !seen.insert(code.as_str()) {
            return Err(format!("Short code {code} is used more than once."));
        }
    }
    for a in items {
        for b in items {
            if let (Some(ca), Some(cb)) = (&a.code, &b.code) {
                if a.id != b.id && cb.len() > ca.len() && cb.starts_with(ca.as_str()) {
                    return Err(format!(
                        "{ca} ({}) is the start of {cb} ({}), so {cb} could never be typed. Pick a different code.",
                        a.label, b.label
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str, code: Option<&str>) -> Item {
        Item {
            id: new_id(),
            label: label.into(),
            value: "v".into(),
            code: code.map(Into::into),
        }
    }

    #[test]
    fn accepts_defaults() {
        assert!(validate(&Data::default().items).is_ok());
    }

    #[test]
    fn rejects_prefix_codes() {
        let items = vec![item("A", Some(":g")), item("B", Some(":gh"))];
        assert!(validate(&items).is_err());
    }

    #[test]
    fn rejects_duplicates_spaces_and_length() {
        assert!(validate(&[item("A", Some(":gh")), item("B", Some(":gh"))]).is_err());
        assert!(validate(&[item("A", Some(": g"))]).is_err());
        assert!(validate(&[item("A", Some(":"))]).is_err());
        assert!(validate(&[item("", None)]).is_err());
    }

    #[test]
    fn normalize_blanks_codes_and_ids() {
        let mut raw = item(" A ", Some("  "));
        raw.id = String::new();
        let out = normalize(vec![raw]);
        assert_eq!(out[0].label, "A");
        assert_eq!(out[0].code, None);
        assert!(!out[0].id.is_empty());
    }
}
