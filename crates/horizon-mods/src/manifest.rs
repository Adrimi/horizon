//! `horizon-mod.toml`, a mod's manifest, and the `.hmod` file around it.
use serde::Deserialize;
use std::{fmt, io::Read, path::Path};

/// The manifest's file name, beside `mod.wasm`.
pub const MANIFEST: &str = "horizon-mod.toml";
/// The component's file name in a `.hmod` or a mod folder.
pub const WASM: &str = "mod.wasm";
/// The API (WIT package `horizon:mods`) versions this build runs: a mod
/// built for any 0.1.x SDK.
pub const API: &str = "0.1";
/// The manifest `api` lines this build loads. 0.2 is 0.1 plus content:
/// `assets/` and `data/` in the `.hmod`, with blocks and items declared in
/// `data/<id>/horizon/` (docs/modpacks.md). Code still links API 0.1.
pub const APIS: [&str; 2] = [API, "0.2"];
/// Namespaces no mod may claim: vanilla's, the convention tags' (mods add
/// to those, docs/mods.md) and the game's own.
const RESERVED_NAMESPACES: [&str; 3] = ["minecraft", "c", "horizon"];
/// The biggest `mod.wasm` a mod may ship.
const MAX_WASM: u64 = 32 << 20;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// `[a-z0-9_]`, 1 to 64 characters: the mod's folder, log and settings key.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    /// The API line the mod was built for: `0.1`, or `0.2` for content.
    pub api: String,
    #[serde(default)]
    pub side: Side,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Namespaces besides `id` that the mod's `assets/` and `data/` use, for
    /// a content mod whose files sit under other names than its id (Towns
    /// and Towers ships `kaisyn` and `towns_and_towers`). No two content mods
    /// that are on may share one (`content::clash`).
    #[serde(default)]
    pub namespaces: Vec<String>,
    /// This mod replaces vanilla's world generation data: files under
    /// `data/minecraft/worldgen/` and `data/minecraft/dimension/`, such as
    /// the overworld's noise settings, density functions, surface rules,
    /// biomes and the dimension's biome distribution. Off by default: a mod
    /// otherwise adds files only in its own namespaces and in tags. A mod
    /// that sets it changes every world generated with it on, so the game
    /// names it on the Mods page (docs/mods.md).
    #[serde(default)]
    pub worldgen_overrides: bool,
    /// What the mod's `storage` may hold, in MiB, when the default (4) is
    /// too little (a map keeping its regions): 1 to 256. Needs capability
    /// `storage`; the player sees the bigger quota on the Mods page.
    #[serde(default)]
    pub storage_quota_mib: Option<u32>,
    /// How many keys the mod's `storage` may hold when the default (64) is
    /// too few: 1 to 4096.
    #[serde(default)]
    pub storage_max_keys: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Runs in the player's game only (HUD, keys, chat commands).
    #[default]
    Client,
    /// Runs where the world is simulated (rules, blocks); not run yet.
    Server,
    /// Both, and the server tells clients which version it expects.
    Both,
}

/// What a mod may do beyond writing its log. Each is a WIT interface the
/// component imports; the player sees them before turning the mod on.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    GameRead,
    Chat,
    Hud,
    /// The block under the crosshair and the one being broken (API 0.1.1).
    Target,
    /// The hotbar's stacks and the selected slot (API 0.1.1).
    InventoryRead,
    /// Changing the selected hotbar slot (API 0.1.1).
    Hotbar,
    /// Blocks of the loaded world and the ones broken each tick (API 0.1.2).
    WorldRead,
    /// Breaking blocks and wearing the held item (API 0.1.2): no interface
    /// of its own, `world`'s edits do nothing without it.
    WorldEdit,
    /// The mod's own key bindings (API 0.1.4).
    Keys,
    /// The mod's own files (API 0.1.4).
    Storage,
    /// A screen the mod draws and handles itself (API 0.1.4).
    Screen,
    /// The items of the game (API 0.1.5).
    ItemsRead,
    /// The recipes of the game (API 0.1.5).
    RecipesRead,
    /// Giving the player items (API 0.1.5): no interface of its own,
    /// `inventory.give` does nothing without it.
    InventoryEdit,
    /// The weather over the player (API 0.1.6).
    WeatherRead,
    /// The client's own settings (API 0.1.6).
    SettingsRead,
    /// The mod's own particles (API 0.1.6).
    Particles,
    /// The mod's haze in the fog (API 0.1.6).
    Atmosphere,
    /// The mod's graphics over the world, in the HUD (API 0.1.7).
    Overlay,
    /// New blocks and items from the mod's `data/` (API 0.2): no WIT
    /// interface, the game reads them before the world loads.
    Registries,
}
impl Capability {
    pub const ALL: [Capability; 20] = [
        Capability::GameRead,
        Capability::Chat,
        Capability::Hud,
        Capability::Target,
        Capability::InventoryRead,
        Capability::Hotbar,
        Capability::WorldRead,
        Capability::WorldEdit,
        Capability::Keys,
        Capability::Storage,
        Capability::Screen,
        Capability::ItemsRead,
        Capability::RecipesRead,
        Capability::InventoryEdit,
        Capability::WeatherRead,
        Capability::SettingsRead,
        Capability::Particles,
        Capability::Atmosphere,
        Capability::Overlay,
        Capability::Registries,
    ];
    /// The WIT interface it grants, in `horizon:mods`.
    pub fn interface(self) -> &'static str {
        match self {
            Capability::GameRead => "game",
            Capability::Chat => "chat",
            Capability::Hud => "hud",
            Capability::Target => "target",
            Capability::InventoryRead => "inventory",
            Capability::Hotbar => "hotbar",
            Capability::WorldRead => "world",
            Capability::WorldEdit => "world",
            Capability::Keys => "keys",
            Capability::Storage => "storage",
            Capability::Screen => "screen",
            Capability::ItemsRead => "items",
            Capability::RecipesRead => "recipes",
            Capability::InventoryEdit => "inventory",
            Capability::WeatherRead => "weather",
            Capability::SettingsRead => "settings",
            Capability::Particles => "particles",
            Capability::Atmosphere => "atmosphere",
            Capability::Overlay => "overlay",
            Capability::Registries => "registries",
        }
    }
    /// What the player reads before turning the mod on: "May <this>".
    pub fn describe(self) -> &'static str {
        match self {
            Capability::GameRead => "see your position, health and the time",
            Capability::Chat => "show you chat messages",
            Capability::Hud => "draw text on your screen",
            Capability::Target => "see the block you look at and the one you break",
            Capability::InventoryRead => "see your hotbar",
            Capability::Hotbar => "change your selected hotbar slot",
            Capability::WorldRead => "see the blocks around you and the ones that break",
            Capability::WorldEdit => "break blocks and wear the item in your hand",
            Capability::Keys => "add keys you can press",
            Capability::Storage => "save its own settings to disk",
            Capability::Screen => "draw a screen of its own",
            Capability::ItemsRead => "see the items of the game",
            Capability::RecipesRead => "see the recipes of the game",
            Capability::InventoryEdit => "give you items",
            Capability::WeatherRead => "see the weather",
            Capability::SettingsRead => "read your settings (particles)",
            Capability::Particles => "add particles of its own",
            Capability::Atmosphere => "change the haze in the fog",
            Capability::Overlay => "draw over the world on your HUD",
            Capability::Registries => "add blocks and items to your worlds",
        }
    }
    /// The capability a component import needs, `Ok(None)` for one every
    /// mod has; `Err` for an import Horizon does not offer.
    pub fn of_import(name: &str) -> Result<Option<Capability>, String> {
        let unknown = || format!("imports {name}, which Horizon does not offer");
        let rest = name.strip_prefix("horizon:mods/").ok_or_else(unknown)?;
        let (interface, version) = rest.split_once('@').ok_or_else(unknown)?;
        if !version.starts_with("0.1.") {
            return Err(format!("needs API {version}; this Horizon runs {API}"));
        }
        if interface == "log" {
            return Ok(None);
        }
        Capability::ALL
            .into_iter()
            .filter(|c| {
                !matches!(
                    c,
                    Capability::Registries | Capability::WorldEdit | Capability::InventoryEdit
                )
            })
            .find(|c| c.interface() == interface)
            .map(Some)
            .ok_or_else(unknown)
    }
}
impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Capability::GameRead => "game-read",
            Capability::Chat => "chat",
            Capability::Hud => "hud",
            Capability::Target => "target",
            Capability::InventoryRead => "inventory-read",
            Capability::Hotbar => "hotbar",
            Capability::WorldRead => "world-read",
            Capability::WorldEdit => "world-edit",
            Capability::Keys => "keys",
            Capability::Storage => "storage",
            Capability::Screen => "screen",
            Capability::ItemsRead => "items-read",
            Capability::RecipesRead => "recipes-read",
            Capability::InventoryEdit => "inventory-edit",
            Capability::WeatherRead => "weather-read",
            Capability::SettingsRead => "settings-read",
            Capability::Particles => "particles",
            Capability::Atmosphere => "atmosphere",
            Capability::Overlay => "overlay",
            Capability::Registries => "registries",
        };
        f.write_str(name)
    }
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let m: Manifest = toml::from_str(text).map_err(|e| format!("{MANIFEST}: {e}"))?;
        let id_ok = (1..=64).contains(&m.id.len())
            && m.id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !id_ok {
            return Err(format!(
                "{MANIFEST}: id {:?} is not [a-z0-9_]{{1,64}}",
                m.id
            ));
        }
        if !APIS.contains(&m.api.as_str()) {
            return Err(format!(
                "built for API {}; this Horizon runs {}",
                m.api,
                APIS.join(" and ")
            ));
        }
        if m.capabilities.contains(&Capability::Registries) && m.api == API {
            return Err(format!(
                "{MANIFEST}: capability registries needs api = \"0.2\""
            ));
        }
        if (m.storage_quota_mib.is_some() || m.storage_max_keys.is_some())
            && !m.capabilities.contains(&Capability::Storage)
        {
            return Err(format!(
                "{MANIFEST}: storage limits are for capability storage"
            ));
        }
        if m.storage_quota_mib.is_some_and(|q| !(1..=256).contains(&q)) {
            return Err(format!("{MANIFEST}: storage_quota_mib is 1 to 256"));
        }
        if m.storage_max_keys.is_some_and(|k| !(1..=4096).contains(&k)) {
            return Err(format!("{MANIFEST}: storage_max_keys is 1 to 4096"));
        }
        if !m.namespaces.is_empty() && !m.capabilities.contains(&Capability::Registries) {
            return Err(format!(
                "{MANIFEST}: namespaces are for content, which needs capability registries"
            ));
        }
        if m.worldgen_overrides && !m.capabilities.contains(&Capability::Registries) {
            return Err(format!(
                "{MANIFEST}: worldgen_overrides replaces world generation data, which needs capability registries"
            ));
        }
        for (i, ns) in m.namespaces.iter().enumerate() {
            // Vanilla's namespace characters, at most as long as an id.
            let chars_ok = (1..=64).contains(&ns.len())
                && ns
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
            if !chars_ok {
                return Err(format!(
                    "{MANIFEST}: namespace {ns:?} is not [a-z0-9_.-]{{1,64}}"
                ));
            }
            if RESERVED_NAMESPACES.contains(&ns.as_str()) {
                return Err(format!(
                    "{MANIFEST}: namespace {ns} is not a mod's to claim (only to add to its tags)"
                ));
            }
            if *ns == m.id || m.namespaces[..i].contains(ns) {
                return Err(format!("{MANIFEST}: namespace {ns} is listed twice"));
            }
        }
        Ok(m)
    }
    /// The namespaces the mod's content owns: its id, then `namespaces`.
    pub fn owned_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str()).chain(self.namespaces.iter().map(String::as_str))
    }
    /// Checks that the component's imports need only declared capabilities.
    pub fn check_imports<'a>(
        &self,
        imports: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), String> {
        for name in imports {
            if let Some(c) = Capability::of_import(name)?
                && !self.capabilities.contains(&c)
            {
                return Err(format!("uses capability {c}, missing from {MANIFEST}"));
            }
        }
        Ok(())
    }
}

/// A mod as found on disk: its manifest and component bytes.
pub struct Package {
    pub manifest: Manifest,
    /// None for a content-only mod (capability `registries`, no code).
    pub wasm: Option<Vec<u8>>,
}
impl Package {
    /// A `.hmod` (a zip holding `horizon-mod.toml`, `mod.wasm` and, for
    /// content, `assets/` and `data/`) or a folder holding the same.
    pub fn read(path: &Path) -> Result<Self, String> {
        let (toml, wasm) = if path.is_dir() {
            let toml = std::fs::read_to_string(path.join(MANIFEST));
            let wasm = path.join(WASM);
            let wasm = match wasm.exists() {
                true => Some(std::fs::read(wasm).map_err(|e| format!("{WASM}: {e}"))?),
                false => None,
            };
            (toml.map_err(|e| format!("{MANIFEST}: {e}"))?, wasm)
        } else {
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            let mut entry = |name: &str| -> Result<Option<Vec<u8>>, String> {
                let f = match zip.by_name(name) {
                    Ok(f) => f,
                    Err(zip::result::ZipError::FileNotFound) => return Ok(None),
                    Err(e) => return Err(format!("{name}: {e}")),
                };
                let mut out = Vec::new();
                f.take(MAX_WASM + 1)
                    .read_to_end(&mut out)
                    .map_err(|e| e.to_string())?;
                Ok(Some(out))
            };
            let toml = entry(MANIFEST)?.ok_or_else(|| format!("{MANIFEST}: missing"))?;
            let toml = String::from_utf8(toml).map_err(|_| format!("{MANIFEST}: not UTF-8"))?;
            (toml, entry(WASM)?)
        };
        if wasm.as_ref().is_some_and(|w| w.len() as u64 > MAX_WASM) {
            return Err(format!("{WASM} is over {} MiB", MAX_WASM >> 20));
        }
        let manifest = Manifest::parse(&toml)?;
        if wasm.is_none() && !manifest.capabilities.contains(&Capability::Registries) {
            return Err(format!(
                "{WASM}: missing (only a content mod, with capability registries, may have no code)"
            ));
        }
        Ok(Package { manifest, wasm })
    }
    /// The `.hmod` bytes for `manifest_text` and a component.
    pub fn write(manifest_text: &str, wasm: &[u8]) -> Result<Vec<u8>, String> {
        Self::write_files(manifest_text, Some(wasm), &[])
    }
    /// The `.hmod` bytes for `manifest_text`, a component if the mod has
    /// code, and content `files` (`assets/...`, `data/...`) by path.
    pub fn write_files(
        manifest_text: &str,
        wasm: Option<&[u8]>,
        files: &[(&str, &[u8])],
    ) -> Result<Vec<u8>, String> {
        use std::io::Write;
        Manifest::parse(manifest_text)?;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        let err = |e: zip::result::ZipError| e.to_string();
        let mut put = |name: &str, bytes: &[u8]| -> Result<(), String> {
            zip.start_file(name, options).map_err(err)?;
            zip.write_all(bytes).map_err(|e| e.to_string())
        };
        put(MANIFEST, manifest_text.as_bytes())?;
        if let Some(wasm) = wasm {
            put(WASM, wasm)?;
        }
        for (name, bytes) in files {
            crate::content::check_path(name)?;
            put(name, bytes)?;
        }
        Ok(zip.finish().map_err(err)?.into_inner())
    }
}

/// Turns a `wasm32-unknown-unknown` cdylib built with the SDK into the
/// component the game loads.
#[cfg(feature = "pack")]
pub fn componentize(core: &[u8]) -> Result<Vec<u8>, String> {
    wit_component::ComponentEncoder::default()
        .module(core)
        .and_then(|e| e.validate(true).encode())
        .map_err(|e| format!("{e:#}"))
}

/// The names a component imports: `horizon:mods/game@0.1.0`, ...
#[cfg(feature = "pack")]
pub fn component_imports(component: &[u8]) -> Result<Vec<String>, String> {
    use wit_parser::{WorldItem, WorldKey};
    let wit_component::DecodedWasm::Component(resolve, world) =
        wit_component::decode(component).map_err(|e| format!("{e:#}"))?
    else {
        return Err("a WIT package, not a component".into());
    };
    let names = resolve.worlds[world]
        .imports
        .iter()
        .map(|(key, item)| match (key, item) {
            (WorldKey::Interface(id), _) | (_, WorldItem::Interface { id, .. }) => resolve
                .id_of(*id)
                .unwrap_or_else(|| resolve.name_world_key(key)),
            _ => resolve.name_world_key(key),
        });
    Ok(names.collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
id = "coords_hud"
name = "Coordinates HUD"
version = "0.1.0"
api = "0.1"
capabilities = ["game-read", "hud"]
"#;

    #[test]
    fn parses_and_checks_imports() {
        let m = Manifest::parse(EXAMPLE).unwrap();
        assert_eq!(m.side, Side::Client);
        assert_eq!(m.capabilities, [Capability::GameRead, Capability::Hud]);
        let ok = [
            "horizon:mods/log@0.1.0",
            "horizon:mods/game@0.1.0",
            "horizon:mods/hud@0.1.2",
        ];
        assert_eq!(m.check_imports(ok), Ok(()));
        let chat = m.check_imports(["horizon:mods/chat@0.1.0"]);
        assert_eq!(
            chat,
            Err("uses capability chat, missing from horizon-mod.toml".into())
        );
        assert!(m.check_imports(["wasi:filesystem/types@0.2.0"]).is_err());
        assert!(m.check_imports(["horizon:mods/game@0.2.0"]).is_err());
        let storage = m.check_imports(["horizon:mods/storage@0.1.0"]);
        assert_eq!(
            storage,
            Err("uses capability storage, missing from horizon-mod.toml".into())
        );
        let with_storage = EXAMPLE.replace("\"hud\"", "\"hud\", \"storage\"");
        let m = Manifest::parse(&with_storage).unwrap();
        assert_eq!(m.check_imports(["horizon:mods/storage@0.1.0"]), Ok(()));
    }

    #[test]
    fn the_hotbar_capabilities_of_api_0_1_1() {
        let tool = EXAMPLE.replace(
            "[\"game-read\", \"hud\"]",
            "[\"target\", \"inventory-read\", \"hotbar\"]",
        );
        let m = Manifest::parse(&tool).unwrap();
        assert_eq!(
            m.capabilities,
            [
                Capability::Target,
                Capability::InventoryRead,
                Capability::Hotbar
            ]
        );
        let imports = [
            "horizon:mods/target@0.1.1",
            "horizon:mods/inventory@0.1.1",
            "horizon:mods/hotbar@0.1.1",
        ];
        assert_eq!(m.check_imports(imports), Ok(()));
        let without = EXAMPLE.replace("\"hud\"", "\"hud\", \"inventory-read\"");
        assert_eq!(
            Manifest::parse(&without)
                .unwrap()
                .check_imports(["horizon:mods/hotbar@0.1.1"]),
            Err("uses capability hotbar, missing from horizon-mod.toml".into())
        );
    }

    #[test]
    fn the_new_capabilities_of_api_0_1_4() {
        let m = Manifest::parse(
            r#"
id = "world_map"
name = "World Map"
version = "0.1.0"
api = "0.1"
capabilities = ["keys", "storage", "screen"]
"#,
        )
        .unwrap();
        assert_eq!(
            m.capabilities,
            [Capability::Keys, Capability::Storage, Capability::Screen]
        );
        for (interface, capability) in [
            ("keys", Capability::Keys),
            ("storage", Capability::Storage),
            ("screen", Capability::Screen),
        ] {
            assert_eq!(capability.interface(), interface);
            assert_eq!(
                m.check_imports([format!("horizon:mods/{interface}@0.1.4").as_str()]),
                Ok(())
            );
        }
        // A mod without them may not import them.
        let plain = Manifest::parse(EXAMPLE).unwrap();
        assert!(plain.check_imports(["horizon:mods/screen@0.1.4"]).is_err());
    }

    #[test]
    fn refuses_bad_ids_and_apis() {
        let bad_id = EXAMPLE.replace("coords_hud", "Coords HUD");
        assert!(Manifest::parse(&bad_id).unwrap_err().contains("is not"));
        let bad_api = EXAMPLE.replace("api = \"0.1\"", "api = \"0.3\"");
        assert!(Manifest::parse(&bad_api).unwrap_err().contains("API 0.3"));
        let unknown = format!("{EXAMPLE}network = true\n");
        assert!(Manifest::parse(&unknown).is_err());
    }

    #[test]
    fn checks_extra_namespaces() {
        let content = "id = \"t_and_t\"\nname = \"T\"\nversion = \"1\"\napi = \"0.2\"\ncapabilities = [\"registries\"]\n";
        let with = |list: &str| Manifest::parse(&format!("{content}namespaces = [{list}]\n"));
        let m = with("\"kaisyn\", \"towns_and_towers\"").unwrap();
        let owned: Vec<&str> = m.owned_namespaces().collect();
        assert_eq!(owned, ["t_and_t", "kaisyn", "towns_and_towers"]);
        for reserved in ["minecraft", "c", "horizon"] {
            let e = with(&format!("\"{reserved}\"")).unwrap_err();
            assert!(e.contains("not a mod's to claim"), "{e}");
        }
        assert!(with("\"t_and_t\"").unwrap_err().contains("twice"));
        assert!(with("\"a\", \"a\"").unwrap_err().contains("twice"));
        assert!(with("\"Kaisyn\"").unwrap_err().contains("is not"));
        assert!(with("\"\"").unwrap_err().contains("is not"));
        let code = format!("{EXAMPLE}namespaces = [\"kaisyn\"]\n");
        assert!(Manifest::parse(&code).unwrap_err().contains("registries"));
    }

    #[test]
    fn hmod_round_trips() {
        let dir = std::env::temp_dir().join(format!("horizon-mods-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("coords_hud.hmod");
        std::fs::write(&path, Package::write(EXAMPLE, b"\0asm").unwrap()).unwrap();
        let p = Package::read(&path).unwrap();
        assert_eq!(p.manifest.id, "coords_hud");
        assert_eq!(p.wasm.as_deref(), Some(&b"\0asm"[..]));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
