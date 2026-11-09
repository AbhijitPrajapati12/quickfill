//! The saved items and settings, persisted as DPAPI-encrypted JSON.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{crypto, totp};

pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+Space";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub kind: ItemKind,
    /// The 2FA secret of a `PasswordTotp` item (its `value` is the password).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

/// `Text` is a one-line value (a link, an id); `Template` is free-form
/// multi-line text such as an email or notes; `Totp` holds a 2FA secret and
/// pastes the current one-time code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    #[default]
    Text,
    Template,
    Totp,
    /// A password plus a 2FA secret; pastes the password followed directly by
    /// the current code (what most VPN logins expect).
    PasswordTotp,
}

/// The text to paste for an item: its value, or for 2FA items the current code.
pub fn output(item: &Item) -> Result<String, String> {
    match item.kind {
        ItemKind::Totp => totp::now(&item.value).map(|(code, ..)| code),
        ItemKind::PasswordTotp => {
            let secret = item.secret.as_deref().unwrap_or_default();
            totp::now(secret).map(|(code, ..)| format!("{}{code}", item.value))
        }
        _ => Ok(item.value.clone()),
    }
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
        Data {
            version: 1,
            items: vec![Item {
                id: new_id(),
                label: "GitHub".into(),
                value: "https://github.com/".into(),
                code: Some(":gh".into()),
                kind: ItemKind::Text,
                secret: None,
            }],
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

/// Every short code starts with this, so codes never fire inside ordinary
/// words (a plain `gh` would expand while typing "high" or "though").
pub const CODE_PREFIX: char = ':';

/// Adds the `:` prefix to codes saved before it was required. A code is left
/// as it was when the prefixed form is already taken, so the user can rename
/// one of them. Returns whether anything changed.
pub fn migrate_codes(items: &mut [Item]) -> bool {
    let mut taken: HashSet<String> = items.iter().filter_map(|i| i.code.clone()).collect();
    let mut changed = false;
    for item in items.iter_mut() {
        let Some(code) = &item.code else { continue };
        if code.starts_with(CODE_PREFIX) {
            continue;
        }
        let prefixed = format!("{CODE_PREFIX}{code}");
        if !taken.contains(&prefixed) {
            taken.insert(prefixed.clone());
            item.code = Some(prefixed);
            changed = true;
        }
    }
    changed
}

/// Trims fields, turns blank codes into `None`, makes sure codes start with
/// `:` and fills in missing ids.
pub fn normalize(items: Vec<Item>) -> Vec<Item> {
    items
        .into_iter()
        .map(|mut item| {
            item.label = item.label.trim().to_string();
            item.code = item
                .code
                .map(|c| c.trim().trim_start_matches(CODE_PREFIX).to_string())
                .filter(|c| !c.is_empty())
                .map(|c| format!("{CODE_PREFIX}{c}"));
            match item.kind {
                // A one-line field can't hold line breaks; drop any that slipped in.
                ItemKind::Text => item.value = item.value.replace(['\r', '\n'], ""),
                ItemKind::Totp => item.value = item.value.trim().to_string(),
                ItemKind::PasswordTotp => {
                    // Passwords keep their spaces; only line breaks are dropped.
                    item.value = item.value.replace(['\r', '\n'], "");
                }
                ItemKind::Template => {}
            }
            item.secret = match item.kind {
                ItemKind::PasswordTotp => item.secret.map(|s| s.trim().to_string()),
                _ => None,
            };
            if item.id.is_empty() {
                item.id = new_id();
            }
            item
        })
        .collect()
}

/// Checks labels and short codes. Codes are `:` plus 1–15 characters without
/// spaces, unique, and no code may be the start of another (it would always
/// fire first).
pub fn validate(items: &[Item]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for item in items {
        if item.label.is_empty() {
            return Err("Every item needs a label.".into());
        }
        if item.kind == ItemKind::Totp {
            totp::parse(&item.value).map_err(|e| format!("{}: {e}", item.label))?;
        }
        if item.kind == ItemKind::PasswordTotp {
            if item.value.is_empty() {
                return Err(format!("{}: enter the password.", item.label));
            }
            totp::parse(item.secret.as_deref().unwrap_or_default())
                .map_err(|e| format!("{}: {e}", item.label))?;
        }
        let Some(code) = &item.code else { continue };
        let Some(rest) = code.strip_prefix(CODE_PREFIX) else {
            return Err(format!("{}: short code must start with {CODE_PREFIX}", item.label));
        };
        let len = rest.chars().count();
        if !(1..=15).contains(&len) {
            return Err(format!(
                "{}: short code needs 1–15 characters after the {CODE_PREFIX}",
                item.label
            ));
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
            kind: ItemKind::Text,
            secret: None,
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
    fn validates_totp_secrets() {
        let mut good = item("Bank", Some(":2fa"));
        good.kind = ItemKind::Totp;
        good.value = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into();
        assert!(validate(&[good.clone()]).is_ok());
        assert_eq!(output(&good).unwrap().len(), 6);
        let mut bad = good;
        bad.value = "not a key!".into();
        assert!(validate(&[bad]).is_err());
    }

    #[test]
    fn validates_password_totp() {
        let mut vpn = item("VPN", Some(":vpn"));
        vpn.kind = ItemKind::PasswordTotp;
        vpn.value = "hunter2".into();
        vpn.secret = Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into());
        assert!(validate(&[vpn.clone()]).is_ok());
        let out = output(&vpn).unwrap();
        assert!(out.starts_with("hunter2") && out.len() == "hunter2".len() + 6);

        let mut no_password = vpn.clone();
        no_password.value.clear();
        assert!(validate(&[no_password]).is_err());
        let mut no_secret = vpn.clone();
        no_secret.secret = None;
        assert!(validate(&[no_secret]).is_err());
    }

    #[test]
    fn requires_colon_prefix() {
        assert!(validate(&[item("A", Some("gh"))]).is_err());
        assert!(validate(&[item("A", Some(":g"))]).is_ok());
        assert!(validate(&[item("A", Some(":123456789012345"))]).is_ok()); // 15 after ":"
        assert!(validate(&[item("A", Some(":1234567890123456"))]).is_err()); // 16 after ":"
    }

    #[test]
    fn normalize_adds_colon_once() {
        let out = normalize(vec![
            item("A", Some("gh")),
            item("B", Some(" ::li ")),
            item("C", Some(":x")),
            item("D", Some(":")),
        ]);
        let codes: Vec<_> = out.iter().map(|i| i.code.as_deref()).collect();
        assert_eq!(codes, [Some(":gh"), Some(":li"), Some(":x"), None]);
    }

    #[test]
    fn migrate_prefixes_old_codes_but_not_clashes() {
        let mut items = vec![
            item("GitHub", Some("gh")),
            item("LinkedIn", Some("li")),
            item("Existing", Some(":li")),
            item("NoCode", None),
        ];
        assert!(migrate_codes(&mut items));
        let codes: Vec<_> = items.iter().map(|i| i.code.as_deref()).collect();
        // "li" stays as it was because ":li" is already taken; the user renames one.
        assert_eq!(codes, [Some(":gh"), Some("li"), Some(":li"), None]);
        assert!(!migrate_codes(&mut items[..1]), "already prefixed codes are left alone");
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
