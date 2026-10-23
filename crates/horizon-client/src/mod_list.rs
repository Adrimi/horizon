//! Which of the installed Horizon mods are on (`docs/mods.md`), kept in
//! `horizon-mods.txt` beside `options.txt` as `mod.<id>:true` lines. Every
//! mod is off until the player turns it on in Options > Mods.
//!
//! The mods' own key bindings live in the same file, as
//! `key.<id>.<action>:<key>` lines (API 0.1.4).
use std::{collections::BTreeMap, collections::BTreeSet, fs, path::Path};

pub const FILE: &str = "horizon-mods.txt";
/// The folder holding `.hmod` files, beside `options.txt`.
pub const DIR: &str = "horizon-mods";
/// Where the mods keep their own files (`storage`), beside `options.txt`.
pub const DATA_DIR: &str = "horizon-mod-data";

/// One of a mod's key bindings: the key it asked for and the one in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModBind {
    /// The key the mod asked for.
    pub default_key: String,
    /// The key in use now: the player's, or the default.
    pub key: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModList {
    /// Ids of the mods the player turned on.
    pub enabled: BTreeSet<String>,
    /// What the Mods page shows, filled by the game from the mods it found.
    pub installed: Vec<Installed>,
    /// The mods' key bindings: (mod id, action) -> the binding.
    pub binds: BTreeMap<(String, String), ModBind>,
}

/// One installed mod as the Mods page shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub id: String,
    /// `Name 1.0` (the file name when the manifest is unreadable).
    pub title: String,
    pub description: String,
    /// Why it is not running, when it failed.
    pub problem: Option<String>,
    /// Loaded with `--dev-mod`: always on, reloaded when rebuilt.
    pub dev: bool,
}

impl ModList {
    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(dir.join(FILE))
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }
    pub fn parse(text: &str) -> Self {
        let mut enabled = BTreeSet::new();
        let mut binds = BTreeMap::new();
        for line in text.lines() {
            let Some((k, v)) = line.split_once(':') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            if let Some(id) = k.strip_prefix("mod.") {
                if v == "true" {
                    enabled.insert(id.to_owned());
                }
            } else if let Some(rest) = k.strip_prefix("key.") {
                // `key.<id>.<action>:<key>`
                let Some((id, action)) = rest.split_once('.') else {
                    continue;
                };
                if id.is_empty() || action.is_empty() || v.is_empty() {
                    continue;
                }
                binds.insert(
                    (id.to_owned(), action.to_owned()),
                    ModBind {
                        default_key: v.to_owned(),
                        key: v.to_owned(),
                    },
                );
            }
        }
        Self {
            enabled,
            installed: Vec::new(),
            binds,
        }
    }
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for id in &self.enabled {
            out.push_str(&format!("mod.{id}:true\n"));
        }
        for ((id, action), bind) in &self.binds {
            out.push_str(&format!("key.{id}.{action}:{}\n", bind.key));
        }
        out
    }
    pub fn save(&self, dir: &Path) {
        let path = dir.join(FILE);
        if self.enabled.is_empty() && self.binds.is_empty() && !path.exists() {
            return;
        }
        let _ = fs::write(path, self.serialize());
    }
    pub fn is_on(&self, i: usize) -> bool {
        self.installed
            .get(i)
            .is_some_and(|m| m.dev || self.enabled.contains(&m.id))
    }
    /// Turns installed mod `i` on or off (a dev mod stays on).
    pub fn toggle(&mut self, i: usize) {
        let Some(m) = self.installed.get(i).filter(|m| !m.dev) else {
            return;
        };
        if !self.enabled.remove(&m.id) {
            self.enabled.insert(m.id.clone());
        }
    }
    /// The key the player bound to the mod's action; none when the action
    /// is unknown.
    pub fn key_of(&self, id: &str, action: &str) -> Option<&str> {
        self.binds
            .get(&(id.to_owned(), action.to_owned()))
            .map(|b| b.key.as_str())
    }
    /// The key in use for the mod's action, its default when the player
    /// changed nothing.
    pub fn key_or_default(&self, id: &str, action: &str, default_key: &str) -> String {
        self.key_of(id, action).unwrap_or(default_key).to_owned()
    }
    /// Binds the mod's action to a key (the player did).
    pub fn set_key(&mut self, id: &str, action: &str, key: &str) {
        let entry = self
            .binds
            .entry((id.to_owned(), action.to_owned()))
            .or_insert_with(|| ModBind {
                default_key: key.to_owned(),
                key: key.to_owned(),
            });
        entry.key = key.to_owned();
    }
    /// The mods' bindings, as the Controls page shows them: the mod's id,
    /// the action and the binding.
    pub fn bind_rows(&self) -> Vec<(&str, &str, &ModBind)> {
        self.binds
            .iter()
            .map(|((id, action), bind)| (id.as_str(), action.as_str(), bind))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_ignores_the_rest() {
        let list = ModList::parse("mod.coords_hud:true\nmod.other:false\nzoom:true\n");
        assert_eq!(list.enabled.iter().collect::<Vec<_>>(), ["coords_hud"]);
        assert_eq!(ModList::parse(&list.serialize()), list);
        assert_eq!(ModList::default().serialize(), "");
    }

    #[test]
    fn toggles_installed_mods_but_not_dev_ones() {
        let installed = |id: &str, dev| Installed {
            id: id.into(),
            title: id.into(),
            description: String::new(),
            problem: None,
            dev,
        };
        let mut list = ModList {
            installed: vec![installed("a", false), installed("b", true)],
            ..Default::default()
        };
        assert!(!list.is_on(0) && list.is_on(1));
        list.toggle(0);
        list.toggle(1);
        assert!(list.is_on(0) && list.is_on(1));
        assert_eq!(list.serialize(), "mod.a:true\n");
        list.toggle(0);
        assert!(!list.is_on(0));
    }
}
