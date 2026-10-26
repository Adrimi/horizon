//! Horizon mods in the game (`docs/mods.md`): loading the ones turned on,
//! their tick, frame, chat and command hooks, their HUD panels and what
//! they print. With no mod on, `horizon_mods::Mods` holds no engine and
//! every hook here returns after one check.
//!
//! Code mods run only in builds with the `mods` feature (on by default in
//! this fork): without it the game links no wasmtime, and every hook does
//! nothing. Content mods (`content_root`) load in every build.
use super::Game;
use horizon_client::{
    command::{Dispatcher, vanilla},
    extras_hud::Corners,
    hud::HudBatch,
    mod_list,
};
use std::{path::PathBuf, time::Duration};
#[cfg(feature = "mods")]
use {
    super::{Screen, mod_scan::Palette},
    horizon_client::{
        extras::Corner,
        extras_hud,
        font::Patch,
        hud::{MOD_BIG_ROWS, MOD_BIG_SIDE, MOD_TEXTURE_ROWS, MOD_TEXTURE_SIDE, MOD_TEXTURE_SLOTS},
        items::ItemStack,
        keybinds,
        mod_list::Installed,
        options_ui::Page,
        text::{self, Text},
        ui,
    },
    horizon_mods::{
        Block, Capability, Clock, Food, FoodStats, GameScreen, Item, ItemTags, KeyEvent, ModKey,
        Modifiers, Mods, Output, Player, Point, Recipe, RecipeKind, Runs, ScreenElement,
        ScreenEvent, ScreenShape, Snapshot, Stack, Status, Ui, UiScreen, UiWidget,
    },
    horizon_mods::{KeyPress, MouseMove, MousePress, WheelTurn},
    horizon_world::registry::BlockStateId,
    std::{collections::HashMap, sync::Arc, time::Instant},
    winit::{event::MouseButton, keyboard::KeyCode},
};

#[derive(Default)]
pub(super) struct ModsState {
    #[cfg(feature = "mods")]
    pub(super) mods: Mods,
    /// Capability `input`: a claimed item's use while the button is down.
    #[cfg(feature = "mods")]
    pub(super) using: Option<super::mod_input::ModUse>,
    /// Capability `render`'s camera shakes.
    #[cfg(feature = "mods")]
    pub(super) shake: super::mod_render::Shake,
    /// `--dev-mod PATH`: always on, reloaded when rebuilt.
    pub(super) dev: Vec<PathBuf>,
    /// When dev mods were last checked for a rebuild.
    #[cfg(feature = "mods")]
    checked: Option<Instant>,
    /// The chat's command tree while a running mod owns commands.
    #[cfg(feature = "mods")]
    tree: Option<Dispatcher>,
    /// Blocks broken since the last tick, for `world.broken-blocks`
    /// (`mod_world.rs`).
    pub(super) broken: Vec<horizon_mods::BlockBroken>,
    /// Blocks that went since the last tick (`world.removed-blocks`), and
    /// the positions the player or a mod broke lately, with how many ticks
    /// they stay in mind (API 0.1.19).
    pub(super) removed: Vec<horizon_mods::BlockRemoved>,
    pub(super) recently_broken: Vec<([i32; 3], u8)>,
    /// Watched blocks that got a random tick (`animate_tick`), for
    /// `world.random-ticked-blocks` (API 0.1.12).
    pub(super) ticked: Vec<[i32; 3]>,
    /// The mods' texture rules as last handed to the mesher
    /// (`Mods::texture_version`, API 0.1.14); 0 for none.
    texture_version: u64,
    /// The number the next chat message gets (`chat.get-messages`).
    chat_id: u64,
    /// Every item tag by item, built once from the resources for
    /// `inventory.item-tags`/`get-tags` (API 0.1.3) and shared through the
    /// snapshot. Built on first use: the world resources holding the tags
    /// load after the mods start.
    pub(super) item_tags: std::sync::OnceLock<Arc<ItemTags>>,
    /// The map colours of the blocks, for the chunk scan
    /// (`world.get-chunk-surface`, API 0.1.4). Built on first use, like the
    /// item tags.
    pub(super) palette: std::sync::OnceLock<Arc<Palette>>,
    /// The world's biome names, in the order of their indexes (API 0.1.4).
    pub(super) biomes: std::sync::OnceLock<Arc<Vec<String>>>,
    /// The registry's entity types, built once (`entity.get-types`).
    pub(super) entity_types: std::sync::OnceLock<Arc<Vec<horizon_mods::EntityType>>>,
    /// The tooltips of the items, built once (`items.get-tooltip`).
    pub(super) item_tooltips:
        std::sync::OnceLock<Arc<HashMap<String, Vec<horizon_mods::ItemTooltipLine>>>>,
    /// The screen a mod has open (API 0.1.4).
    pub(super) screen: ModScreen,
    /// Every item of the game, for `items.list` (API 0.1.5); built on first use.
    items: std::sync::OnceLock<Arc<Vec<Item>>>,
    /// The colour of each item's name (API 0.1.20); built with `items`.
    item_colors: std::sync::OnceLock<Arc<HashMap<String, u32>>>,
    /// Every recipe of the game, for `recipes.list` (API 0.1.5); built on first use.
    recipes: std::sync::OnceLock<Arc<Vec<Recipe>>>,
    /// The width of the first characters of the font, for `screen.get-text-width`.
    text_widths: std::sync::OnceLock<Arc<Vec<f32>>>,
    /// The mod whose panel took a mouse press, so it gets the release too.
    overlay_down: Option<String>,
    /// Particle pictures waiting to go into the atlas' particle sheet, and
    /// the names the mods' numbers stand for (API 0.1.6).
    particle_patches: Vec<Patch>,
    /// Every particle picture sent so far, by its cell (`x`, `y`), and how
    /// many frames more the atlas gets them again: a patch that reaches the
    /// GPU before its atlas exists, or a rebuilt atlas, would lose them.
    particle_store: Vec<Patch>,
    pub(super) atlas_resend: u32,
    /// The passthrough list the particle engine has (API 0.1.25).
    passthrough_last: Vec<String>,
    /// Falling blocks that vanish where they land, by where they fell
    /// from (`world.fall-block`, API 0.1.24).
    pub(super) vanish_falls: std::collections::HashSet<[i32; 3]>,
    particle_names: HashMap<u32, String>,
    /// The mods' command particles by `<mod id>:<name>` (API 0.1.19).
    command_particles: HashMap<String, horizon_mods::ParticleOptions>,
    /// The bars the game drew in the last frame, and the camera it drew with
    /// (`hud.get-bars`, `overlay`; API 0.1.7).
    pub(super) bars: Option<horizon_mods::Bars>,
    projection: Option<Projection>,
    /// The number of the world the player is in and when it was loaded
    /// (`game.get-session`, API 0.1.10).
    session: u64,
    session_started: Option<Instant>,
    /// The frame of the last tooltip box the game drew, and the item it was
    /// for (`tooltip`, API 0.1.11).
    pub(super) tooltip_frame: std::cell::Cell<[f32; 4]>,
    tooltip_hovered: std::cell::Cell<Option<(String, [f32; 4])>>,
    /// The enchantments of that item (`tooltip`, API 0.1.27), and the
    /// translation keys of the enchantment lines of the last tooltip the
    /// game built, by the index of the line.
    tooltip_enchantments: std::cell::RefCell<Vec<horizon_mods::TooltipEnchantment>>,
    tooltip_stack: std::cell::RefCell<Option<horizon_mods::TooltipHoveredStack>>,
    pub(super) tooltip_keys: std::cell::RefCell<Vec<(usize, String)>>,
    /// What eating each food gives and whether it can hurt, built once.
    foods: std::sync::OnceLock<Arc<HashMap<String, (Food, bool)>>>,
    /// The light of each block that gives any, built on first use
    /// (`lighting.get-block-emission`, API 0.1.13).
    emissions: std::sync::OnceLock<Arc<HashMap<String, u8>>>,
    /// The atlas slot of each mod's overlay image, by (mod, image).
    overlay_slots: HashMap<(String, u32), usize>,
    /// The version uploaded to each overlay slot.
    overlay_versions: HashMap<usize, u64>,
    /// `command.run` lines for this window's own world, run on the next tick:
    /// run at once, their chat lines would reach the mods while their output
    /// is still being shown, and a mod answering its own lines would recurse.
    #[cfg(feature = "mods")]
    commands: Vec<String>,
    /// The server's command tree with the running mods' commands added.
    #[cfg(feature = "mods")]
    remote_tree: Option<Dispatcher>,
    /// A `ui` mod's screen or settings page while it is up.
    #[cfg(feature = "mods")]
    ui_screen: Option<ModUiScreen>,
    /// The HUD font's advances for `render.text-width`, made once.
    #[cfg(feature = "mods")]
    font_widths: std::sync::Arc<[f32]>,
    /// Extras that are official mods now, being installed.
    #[cfg(feature = "mods")]
    pub(super) moved: super::moved_extras::MovedState,
    /// Playing at the last frame, for `mod_input_frame`.
    #[cfg(feature = "mods")]
    pub(super) playing: bool,
}

/// The mappings `input.mapping` answers for, with the default key their
/// binding is translated to (none: a mouse button, attack or use).
#[cfg(feature = "mods")]
const MAPPINGS: [(&str, Option<winit::keyboard::KeyCode>); 9] = {
    use winit::keyboard::KeyCode as K;
    [
        ("key.forward", Some(K::KeyW)),
        ("key.left", Some(K::KeyA)),
        ("key.back", Some(K::KeyS)),
        ("key.right", Some(K::KeyD)),
        ("key.jump", Some(K::Space)),
        ("key.sneak", Some(K::ShiftLeft)),
        ("key.sprint", Some(K::ControlLeft)),
        ("key.attack", None),
        ("key.use", None),
    ]
};
/// Characters `render.text-width` knows exactly: U+0000 to U+024F.
#[cfg(feature = "mods")]
const FONT_CHARS: u32 = 0x250;

/// A `ui` mod's own screen, or its settings page from Options > Mods: the
/// widgets the mod pushed, laid out and drawn with the menus' own widgets.
#[cfg(feature = "mods")]
struct ModUiScreen {
    /// The running mod's index in `mods.mods.entries()`.
    owner: usize,
    /// Opened from Options > Mods: Done and Esc return there.
    settings: bool,
    /// What the mod's screen replaced (Playing, Chat, ...), to restore.
    under: Screen,
    /// One scroll per `list` widget, in widget order.
    scrolls: Vec<ui::Scroll>,
    /// A key row waiting for the key to bind: the mod key's name.
    binding: Option<String>,
}
impl ModsState {
    /// The chat's command tree: vanilla's, plus the running mods' commands.
    pub(super) fn dispatcher(&self) -> &Dispatcher {
        #[cfg(feature = "mods")]
        if let Some(tree) = &self.tree {
            return tree;
        }
        vanilla::game_dispatcher()
    }
    /// The server's tree plus the mods' commands, when a mod owns any.
    pub(super) fn remote_dispatcher(&self) -> Option<&Dispatcher> {
        #[cfg(feature = "mods")]
        if let Some(tree) = &self.remote_tree {
            return Some(tree);
        }
        None
    }
    /// Whether a mod runs: the chat keeps its lines for them.
    pub(super) fn running(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.is_running() {
            return true;
        }
        false
    }
    /// Whether a mod draws a HUD panel or a corner panel.
    pub(super) fn has_panels(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.panels().next().is_some() || self.mods.corner_panels().next().is_some() {
            return true;
        }
        false
    }
}

/// The screen a mod has open (API 0.1.4): the mod that owns it, the
/// screen's id and the atlas slots of the images it drew.
#[derive(Default)]
pub(super) struct ModScreen {
    /// The mod that owns the screen.
    id: String,
    /// The mod's name, for the title bar.
    name: String,
    /// The screen's id (the mod's own): a new one starts with fresh
    /// images.
    screen_id: u32,
    /// The atlas slot of each of the mod's images, by the image's id.
    slots: HashMap<u32, usize>,
    /// The version of the image in each slot, so it is uploaded once.
    versions: Vec<u64>,
}
impl ModScreen {
    /// The atlas slot of the mod's image `id`; a new one when it has none,
    /// none when the atlas has no slot left.
    fn slot(&mut self, id: u32) -> Option<usize> {
        if let Some(slot) = self.slots.get(&id) {
            return Some(*slot);
        }
        let slot = self.slots.len();
        // The last slot of the atlas' reserved row is the particle sheet.
        if slot >= 8 {
            return None;
        }
        self.slots.insert(id, slot);
        self.versions.push(0);
        Some(slot)
    }
    /// The slot of the mod's image `id`, without making one.
    fn slot_of(&self, id: u32) -> Option<usize> {
        self.slots.get(&id).copied()
    }
    /// The image in the slot changed since it was uploaded.
    fn changed(&mut self, slot: usize, version: u64) -> bool {
        if self.versions.get(slot).copied() == Some(version) {
            return false;
        }
        if let Some(v) = self.versions.get_mut(slot) {
            *v = version;
        }
        true
    }
    /// Forgets the mod's images (its screen closed or changed).
    fn clear(&mut self) {
        self.slots.clear();
        self.versions.clear();
    }
}
/// How often dev mods are checked for a rebuild.
#[cfg(feature = "mods")]
const RELOAD_CHECK: Duration = Duration::from_secs(1);

/// A vanilla command name: mods may not take it.
#[cfg(feature = "mods")]
fn vanilla_command(name: &str) -> bool {
    let d = vanilla::game_dispatcher();
    d.nodes[0]
        .children
        .iter()
        .any(|&c| d.nodes[c].name() == name)
}

/// `root` with the content mods that are on layered over it
/// (docs/modpacks.md): their blocks, items, data and assets. Read once at
/// start, before the resources load; a mod whose content is refused is left
/// out here and its row on the Mods page says why.
pub(super) fn content_root(
    root: &std::path::Path,
    enabled: &std::collections::BTreeSet<String>,
    dev: &[std::path::PathBuf],
) -> std::path::PathBuf {
    use horizon_client::mod_content::{self, ContentMod};
    use horizon_mods::{Capability, Content, Package};
    let here = std::env::current_dir().unwrap_or_default();
    let mut paths = horizon_mods::find(&here.join(mod_list::DIR));
    paths.retain(|p| !dev.contains(p));
    paths.extend(dev.iter().cloned());
    let mut mods: Vec<ContentMod> = Vec::new();
    for path in &paths {
        let Ok(package) = Package::read(path) else {
            continue;
        };
        let m = &package.manifest;
        let on = enabled.contains(&m.id) || dev.contains(path);
        if !on
            || !m.capabilities.contains(&Capability::Registries)
            || mods.iter().any(|c| c.id == m.id)
        {
            continue;
        }
        let content = Content::read(path, m).and_then(|content| {
            // The first of two mods owning one namespace stays on.
            let own = (m.id.as_str(), content.namespaces.as_slice());
            match mods
                .iter()
                .find_map(|c| horizon_mods::content::clash((&c.id, &c.content.namespaces), own))
            {
                Some(e) => Err(e),
                None => Ok(content),
            }
        });
        match content {
            Ok(content) => mods.push(ContentMod {
                id: m.id.clone(),
                content,
            }),
            Err(e) => log::warn!("Horizon: mod {} content not loaded: {e}", m.id),
        }
    }
    match mod_content::resource_root(root, &mods, &here.join(".horizon-cache")) {
        Ok(out) => {
            if out != root {
                log::info!(
                    "Horizon: content from {} mod(s) in {}",
                    mods.len(),
                    out.display()
                );
            }
            mod_content::init_world_types(&out);
            horizon_client::mobs::wolf_variants::init(&out);
            out
        }
        Err(e) => {
            log::warn!("Horizon: content mods not loaded: {e}");
            mod_content::init_world_types(root);
            root.to_owned()
        }
    }
}

/// A build without the `mods` feature: no mod runs.
#[cfg(not(feature = "mods"))]
impl Game<'_> {
    /// Says why the installed and dev mods don't run.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let found = horizon_mods::find(&dir).len() + self.mods.dev.len();
        if found > 0 {
            log::warn!(
                "Horizon: {found} mod(s) not started: this build has no mod runtime (build with --features mods)"
            );
        }
    }
    pub(super) fn merge_remote_tree(&mut self) {}
    pub(super) fn sync_mods(&mut self) {}
    pub(super) fn tick_mods(&mut self) {}
    pub(super) fn mods_frame(&mut self, _elapsed: Duration) {}
    pub(super) fn mod_command(&mut self, _command: &str) -> bool {
        false
    }
    pub(super) fn mods_chat(&mut self, _line: &str) {}
    pub(super) fn draw_mod_panels(&self, _batch: &mut HudBatch, _corners: &mut Corners) {}
    pub(super) fn open_mod_ui_screen(&mut self, _owner: usize, _settings: bool) {}
    pub(super) fn close_mod_ui_screen(&mut self, _tell: bool) {}
    pub(super) fn draw_mod_ui_screen(&mut self, _batch: &mut HudBatch) {}
    pub(super) fn mod_ui_screen_click(&mut self, _button: winit::event::MouseButton) {}
    pub(super) fn mod_ui_screen_key(&mut self, _key: winit::keyboard::KeyCode) -> bool {
        false
    }
    pub(super) fn mod_ui_screen_scroll(&mut self, _delta: f32) {}
}

#[cfg(feature = "mods")]
impl Game<'_> {
    /// Reads `horizon-mods/` and the dev mods, and starts the ones turned on.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let dev = &self.mods.dev;
        let mut paths = horizon_mods::find(&dir);
        paths.retain(|p| !dev.contains(p));
        paths.extend(dev.iter().cloned());
        // A dev mod is on whatever the list says.
        let dev_ids: Vec<String> = dev
            .iter()
            .filter_map(|p| horizon_mods::Package::read(p).ok())
            .map(|p| p.manifest.id)
            .collect();
        let enabled = &self.settings.mods.enabled;
        let on = |id: &str| enabled.contains(id) || dev_ids.iter().any(|d| d == id);
        let mods = Mods::load_for(
            Runs::Client,
            &paths,
            on,
            vanilla_command,
            Some(self.options.resources.clone()),
        );
        self.mods.mods = mods;
        // The mods' own files, in a folder of their own beside the
        // settings (API 0.1.4).
        self.mods.mods.set_storage(Some(
            Self::options_path().with_file_name(mod_list::DATA_DIR),
        ));
        self.mods.mods.set_sprite_source(Some(Self::sprite_source(
            self.pack.render.clone(),
            self.mod_gui_sprites(),
        )));
        self.sync_mod_keys();
        let chosen = &self.settings.mods;
        self.mods.mods.set_bindings(|id| chosen.keys_of(id));
        // Entries were replaced wholesale: a screen's owner index is stale.
        self.mods.ui_screen = None;
        self.list_mods();
        self.sync_mod_commands();
        self.show_mod_output();
    }
    /// The mods' key bindings (API 0.1.4): the key the player chose, or the
    /// mod's own default.
    pub(super) fn sync_mod_keys(&mut self) {
        for (id, bind) in self.mods.mods.all_key_binds() {
            let entry = self
                .settings
                .mods
                .binds
                .entry((id.clone(), bind.action.clone()))
                .or_insert_with(|| mod_list::ModBind {
                    default_key: bind.default_key.clone(),
                    key: bind.default_key.clone(),
                });
            entry.default_key = bind.default_key.clone();
            let key = entry.key.clone();
            self.mods.mods.set_key(&id, &bind.action, &key);
        }
    }
    /// The Mods page's rows, from what loaded.
    fn list_mods(&mut self) {
        let dev = &self.mods.dev;
        self.settings.mods.installed = self
            .mods
            .mods
            .entries()
            .iter()
            .map(|e| {
                let file = e.path.file_name().unwrap_or_default().to_string_lossy();
                let problem = match &e.status {
                    Status::Failed(reason) => Some(format!("Stopped: {reason}")),
                    _ => None,
                };
                let Some(m) = &e.manifest else {
                    return Installed {
                        id: String::new(),
                        title: file.into_owned(),
                        description: String::new(),
                        problem,
                        dev: false,
                        has_settings: false,
                    };
                };
                // What it may do first: the row cuts long descriptions.
                let may: Vec<&str> = m.capabilities.iter().map(|c| c.describe()).collect();
                let may = if may.is_empty() {
                    "Asks for nothing.".to_owned()
                } else {
                    format!("May {}.", may.join(", "))
                };
                Installed {
                    id: m.id.clone(),
                    title: format!("{} {}", m.name, m.version),
                    description: format!("{may} {}", m.description).trim().to_owned(),
                    problem,
                    dev: dev.contains(&e.path),
                    has_settings: e.has_ui_settings(),
                }
            })
            .collect();
    }
    /// The chat's tree after the running mods changed.
    fn sync_mod_commands(&mut self) {
        let mut names = self.mods.mods.commands().peekable();
        self.mods.tree = names
            .peek()
            .is_some()
            .then(|| vanilla::game_dispatcher().with_mod_commands(names));
        self.merge_remote_tree();
    }

    /// The server's command tree with the mods' commands, for the chat.
    pub(super) fn merge_remote_tree(&mut self) {
        let names: Vec<String> = self.mods.mods.commands().map(str::to_owned).collect();
        self.mods.remote_tree = self
            .remote
            .as_ref()
            .and_then(|r| r.commands.as_ref())
            .filter(|_| !names.is_empty())
            .map(|t| t.with_mod_commands(names.iter().map(String::as_str)));
    }

    /// After the options changed: restart the mods if the list of the ones
    /// turned on did.
    pub(super) fn sync_mods(&mut self) {
        let running: Vec<&str> = self
            .mods
            .mods
            .entries()
            .iter()
            .filter(|e| e.status != Status::Off)
            .filter_map(|e| Some(e.manifest.as_ref()?.id.as_str()))
            .collect();
        let wanted: Vec<&str> = self
            .settings
            .mods
            .installed
            .iter()
            .enumerate()
            .filter(|&(i, _)| self.settings.mods.is_on(i))
            .map(|(_, m)| m.id.as_str())
            .collect();
        if running != wanted {
            self.start_mods();
        }
    }
    /// What mods read this tick (or frame, for mods with a `frame` hook).
    fn mod_snapshot(&mut self) -> Snapshot {
        let player = self.player.as_ref().map(|p| Player {
            x: p.position.x,
            y: p.position.y,
            z: p.position.z,
            yaw: self.yaw,
            pitch: self.pitch,
            health: p.health.health(),
            food: p.health.food.food.max(0) as u32,
            dimension: self.dim.id().to_owned(),
        });
        let clock = player.is_some().then(|| Clock {
            game_time: self.world.game_time.max(0) as u64,
            day_time: self.world.day_time.max(0) as u64,
        });
        let food = self.player.as_ref().map(|p| FoodStats {
            food: p.health.food.food.max(0) as u32,
            saturation: p.health.food.saturation,
            exhaustion: p.health.food.exhaustion,
        });
        let (cps, mouse) = self.mouse_reading();
        // The movement and click mappings, as the keystroke overlay reads them:
        // keys by the default key their binding is translated to.
        let mappings = MAPPINGS
            .iter()
            .map(|&(name, key)| {
                let label = keybinds::display_name(self.settings.keys.get(name)).plain();
                let down = match key {
                    Some(k) => self.keys.contains(&k),
                    None => mouse[usize::from(name == "key.use")],
                };
                (name, label, down)
            })
            .collect();
        if self.mods.font_widths.is_empty() {
            let atlas = &self.gui.atlas;
            self.mods.font_widths = (0..FONT_CHARS)
                .map(|c| char::from_u32(c).map_or(6.0, |c| atlas.glyph_cell(c).1))
                .collect();
        }
        let mut snapshot = Snapshot {
            weather: self.mod_weather(),
            particles_setting: u32::from(self.settings.particles),
            player,
            clock,
            gui_size: self.gui_size(),
            held: self.mod_held(),
            frame: self.frame_reading(),
            mappings,
            cps,
            font_widths: self.mods.font_widths.clone(),
            selected: self.selected as u32,
            food,
            ..Snapshot::default()
        };
        let mods = &self.mods.mods;
        let (target, inventory) = (
            mods.uses(Capability::Target),
            mods.uses(Capability::InventoryRead),
        );
        if let (true, Some(rest)) = (inventory, self.res.rest.get()) {
            // Item tags never change while the game runs: index them once, by
            // item, and share the `Arc` with every snapshot (API 0.1.3).
            let tags = self.mods.item_tags.get_or_init(|| {
                Arc::new(ItemTags::new(rest.items.item_tags.iter().map(
                    |(tag, items)| (tag.clone(), items.iter().cloned().collect()),
                )))
            });
            snapshot.item_tags = Some(Arc::clone(tags));
        }
        if mods.uses(Capability::ItemsRead) || mods.uses(Capability::InventoryEdit) {
            snapshot.items = self.mod_items();
            if let Some(items) = snapshot.items.as_ref() {
                let colors = self.mods.item_colors.get_or_init(|| {
                    use horizon_client::text;
                    Arc::new(
                        items
                            .iter()
                            .map(|i| {
                                let color = match self.item_rarity(&i.id).as_str() {
                                    "uncommon" => text::YELLOW,
                                    "rare" => text::AQUA,
                                    "epic" => text::LIGHT_PURPLE,
                                    _ => text::WHITE,
                                };
                                (i.id.clone(), color)
                            })
                            .collect(),
                    )
                });
                snapshot.item_colors = Some(Arc::clone(colors));
            }
        }
        if mods.uses(Capability::RecipesRead) {
            snapshot.recipes = self.mod_recipes();
        }
        if mods.uses(Capability::Particles) {
            snapshot.mod_particles = Some(Arc::new(
                self.particles
                    .mod_particles()
                    .into_iter()
                    .map(|(sprite, p, v, remaining)| horizon_mods::ModParticleInfo {
                        sprite,
                        x: p[0],
                        y: p[1],
                        z: p[2],
                        vx: v[0],
                        vy: v[1],
                        vz: v[2],
                        remaining,
                    })
                    .collect(),
            ));
        }
        if mods.uses(Capability::GameRead) && self.player.is_some() {
            // Only the overworld has clouds (CloudRenderer's height).
            snapshot.cloud_height = (self.dim == super::Dim::Overworld).then_some(192);
            snapshot.viewport = self.renderer.as_ref().map(|r| {
                let size = [r.config.width, r.config.height];
                (
                    HudBatch::new(size).scale,
                    r.config.width as f32,
                    r.config.height as f32,
                )
            });
            snapshot.server_view_distance = if self.has_singleplayer_server() {
                self.settings.render_distance.max(2) as u32
            } else {
                0
            };
            snapshot.game_mode = Some(self.mode.name().to_owned());
            snapshot.player_tags = self.tags_of(super::commands::Target::Player);
        }
        if mods.uses(Capability::GameRead)
            && let Some(p) = self.player.as_ref()
        {
            snapshot.health = Some(horizon_mods::Health {
                health: p.health.health(),
                max_health: 20.0,
                absorption: p.health.absorption(),
            });
            snapshot.effects = self
                .potions
                .effects
                .list
                .iter()
                .map(|e| {
                    if e.id.contains(':') {
                        e.id.to_string()
                    } else {
                        format!("minecraft:{}", e.id)
                    }
                })
                .collect();
            snapshot.difficulty = Some(
                match self.ui.difficulty {
                    0 => "peaceful",
                    1 => "easy",
                    3 => "hard",
                    _ => "normal",
                }
                .to_owned(),
            );
        }
        if mods.uses(Capability::Lighting) {
            snapshot.emissions = Some(Arc::clone(self.mods.emissions.get_or_init(|| {
                let facts = self.res.facts();
                let registry = self.pack.assets.registry;
                let mut emissions = HashMap::new();
                for i in 0..registry.len() as u32 {
                    let name = &facts.get(BlockStateId(i)).name;
                    if emissions.contains_key(&**name) {
                        continue;
                    }
                    let light = registry
                        .default_state(name)
                        .map_or(0, |s| facts.get(s).emission.min(15));
                    emissions.insert(name.to_string(), light);
                }
                emissions.retain(|_, light| *light > 0);
                Arc::new(emissions)
            })));
        }
        if inventory && self.player.is_some() {
            snapshot.foods = Some(self.mod_foods());
            snapshot.offhand = self.offhand.as_ref().map(|s| self.mod_stack(s, None));
        }
        if mods.uses(Capability::GameRead) && self.player.is_some() {
            snapshot.session = Some(horizon_mods::Session {
                id: self.mods.session,
                elapsed_ms: self
                    .mods
                    .session_started
                    .map_or(0, |t| t.elapsed().as_millis() as u64),
            });
        }
        if inventory && self.player.is_some() {
            snapshot.tools = (0..9).map(|i| self.mod_tool(i)).collect();
            snapshot.slots = self.mod_slots();
        }
        if mods.uses(Capability::GameRead)
            && let Some(p) = self.player.as_ref()
        {
            snapshot.input = Some(horizon_mods::InputState {
                sneaking: p.pose == horizon_client::pose::Pose::Crouching,
                attacking: self.left,
            });
        }
        if mods.uses(Capability::GameRead) {
            snapshot.mod_names = Some(Arc::new(
                self.mods
                    .mods
                    .entries()
                    .iter()
                    .filter_map(|e| e.manifest.as_ref())
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect(),
            ));
            snapshot.translator = horizon_mods::Translator(Some(Arc::new(|key: &str| {
                horizon_client::lang::has(key).then(|| horizon_client::lang::get_or(key, key))
            })));
            snapshot.narration_mode = Some("off".to_owned());
        }
        if mods.uses(Capability::ItemsRead) {
            snapshot.item_tooltips = Some(self.mod_item_tooltips());
        }
        if self.player.is_some() {
            snapshot.main_hand = Some(
                if self.settings.main_hand_left {
                    "left"
                } else {
                    "right"
                }
                .to_owned(),
            );
            snapshot.tab_list_open = self.screen == Screen::Playing
                && self.keys.contains(&winit::keyboard::KeyCode::Tab);
            if mods.uses(Capability::Hud) {
                snapshot.boss_bars = self.mod_boss_bars();
            }
        }
        if inventory && self.player.is_some() {
            snapshot.inventory = Some(self.mod_inventory());
            snapshot.crafting = self.mod_crafting();
        }
        let (entity_read, light_events) = (
            mods.uses(Capability::EntityRead),
            mods.uses(Capability::LightEvents),
        );
        if entity_read && self.player.is_some() {
            let entities = self.mod_entities();
            snapshot.entity_details = Some(Arc::new(self.mod_entity_details(&entities)));
            snapshot.invisible = Some(Arc::new(
                self.mobs
                    .iter()
                    .filter(|m| m.invisible || m.has_effect("minecraft:invisibility"))
                    .map(|m| m.id)
                    .collect(),
            ));
            snapshot.entity_types = Some(self.mod_entity_types());
            self.mod_entity_extras(&entities, &mut snapshot);
            snapshot.faces = Some(Arc::new(self.mod_faces(&entities)));
            snapshot.entities = Some(Arc::new(entities));
            snapshot.entities_editable = self.has_singleplayer_server();
        }
        self.particles.watch_light = light_events;
        if light_events && self.player.is_some() {
            snapshot.flashes = Some(Arc::new(self.mod_flashes()));
        }
        if snapshot.player.is_none() {
            return snapshot;
        }
        if !target && !inventory {
            return snapshot;
        }
        let hit = self.pick();
        let looked = hit.as_ref().map(|hit| hit.pos);
        let breaking = self.mining.as_ref().map(|m| m.pos);
        if target {
            snapshot.block_face = hit.as_ref().map(|hit| {
                match hit.face {
                    [0, -1, 0] => "down",
                    [0, 1, 0] => "up",
                    [0, 0, -1] => "north",
                    [0, 0, 1] => "south",
                    [-1, 0, 0] => "west",
                    _ => "east",
                }
                .to_owned()
            });
            snapshot.container = looked.or(breaking).and_then(|pos| {
                let stacks = self.mod_container(pos)?;
                Some((pos, stacks))
            });
            snapshot.block = looked.map(|pos| self.mod_block(pos));
            snapshot.block_properties = snapshot
                .block
                .as_ref()
                .map(|b| self.mod_block_properties(b));
            snapshot.block_loot_table = snapshot.block.as_ref().and_then(|b| {
                let path = b.id.strip_prefix("minecraft:")?;
                let id = format!("minecraft:blocks/{path}");
                self.res.items().has_loot_table(&id).then_some(id)
            });
            snapshot.block_entity = looked.or(breaking).and_then(|pos| {
                let entity = self.mod_block_entity(pos)?;
                Some((pos, entity))
            });
            snapshot.breaking = breaking.map(|pos| self.mod_block(pos));
            snapshot.breaking_progress = self.mining.as_ref().map(|m| m.progress.min(1.0));
            snapshot.block_name = snapshot.block.as_ref().map(|b| {
                horizon_client::lang::get_or(&format!("block.{}", b.id.replace(':', ".")), &b.id)
            });
            snapshot.entity = self.mod_entity();
            self.mod_target_extras(&mut snapshot);
            if let Some(e) = &snapshot.entity
                && let Some(m) = self.mobs.iter().find(|m| m.id == e.id)
            {
                snapshot.entity_armor = Some(m.kind.armor);
                snapshot.entity_absorption = Some(0.0);
            }
        }
        if inventory {
            // The block a tool would be used on: the one being broken, else
            // the one looked at.
            let name = breaking.or(looked).map(|[x, y, z]| {
                let state = self.world.block(x, y, z);
                self.res.facts().get(state).name.clone()
            });
            snapshot.hotbar = (0..9)
                .map(|i| self.inventory.get(i)?.as_ref())
                .map(|stack| stack.map(|stack| self.mod_stack(stack, name.as_deref())))
                .collect();
            let items = self.res.items();
            snapshot.hotbar_food = (0..9)
                .map(|i| {
                    let stack = self.inventory.get(i)?.as_ref()?;
                    let (nutrition, saturation) = items.get(&stack.item)?.food?;
                    Some(Food {
                        nutrition,
                        saturation,
                    })
                })
                .collect();
        }
        snapshot
    }
    /// The mob an attack would hit, as `target` gives it.
    fn mod_entity(&self) -> Option<horizon_mods::Entity> {
        let (i, distance) = self.mob_hit()?;
        if !self.before_block(distance) {
            return None;
        }
        Some(self.mod_entity_of(i))
    }
    /// The mob `i` as `target` gives it.
    fn mod_entity_of(&self, i: usize) -> horizon_mods::Entity {
        let m = &self.mobs[i];
        let name = m.custom_name.clone().unwrap_or_else(|| {
            let key = format!("entity.{}", m.kind.id.replace(':', "."));
            horizon_client::lang::get_or(&key, &m.kind.id)
        });
        horizon_mods::Entity {
            id: m.id,
            type_id: m.kind.id.clone(),
            name,
            health: m.health,
            max_health: m.attrs.max_health,
        }
    }
    /// A block as `target` gives it: its id, hardness and every block tag
    /// it is in.
    fn mod_block(&self, [x, y, z]: [i32; 3]) -> Block {
        let state = self.world.block(x, y, z);
        let facts = self.res.facts().get(state);
        let id = facts.name.to_string();
        let mut tags: Vec<String> = self
            .res
            .items()
            .block_tags
            .iter()
            .filter(|(_, blocks)| blocks.contains(id.as_str()))
            .map(|(tag, _)| tag.clone())
            .collect();
        tags.sort();
        Block {
            x,
            y,
            z,
            id,
            hardness: facts.hardness,
            requires_tool: facts.requires_tool,
            tags,
        }
    }
    /// A hotbar stack as `inventory` gives it, with how fast it breaks
    /// `block` (`Player.getDestroySpeed`'s item part: the tool's speed, plus
    /// `mining_efficiency` when that is above 1).
    fn mod_stack(&self, stack: &horizon_client::items::ItemStack, block: Option<&str>) -> Stack {
        let items = self.res.items();
        let (mining_speed, correct_for_drops) = match block {
            Some(block) => {
                let mut speed = items.mining_speed(Some(stack), block);
                if speed > 1.0 {
                    speed += self
                        .res
                        .stations()
                        .enchantments
                        .attribute_add(Some(stack), "minecraft:mining_efficiency")
                        as f32;
                }
                (speed, items.correct_for_drops(Some(stack), block))
            }
            None => (1.0, false),
        };
        Stack {
            id: stack.item.to_string(),
            count: stack.count,
            damage: stack.damage,
            max_damage: items.get(&stack.item).map_or(0, |i| i.max_damage),
            efficiency: stack.enchantment_level("minecraft:efficiency"),
            mining_speed,
            correct_for_drops,
        }
    }
    /// A game tick.
    pub(super) fn tick_mods(&mut self) {
        // Here, at the player's level: shown inside a console call (a control
        // socket command set the hook off), they would run as the console.
        for line in std::mem::take(&mut self.mods.commands) {
            self.run_command(&line);
        }
        if !self.mods.mods.is_running() {
            return;
        }
        self.tick_mod_use();
        let mut snapshot = self.mod_snapshot();
        // The maximum health is an attribute the snapshot, which only reads,
        // cannot compute.
        if let (Some(h), Some(p)) = (snapshot.health.as_mut(), self.player.as_mut()) {
            h.max_health = p.attribute("minecraft:max_health").unwrap_or(20.0) as f32;
        }
        if self.mods.mods.uses(Capability::Players) {
            snapshot.players = Some(Arc::new(self.mod_players()));
        }
        // The blocks the world's writes turned into air since the last tick,
        // whoever did it (the player, a mod, a leaf decaying, a server).
        if self.mods.mods.uses(Capability::WorldRead) {
            let log = self.world.removed_log.get_or_insert_with(Vec::new);
            let gone = std::mem::take(log);
            for ((x, y, z), before) in gone {
                self.mod_block_removed([x, y, z], BlockStateId(before));
            }
        } else {
            self.world.removed_log = None;
        }
        snapshot.broken = std::mem::take(&mut self.mods.broken);
        snapshot.removed = std::mem::take(&mut self.mods.removed);
        self.mods.recently_broken.retain_mut(|(_, age)| {
            *age = age.saturating_sub(1);
            *age > 0
        });
        snapshot.ticked = self.mod_ticked();
        let landed: Vec<horizon_mods::Landing> = std::mem::take(&mut self.particles.mod_landings)
            .into_iter()
            .map(|l| horizon_mods::Landing {
                sprite: l.sprite,
                x: l.position[0],
                y: l.position[1],
                z: l.position[2],
                water: l.water,
            })
            .collect();
        self.mods.mods.add_landings(&landed);
        self.mods.mods.set_snapshot(snapshot);
        self.lend_mod_world();
        self.mods.mods.tick(self.ticks);
        self.take_mod_world();
        self.sync_command_particles();
        self.sync_mod_textures();
        self.show_mod_output();
        self.apply_mod_particles();
        self.sync_weather_passthrough();
    }
    /// The mods' pictures and rules for faces (API 0.1.14): when they change,
    /// the atlas is built again with them and every section is meshed again,
    /// as for a resource pack reload.
    fn sync_mod_textures(&mut self) {
        let version = self.mods.mods.texture_version();
        if version == self.mods.texture_version {
            return;
        }
        self.mods.texture_version = version;
        horizon_client::texture_rules::set_definitions(self.mods.mods.textures());
        self.reload_packs();
    }
    /// Once per frame: the frame hook, and dev mods' rebuilds.
    pub(super) fn mods_frame(&mut self, elapsed: Duration) {
        if !self.mods.dev.is_empty()
            && self
                .mods
                .checked
                .is_none_or(|t| t.elapsed() >= RELOAD_CHECK)
        {
            self.mods.checked = Some(Instant::now());
            if self.mods.mods.reload_changed(vanilla_command) {
                let chosen = &self.settings.mods;
                self.mods.mods.set_bindings(|id| chosen.keys_of(id));
                self.list_mods();
                self.sync_mod_commands();
                self.system_message(Text::literal("Mods reloaded").with_color(text::GRAY));
            }
        }
        if !self.mods.mods.is_running() {
            return;
        }
        self.sync_mod_ui();
        self.mod_input_frame();
        if self.mods.mods.wants_frames() {
            let snapshot = self.mod_snapshot();
            self.mods.mods.set_snapshot(snapshot);
        }
        self.mods.mods.frame(elapsed.as_secs_f32());
        self.show_mod_output();
        self.apply_mod_particles();
        self.sync_mod_screen();
    }
    /// Keeps the game's screen and the mods' in step (API 0.1.4): a mod
    /// with a screen open has it drawn over the world, with the mouse free.
    fn sync_mod_screen(&mut self) {
        let size = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height]);
        if let Some(size) = size {
            let batch = HudBatch::new(size);
            self.mods.mods.set_screen_size(batch.width, batch.height);
        }
        let open = self
            .mods
            .mods
            .open_screen()
            .map(|(id, name, screen)| (id.to_owned(), name.to_owned(), screen.id));
        match open {
            Some((id, name, screen_id)) => {
                if self.mods.screen.id != id || self.mods.screen.screen_id != screen_id {
                    self.mods.screen = ModScreen {
                        id,
                        name,
                        screen_id,
                        ..Default::default()
                    };
                }
                if self.screen != Screen::ModOwned {
                    self.screen = Screen::ModOwned;
                    self.capture(false);
                }
            }
            None => {
                if self.screen == Screen::ModOwned {
                    self.screen = Screen::Playing;
                    self.capture(true);
                }
                self.mods.screen.clear();
            }
        }
    }
    /// The mod's screen, drawn over the world (API 0.1.4).
    pub(super) fn draw_mod_screen(&mut self, batch: &mut HudBatch) {
        let Some((_, _, screen)) = self.mods.mods.open_screen() else {
            return;
        };
        let elements = screen.elements.clone();
        let atlas = &self.gui.atlas;
        // The images the mod uploaded, into the atlas slots.
        for texture in &screen.textures {
            let Some(slot) = self.mods.screen.slot(texture.id) else {
                continue;
            };
            if !self.mods.screen.changed(slot, texture.version) {
                continue;
            }
            batch.patches.push(Patch {
                x: slot as u32 * MOD_BIG_SIDE,
                y: atlas.height - MOD_TEXTURE_ROWS - MOD_BIG_ROWS,
                width: texture.width,
                height: texture.height,
                rgba: texture.pixels.clone(),
            });
        }
        // The title bar: the mod's name and the screen's title.
        let (w, h) = (batch.width, batch.height);
        let owner = self.mods.screen.name.clone();
        let title = if owner.is_empty() {
            screen.title.clone()
        } else {
            format!("{owner}: {}", screen.title)
        };
        batch.fill(
            atlas,
            0.0,
            0.0,
            w,
            14.0,
            horizon_client::hud::rgba(0, 0, 0, 180),
        );
        batch.text(atlas, &title, 4.0, 3.0, u32::MAX, true);
        for shape in &screen.shapes {
            match shape {
                ScreenShape::Rect(r) => {
                    let [_, red, green, blue] = r.color.to_be_bytes();
                    batch.fill(
                        atlas,
                        r.x,
                        r.y,
                        r.width,
                        r.height,
                        horizon_client::hud::rgba(red, green, blue, 255),
                    );
                }
                ScreenShape::Text(t) => {
                    let [_, red, green, blue] = t.color.to_be_bytes();
                    batch.text(
                        atlas,
                        &t.text,
                        t.x,
                        t.y,
                        horizon_client::hud::rgba(red, green, blue, 255),
                        t.shadow,
                    );
                }
                ScreenShape::Image(i) => {
                    let Some(slot) = self.mods.screen.slot_of(i.texture) else {
                        continue;
                    };
                    let Some(rect) = atlas
                        .sprites
                        .get(&format!("horizon/mod-big{slot}"))
                        .copied()
                    else {
                        continue;
                    };
                    // The image may be smaller than the slot: draw only
                    // the part that was uploaded.
                    let texture = screen.textures.iter().find(|t| t.id == i.texture);
                    let (w, h) = texture.map_or((1, 1), |t| (t.width, t.height));
                    let smooth = texture.is_some_and(|t| t.smooth);
                    let rect = horizon_client::hud::Rect {
                        u1: rect.u0 + (rect.u1 - rect.u0) * (w as f32 / MOD_BIG_SIDE as f32),
                        v1: rect.v0 + (rect.v1 - rect.v0) * (h as f32 / MOD_BIG_SIDE as f32),
                        w,
                        h,
                        ..rect
                    };
                    if smooth {
                        batch.rect_uv_smooth(i.x, i.y, i.width, i.height, rect, u32::MAX);
                    } else {
                        batch.rect_uv(i.x, i.y, i.width, i.height, rect, u32::MAX);
                    }
                }
            }
        }
        let _ = h;
        self.draw_mod_elements(batch, &elements, true);
    }
    /// The mod that has a screen open, if any.
    fn mod_screen_owner(&self) -> Option<String> {
        self.mods.mods.open_screen().map(|(id, _, _)| id.to_owned())
    }
    /// A key the player pressed, for the mod's screen (API 0.1.4). Escape
    /// closes the screen, as it does every screen.
    pub(super) fn mod_screen_key(
        &mut self,
        code: winit::keyboard::KeyCode,
        text: Option<&str>,
        pressed: bool,
    ) {
        let Some(id) = self.mod_screen_owner() else {
            return;
        };
        let key = horizon_client::keybinds::key_name(code)
            .unwrap_or("")
            .to_owned();
        if key.is_empty() {
            return;
        }
        if key == "key.keyboard.escape" {
            self.close_mod_screen();
            return;
        }
        self.mods.mods.push_screen_input(
            &id,
            ScreenEvent::Key(KeyPress {
                key,
                pressed,
                text: text.map(str::to_owned),
            }),
        );
    }
    /// A mouse button, for the mod's screen (API 0.1.4).
    pub(super) fn mod_screen_mouse(&mut self, button: winit::event::MouseButton, pressed: bool) {
        let Some(id) = self.mod_screen_owner() else {
            return;
        };
        let number = match button {
            winit::event::MouseButton::Left => 0,
            winit::event::MouseButton::Right => 1,
            winit::event::MouseButton::Middle => 2,
            _ => 3,
        };
        let (x, y) = self.cursor;
        self.mods.mods.push_screen_input(
            &id,
            ScreenEvent::MouseButton(MousePress {
                button: number,
                pressed,
                x,
                y,
            }),
        );
    }
    /// The mouse moved, for the mod's screen (API 0.1.4).
    pub(super) fn mod_screen_move(&mut self) {
        let Some(id) = self.mod_screen_owner() else {
            return;
        };
        let (x, y) = self.cursor;
        self.mods
            .mods
            .push_screen_input(&id, ScreenEvent::MouseMove(MouseMove { x, y }));
    }
    /// The wheel, for the mod's screen (API 0.1.4).
    pub(super) fn mod_screen_wheel(&mut self, steps: f32) {
        let Some(id) = self.mod_screen_owner() else {
            return;
        };
        let (x, y) = self.cursor;
        self.mods
            .mods
            .push_screen_input(&id, ScreenEvent::Wheel(WheelTurn { steps, x, y }));
    }
    /// The mod's screen closes (the player pressed Escape).
    pub(super) fn close_mod_screen(&mut self) {
        let Some(id) = self.mod_screen_owner() else {
            return;
        };
        self.mods.mods.close_screen(&id);
        self.sync_mod_screen();
    }
    /// The mods' own key bindings (API 0.1.4): the key events they get.
    pub(super) fn mod_key(&mut self, key: winit::keyboard::KeyCode, pressed: bool, repeat: bool) {
        self.mod_controls_key(key, pressed, repeat);
        if !self.mods.mods.uses(Capability::Keys) {
            return;
        }
        let Some(name) = horizon_client::keybinds::key_name(key) else {
            return;
        };
        for (id, bind) in self.mods.mods.all_key_binds() {
            if bind.key != name {
                continue;
            }
            self.mods.mods.push_key_event(
                &id,
                KeyEvent {
                    action: bind.action.clone(),
                    pressed,
                    repeat,
                },
            );
        }
    }
    /// `/command` typed in chat: true when a mod owns it.
    pub(super) fn mod_command(&mut self, command: &str) -> bool {
        if !self.mods.mods.is_running() {
            return false;
        }
        // The command may follow a teleport in the same tick.
        let mut snapshot = self.mod_snapshot();
        if self.mods.mods.uses(Capability::Players) {
            snapshot.players = Some(Arc::new(self.mod_players()));
        }
        self.mods.mods.set_snapshot(snapshot);
        self.lend_mod_world();
        let owned = self.mods.mods.command(command);
        self.take_mod_world();
        if !owned {
            return false;
        }
        self.show_mod_output();
        true
    }
    /// Tells the mods of a chat message before it joins the chat (API
    /// 0.1.15) and returns the picture a mod gave it.
    pub(super) fn mods_chat_message(
        &mut self,
        text: &str,
        sender: Option<u128>,
    ) -> Option<horizon_client::chat::Picture> {
        use horizon_client::chat::{Picture, PictureSource};
        use horizon_mods::{ChatMessage, ChatSender, ChatSource};
        if !self.mods.mods.is_running() {
            return None;
        }
        let id = self.mods.chat_id;
        self.mods.chat_id += 1;
        if let Some(uuid) = sender
            && self.mods.mods.uses(Capability::ChatDecorate)
        {
            self.refresh_chat_heads(uuid);
        }
        let name = sender
            .and_then(|uuid| {
                let (players, _, _) = self.remote_tab_players()?;
                players.into_iter().find(|p| p.uuid == uuid).map(|p| p.name)
            })
            .unwrap_or_default();
        let message = ChatMessage {
            id,
            text: text.to_owned(),
            sender: sender.map(|uuid| ChatSender {
                name,
                uuid: format!("{uuid:032x}"),
            }),
        };
        let decoration = self.mods.mods.chat_message(message)?;
        let (gap, place, styles) = (decoration.gap, decoration.place, decoration.styles);
        let picture = decoration.picture;
        Some(Picture {
            gap: gap.map_or(2, |g| g as i32),
            place: match place {
                horizon_mods::ChatPlace::LineStart => None,
                horizon_mods::ChatPlace::Text(at) => Some(at as usize),
            },
            styles: styles
                .iter()
                .map(|s| horizon_client::chat::PictureStyle {
                    dx: s.offset_x,
                    dy: s.offset_y,
                    width: s.width as i32,
                    height: s.height as i32,
                    color: s.color,
                })
                .collect(),
            source: match picture.source {
                ChatSource::SenderSkin => PictureSource::Sender,
                ChatSource::PlayerSkin(uuid) => {
                    PictureSource::Skin(u128::from_str_radix(&uuid, 16).unwrap_or_default())
                }
                ChatSource::Sprite(name) => PictureSource::Sprite(name),
            },
            regions: picture
                .regions
                .iter()
                .map(|r| [r.x as i32, r.y as i32, r.width as i32, r.height as i32])
                .collect(),
            width: picture.width as i32,
            height: picture.height as i32,
        })
    }
    /// The player list for `players.get-players` (API 0.1.18): the listed
    /// players of the connection, else the player of this game alone.
    fn mod_players(&mut self) -> Vec<horizon_mods::ListedPlayer> {
        if let Some((players, _, _)) = self.remote_tab_players() {
            return players
                .into_iter()
                .map(|p| {
                    let display = p
                        .display_name
                        .as_ref()
                        .map(|d| horizon_client::scoreboard::comp_text(d).plain())
                        .filter(|d| *d != p.name)
                        .unwrap_or_default();
                    horizon_mods::ListedPlayer {
                        name: p.name,
                        display_name: display,
                        uuid: format!("{:032x}", p.uuid),
                    }
                })
                .collect();
        }
        let me = self.level.player_uuid.map(|u| {
            u.iter()
                .fold(0u128, |a, &w| (a << 32) | u128::from(w as u32))
        });
        me.map(|uuid| horizon_mods::ListedPlayer {
            name: super::chat::PLAYER_NAME.to_owned(),
            display_name: String::new(),
            uuid: format!("{uuid:032x}"),
        })
        .into_iter()
        .collect()
    }
    /// Chat lines, command feedback and stopped mods, into the chat.
    pub(super) fn show_mod_output(&mut self) {
        let mut commands = Vec::new();
        // What the world asks for goes to the integrated server's thread,
        // in the order the mods gave it.
        let mut for_server = Vec::new();
        for output in self.mods.mods.drain() {
            let output = match self.route_mod_output(output) {
                Ok(output) => output,
                Err(forwarded) => {
                    for_server.extend(forwarded);
                    continue;
                }
            };
            let line = match output {
                Output::RegisterSound { mod_id, name, path } => {
                    self.audio.add_mod_sound(&mod_id, &name, path);
                    continue;
                }
                Output::RemoveParticles { sprite, pos } => {
                    self.particles.remove_mod(sprite, pos);
                    continue;
                }
                Output::FallBlock {
                    pos,
                    velocity,
                    drops,
                    place,
                } => {
                    self.mod_fall_block(pos, velocity, drops, place);
                    continue;
                }
                Output::SetBlock {
                    pos,
                    id,
                    properties,
                } => {
                    self.mod_set_block(pos, &id, &properties);
                    continue;
                }
                Output::SpawnItem { pos, id, count } => {
                    self.mod_spawn_item(pos, &id, count);
                    continue;
                }
                Output::Exhaust { amount } => {
                    if self.remote.is_none()
                        && let Some(p) = self.player.as_mut()
                    {
                        p.cause_food_exhaustion(amount);
                    }
                    continue;
                }
                Output::MergeDrops { pos } => {
                    self.mod_merge_drops(pos);
                    continue;
                }
                Output::PlaySound {
                    sound,
                    place,
                    volume,
                    pitch,
                } => {
                    let at = place.or_else(|| {
                        self.player
                            .as_ref()
                            .map(|p| [p.position.x, p.position.y, p.position.z])
                    });
                    self.audio.play(&sound, at, volume, pitch);
                    continue;
                }
                Output::Command { command } => {
                    if self.remote.is_none() {
                        commands.push(command);
                    }
                    continue;
                }
                Output::Chat { name, text } => {
                    let mut line = Text::literal(format!("[{name}] ")).with_color(text::GRAY);
                    line.push(Text::literal(text));
                    line
                }
                Output::Feedback { text, error: false } => Text::literal(text),
                Output::Feedback { text, error: true } => Text::literal(text).with_color(text::RED),
                // Vanilla's commands only: a mod running a mod command could loop.
                Output::RunCommand { line } => {
                    let name = line.split(' ').next().unwrap_or_default();
                    if !vanilla_command(name) {
                        // Dropped.
                    } else if self.remote.is_some() {
                        // The server checks the player's permissions.
                        self.send_chat(&format!("/{line}"));
                    } else {
                        self.mods.commands.push(line);
                    }
                    continue;
                }
                Output::Stopped { name, reason } => {
                    // Its screen goes with it: the entry's run is gone, so
                    // `mod_ui_view` finds no widgets and the screen closes.
                    self.list_mods();
                    self.sync_mod_commands();
                    Text::literal(format!("Mod {name} stopped: {reason}")).with_color(text::RED)
                }
                // As the number keys do; the next tick tells a server.
                Output::Select { slot } => {
                    if self.player.is_some() {
                        self.selected = (slot as usize).min(8);
                    }
                    continue;
                }
                // `world-edit`, made after the hook in a world this game runs.
                Output::Break {
                    pos,
                    drops,
                    with_held,
                } => {
                    self.mod_break(pos, drops, with_held);
                    continue;
                }
                Output::RemoveEntity { id } => {
                    self.mod_remove_entity(id);
                    continue;
                }
                Output::SetXp { id, xp } => {
                    if let Some(orb) = self.orbs.iter_mut().find(|o| o.id == id) {
                        orb.value = xp;
                    }
                    continue;
                }
                Output::MoveItem { from, to } => {
                    self.mod_move_item(from as usize, to as usize);
                    continue;
                }
                Output::Narrate { name, text } => {
                    log::info!("[narrator] {name}: {text}");
                    continue;
                }
                Output::ActionBar { text, color } => {
                    self.titles
                        .set_overlay(Text::literal(text).with_color(color), false);
                    continue;
                }
                Output::Give { id, count } => {
                    self.mod_give(&id, count);
                    continue;
                }
                Output::DamageHeld { amount } => {
                    if self.player.is_some() && self.remote.is_none() {
                        self.damage_held(amount);
                    }
                    continue;
                }
                Output::OpenScreen { owner } => {
                    self.open_mod_ui_screen(owner, false);
                    continue;
                }
                Output::Shake { strength, seconds } => {
                    self.mods.shake.add(strength, seconds);
                    continue;
                }
                Output::CloseScreen { owner } => {
                    if self
                        .mods
                        .ui_screen
                        .as_ref()
                        .is_some_and(|s| s.owner == owner)
                    {
                        self.close_mod_ui_screen(false);
                    }
                    continue;
                }
                // Server mods' lines: this game runs no server mod yet.
                Output::Tell { .. } | Output::Broadcast { .. } | Output::Reply { .. } => continue,
            };
            // Not through system_message: a mod must not hear its own lines.
            // Written by this client, so the Chat option doesn't hide it.
            let focused = self.screen == Screen::Chat;
            self.ui.chat.add(
                &self.gui.atlas,
                line,
                None,
                horizon_client::chat::ChatSource::Client,
                self.ticks,
                focused,
            );
        }
        // A mod may set or clear its settings page in any call, not only
        // while loading: its row's Settings button follows.
        let rows = self.settings.mods.installed.iter_mut();
        for (row, e) in rows.zip(self.mods.mods.entries()) {
            row.has_settings = e.has_ui_settings();
        }
        for command in commands {
            self.run_command(&command);
        }
        self.post_mod_server_ops(for_server);
    }
    /// With the world on a server thread, the outputs that change the world
    /// or its player are the server's to carry out: they come back as
    /// `Err` (a command goes to the server as the player's own), the rest
    /// as `Ok`.
    fn route_mod_output(&mut self, output: Output) -> Result<Output, Option<Output>> {
        if self.integrated.is_none() {
            return Ok(output);
        }
        match output {
            Output::Break { pos, .. } => {
                self.mod_note_break(pos);
                Err(Some(output))
            }
            Output::FallBlock { .. }
            | Output::SetBlock { .. }
            | Output::SpawnItem { .. }
            | Output::Exhaust { .. }
            | Output::MergeDrops { .. }
            | Output::RemoveEntity { .. }
            | Output::SetXp { .. }
            | Output::Give { .. }
            | Output::MoveItem { .. }
            | Output::DamageHeld { .. } => Err(Some(output)),
            Output::Command { command } => {
                self.send_chat(&format!("/{command}"));
                Err(None)
            }
            output => Ok(output),
        }
    }
    /// Hands the server thread what mods asked of the world, to carry out
    /// between two ticks as the player (`MinecraftServer.execute`).
    fn post_mod_server_ops(&mut self, ops: Vec<Output>) {
        if ops.is_empty() {
            return;
        }
        if let Some(server) = self.integrated.as_ref() {
            server.post(move |game| game.apply_mod_server_ops(ops));
        }
    }
    /// The mods' HUD panels, stacked with the extras in their corners.
    pub(super) fn draw_mod_panels(&self, batch: &mut HudBatch, corners: &mut Corners) {
        for panel in self.mods.mods.panels() {
            let corner = match panel.corner {
                horizon_mods::Corner::TopLeft => Corner::TopLeft,
                horizon_mods::Corner::TopRight => Corner::TopRight,
                horizon_mods::Corner::BottomLeft => Corner::BottomLeft,
                horizon_mods::Corner::BottomRight => Corner::BottomRight,
            };
            // F3 takes the top left, as it does from the extras.
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let lines: Vec<(&str, u32)> = panel
                .lines
                .iter()
                .map(|l| (l.text.as_str(), l.color))
                .collect();
            extras_hud::draw_lines(batch, &self.gui.atlas, corners, corner, &lines);
        }
        // Capability `render`'s corner panels, placed the same way.
        for panel in self.mods.mods.corner_panels() {
            let corner = match panel.corner {
                horizon_mods::RenderCorner::TopLeft => Corner::TopLeft,
                horizon_mods::RenderCorner::TopRight => Corner::TopRight,
                horizon_mods::RenderCorner::BottomLeft => Corner::BottomLeft,
                horizon_mods::RenderCorner::BottomRight => Corner::BottomRight,
            };
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let at = corners.place(corner, panel.width, panel.height);
            self.draw_shapes(batch, &panel.shapes, at);
        }
    }

    /// A `ui` screen's title and widgets as the mod last pushed them; None
    /// when the surface is gone (the mod stopped or closed it). Borrows
    /// `mods` only, so `self.mods.ui_screen` may stay borrowed for the scrolls.
    fn mod_ui_view<'a>(mods: &'a Mods, ms: &ModUiScreen) -> Option<(&'a str, &'a [UiWidget])> {
        if ms.settings {
            let widgets = mods.settings_widgets(ms.owner);
            if widgets.is_empty() && mods.keys_of(ms.owner).is_empty() {
                return None;
            }
            let name = mods
                .entries()
                .get(ms.owner)
                .and_then(|e| e.manifest.as_ref())
                .map_or("Mod", |m| m.name.as_str());
            Some((name, widgets))
        } else {
            let s: &UiScreen = mods.screen(ms.owner)?;
            Some((s.title.as_str(), s.widgets.as_slice()))
        }
    }
    /// The keys a settings page lists first: the mod's own (`input`).
    fn mod_page_keys<'a>(mods: &'a Mods, ms: &ModUiScreen) -> &'a [ModKey] {
        if ms.settings {
            mods.keys_of(ms.owner)
        } else {
            &[]
        }
    }
    /// A key row's label as Key Binds shows one, "Zoom: Z", marked
    /// "> Zoom: Z <" in yellow while it waits for the new key.
    fn mod_key_label(k: &ModKey, waiting: bool) -> Text {
        let value = format!("{}: {}", k.label, keybinds::display_name(&k.key).plain());
        if waiting {
            Text::literal(format!("> {value} <")).with_color(text::YELLOW)
        } else {
            Text::literal(value)
        }
    }
    /// The player bound the key row that waited to `key`: the mod's key
    /// follows it now and at the next start (`horizon-mods.txt`).
    fn bind_mod_key(&mut self, key: String) {
        let Some(ms) = self.mods.ui_screen.as_mut() else {
            return;
        };
        let Some(name) = ms.binding.take() else {
            return;
        };
        let owner = ms.owner;
        if !self.mods.mods.rebind(owner, &name, &key) {
            return;
        }
        let Some(id) = self
            .mods
            .mods
            .entries()
            .get(owner)
            .and_then(|e| e.manifest.as_ref())
            .map(|m| m.id.clone())
        else {
            return;
        };
        self.settings
            .mods
            .keys
            .entry(id)
            .or_default()
            .insert(name, key);
        self.settings.save(&Self::options_path());
    }
    /// A key while a mod screen is up: true when a key row waited for
    /// it (Key Binds: Escape binds none).
    pub(super) fn mod_ui_screen_key(&mut self, key: KeyCode) -> bool {
        if self
            .mods
            .ui_screen
            .as_ref()
            .is_none_or(|ms| ms.binding.is_none())
        {
            return false;
        }
        let value = match key {
            KeyCode::Escape => None,
            k => keybinds::key_name(k),
        };
        self.bind_mod_key(value.unwrap_or("key.keyboard.unknown").to_owned());
        true
    }
    /// The layout the mod screen's widgets get at `h`: (widget, top,
    /// height), the column being `COLUMN` wide down the middle.
    fn mod_ui_layout(h: f32, top: f32, widgets: &[UiWidget]) -> Vec<(f32, f32)> {
        let mut out = Vec::with_capacity(widgets.len());
        let mut y = top;
        for w in widgets {
            let wh = match w {
                UiWidget::Text(_) => 12.0,
                UiWidget::Item(_) | UiWidget::Button(_) => 20.0,
                UiWidget::List(l) => (l.rows.len().min(6) as f32 * 12.0) + 8.0,
            };
            // Nothing may cover Done; a crowded mod loses what won't fit.
            if y + wh > h - 34.0 {
                break;
            }
            out.push((y, wh));
            y += wh + 6.0;
        }
        out
    }
    /// A mod pushes its screen (`ui.open-screen`), or Options > Mods opens
    /// its settings page (`settings`, the mod's `installed` index).
    pub(super) fn open_mod_ui_screen(&mut self, owner: usize, settings: bool) {
        if self.title.is_some() {
            return;
        }
        if settings {
            self.mods.mods.ui_settings_opened(owner);
            let keys = self.mods.mods.keys_of(owner);
            if self.mods.mods.settings_widgets(owner).is_empty() && keys.is_empty() {
                self.show_mod_output();
                return;
            }
        }
        self.mods.ui_screen = Some(ModUiScreen {
            owner,
            settings,
            under: self.screen,
            scrolls: Vec::new(),
            binding: None,
        });
        self.screen = Screen::Mod;
        if self.player.is_some() {
            self.capture(false);
        }
        self.show_mod_output();
    }
    /// Leaves the mod screen: back to Options for a settings page, to what
    /// it covered for a mod's own. `tell` is Esc/Done, heard as `closed`.
    pub(super) fn close_mod_ui_screen(&mut self, tell: bool) {
        let Some(ms) = self.mods.ui_screen.take() else {
            return;
        };
        if ms.settings {
            self.open_options_at(self.options_from_title, Page::Mods);
        } else {
            self.screen = ms.under;
            if self.screen == Screen::Playing && self.player.is_some() {
                self.capture(true);
            }
        }
        if tell {
            self.mods.mods.ui_closed(ms.owner);
            self.show_mod_output();
        }
    }
    /// The mod's screen or settings page: its title over widgets stacked
    /// down the middle, a Done button, Esc to leave (vanilla's menus).
    pub(super) fn draw_mod_ui_screen(&mut self, batch: &mut HudBatch) {
        let Some(ms) = self.mods.ui_screen.as_mut() else {
            // The screen was dropped while it was up: back to the world,
            // mouse held again like any return to Playing.
            self.screen = Screen::Playing;
            if self.player.is_some() {
                self.capture(true);
            }
            return;
        };
        let Some((title, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            // The mod's surface is gone (it stopped or cleared it): leave
            // the screen like Done did, without `closed` — the mod ended it.
            self.close_mod_ui_screen(false);
            return;
        };
        // Item icons draw in a second pass: `draw_item` needs `&mut self`,
        // so `atlas` may not stay borrowed across it.
        let mut items: Vec<(String, f32, f32)> = Vec::new();
        let atlas = &self.gui.atlas;
        let in_world = self.player.is_some();
        ui::menu_background(batch, atlas, 0.0, 0.0, batch.width, batch.height, in_world);
        ui::centered_text(
            batch,
            atlas,
            &Text::literal(title),
            0.0,
            15.0,
            batch.width,
            u32::MAX,
        );
        let cx = (batch.width / 2.0).floor();
        let x = cx - MOD_UI_COLUMN / 2.0;
        // A settings page starts with the mod's keys, as Key Binds rows.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        for (i, k) in keys.iter().enumerate() {
            let wy = MOD_UI_TOP + i as f32 * MOD_UI_KEY_ROW;
            let hover = ui::hovered(self.cursor, x, wy, MOD_UI_COLUMN, 20.0);
            let waiting = ms.binding.as_deref() == Some(k.name.as_str());
            let label = Self::mod_key_label(k, waiting);
            ui::button(
                batch,
                atlas,
                x,
                wy,
                MOD_UI_COLUMN,
                20.0,
                &label,
                true,
                hover,
            );
        }
        let top = MOD_UI_TOP + keys.len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(batch.height, top, widgets);
        let mut list_i = 0usize;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            match w {
                UiWidget::Button(b) => {
                    let hover = ui::hovered(self.cursor, x, wy, MOD_UI_COLUMN, 20.0);
                    let label = Text::literal(b.label.clone());
                    ui::button(
                        batch,
                        atlas,
                        x,
                        wy,
                        MOD_UI_COLUMN,
                        20.0,
                        &label,
                        true,
                        hover,
                    );
                }
                UiWidget::Text(t) => {
                    let color = if t.color == 0 {
                        u32::MAX
                    } else {
                        t.color | 0xFF00_0000
                    };
                    ui::centered_text(
                        batch,
                        atlas,
                        &Text::literal(t.text.clone()),
                        x,
                        wy + 1.0,
                        MOD_UI_COLUMN,
                        color,
                    );
                }
                UiWidget::Item(i) => {
                    if self.res.items().get(&i.item).is_some() {
                        items.push((i.item.clone(), x + 2.0, wy + 2.0));
                    } else {
                        batch.text(atlas, "?", x + 6.0, wy + 6.0, u32::MAX, true);
                    }
                    let name = if i.name.is_empty() {
                        i.item.rsplit(':').next().unwrap_or(&i.item).to_string()
                    } else {
                        i.name.clone()
                    };
                    batch.text_styled(
                        atlas,
                        &Text::literal(name),
                        x + 22.0,
                        wy + 6.0,
                        u32::MAX,
                        true,
                    );
                }
                UiWidget::List(l) => {
                    ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
                    let scroll = ms.scrolls[list_i];
                    list_i += 1;
                    let rows_top = wy + 4.0;
                    let rows_bottom = wy + wh - 4.0;
                    batch.fill(atlas, x, wy, MOD_UI_COLUMN, wh, 0x8000_0000);
                    for (b, yy) in [(x, wy), (x, wy + wh - 1.0)] {
                        batch.fill(atlas, b, yy, MOD_UI_COLUMN, 1.0, 0xFF50_5050);
                    }
                    for (xx, yy) in [(x, wy), (x + MOD_UI_COLUMN - 1.0, wy)] {
                        batch.fill(atlas, xx, yy, 1.0, wh, 0xFF50_5050);
                    }
                    let content = l.rows.len() as f32 * 12.0;
                    let first = (scroll.amount / 12.0) as usize;
                    let last = (first + ((rows_bottom - rows_top) / 12.0).ceil() as usize + 1)
                        .min(l.rows.len());
                    for (vi, ri) in (first..last).enumerate() {
                        let ry = rows_top + vi as f32 * 12.0 - (scroll.amount % 12.0);
                        if ui::hovered(self.cursor, x, ry, MOD_UI_COLUMN - 6.0, 12.0) {
                            batch.fill(atlas, x + 1.0, ry, MOD_UI_COLUMN - 8.0, 12.0, 0x20FF_FFFF);
                        }
                        if l.selected == Some(ri as u32) {
                            batch.fill(atlas, x + 1.0, ry, 3.0, 12.0, 0xFFFF_FFFF);
                        }
                        batch.text_styled(
                            atlas,
                            &Text::literal(l.rows[ri].clone()),
                            x + 6.0,
                            ry + 2.0,
                            u32::MAX,
                            true,
                        );
                    }
                    ui::scrollbar(
                        batch,
                        atlas,
                        x + MOD_UI_COLUMN - 5.0,
                        rows_top,
                        rows_bottom,
                        content,
                        scroll.amount,
                    );
                }
            }
        }
        let hover = ui::hovered(self.cursor, cx - 100.0, batch.height - 27.0, 200.0, 20.0);
        ui::button(
            batch,
            atlas,
            cx - 100.0,
            batch.height - 27.0,
            200.0,
            20.0,
            &Text::literal("Done"),
            true,
            hover,
        );
        for (id, ix, iy) in items {
            let stack = ItemStack::new(id.as_str(), 1);
            self.draw_item(batch, &stack, ix, iy);
        }
    }
    /// A click on the mod screen: Done leaves it, a `list` row is the mod's
    /// `ui-events::select`.
    pub(super) fn mod_ui_screen_click(&mut self, button: MouseButton) {
        // Key Binds: a mouse button pressed while waiting binds it.
        if self
            .mods
            .ui_screen
            .as_ref()
            .is_some_and(|ms| ms.binding.is_some())
        {
            self.bind_mod_key(keybinds::mouse_name(button));
            return;
        }
        if button != MouseButton::Left {
            return;
        }
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        if ui::hovered(cursor, (w / 2.0).floor() - 100.0, h - 27.0, 200.0, 20.0) {
            self.audio.play_button_click();
            self.close_mod_ui_screen(true);
            return;
        }
        let Some(ms) = self.mods.ui_screen.as_ref() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let owner = ms.owner;
        let x = (w / 2.0).floor() - MOD_UI_COLUMN / 2.0;
        // A key row waits for the key to bind.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        let picked = (0..keys.len()).find(|&i| {
            let wy = MOD_UI_TOP + i as f32 * MOD_UI_KEY_ROW;
            ui::hovered(cursor, x, wy, MOD_UI_COLUMN, 20.0)
        });
        if let Some(i) = picked {
            let name = keys[i].name.clone();
            self.audio.play_button_click();
            if let Some(ms) = self.mods.ui_screen.as_mut() {
                ms.binding = Some(name);
            }
            return;
        }
        let top = MOD_UI_TOP + keys.len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(h, top, widgets);
        let mut list_i = 0usize;
        // The `ui_select` call needs `&mut`: find the pick first.
        let mut hit = None;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            // A button is `select(id, 0)`.
            if let UiWidget::Button(b) = w
                && ui::hovered(cursor, x, wy, MOD_UI_COLUMN, 20.0)
            {
                self.audio.play_button_click();
                let owner = self.mods.ui_screen.as_ref().map_or(0, |ms| ms.owner);
                self.mods.mods.ui_select(owner, b.id, 0);
                self.show_mod_output();
                return;
            }
            let UiWidget::List(l) = w else { continue };
            let scroll = ms.scrolls.get(list_i).copied().unwrap_or_default();
            list_i += 1;
            let row = ((cursor.1 - (wy + 4.0) + scroll.amount) / 12.0) as i64;
            if ui::hovered(cursor, x + 1.0, wy + 1.0, MOD_UI_COLUMN - 7.0, wh - 2.0)
                && row >= 0
                && (row as usize) < l.rows.len()
            {
                hit = Some((l.id, row as u32));
                break;
            }
        }
        if let Some((id, row)) = hit {
            self.audio.play_button_click();
            self.mods.mods.ui_select(owner, id, row);
            self.show_mod_output();
        }
    }
    /// The wheel over a `list` scrolls it.
    pub(super) fn mod_ui_screen_scroll(&mut self, delta: f32) {
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        let Some(ms) = self.mods.ui_screen.as_mut() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let x = (w / 2.0).floor() - MOD_UI_COLUMN / 2.0;
        let top =
            MOD_UI_TOP + Self::mod_page_keys(&self.mods.mods, ms).len() as f32 * MOD_UI_KEY_ROW;
        let layout = Self::mod_ui_layout(h, top, widgets);
        let mut list_i = 0usize;
        for (w, &(wy, wh)) in widgets.iter().zip(&layout) {
            let UiWidget::List(l) = w else { continue };
            let over = ui::hovered(cursor, x, wy, MOD_UI_COLUMN, wh);
            ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
            if over {
                let content = l.rows.len() as f32 * 12.0;
                ms.scrolls[list_i].wheel(delta, 12.0, wy + 4.0, wy + wh - 4.0, content);
                return;
            }
            list_i += 1;
        }
    }
}

/// 0xAARRGGBB, as mods give colours, in the HUD batch's byte order.
fn argb(c: u32) -> u32 {
    let [a, r, g, b] = c.to_be_bytes();
    horizon_client::hud::rgba(r, g, b, a)
}
fn tip_lines(lines: &[horizon_mods::TipLine]) -> Vec<(Text, u32)> {
    lines
        .iter()
        .map(|l| (Text::legacy(&l.text), argb(l.color)))
        .collect()
}

/// The game's screens, items and recipes for mods, and the panels mods paint
/// over the game's own screens (API 0.1.5).
impl Game<'_> {
    /// Which screen of the game is open.
    fn mod_game_screen(&self) -> GameScreen {
        match self.screen {
            Screen::Playing | Screen::ModOwned | Screen::GameModeSwitcher => GameScreen::None,
            Screen::Chat => GameScreen::Chat,
            Screen::Options | Screen::Paused => GameScreen::Options,
            Screen::Creative => GameScreen::Creative,
            Screen::Inventory if self.station.is_some() => GameScreen::Station,
            Screen::Inventory if self.mount_active() => GameScreen::Mount,
            Screen::Inventory if self.container.is_some() => GameScreen::Container,
            Screen::Inventory if self.craft_size == 3 => GameScreen::Crafting,
            Screen::Inventory => GameScreen::Inventory,
            _ => GameScreen::Other,
        }
    }
    /// The workstation screen that is open, for `screen.get-station`.
    fn mod_station(&self) -> Option<String> {
        use super::stations::Kind;
        if self.screen != Screen::Inventory {
            return None;
        }
        let id = match self.station.as_ref()?.kind {
            Kind::Enchanting => "minecraft:enchanting_table",
            Kind::Anvil => "minecraft:anvil",
            Kind::Grindstone => "minecraft:grindstone",
            Kind::Smithing => "minecraft:smithing_table",
            Kind::Stonecutter => "minecraft:stonecutter",
            Kind::Merchant => "minecraft:villager",
            Kind::Beacon => "minecraft:beacon",
            Kind::Cartography => "minecraft:cartography_table",
            Kind::Loom => "minecraft:loom",
        };
        Some(id.to_owned())
    }
    /// A mod's panel is painted over the inventory-like screens only.
    fn mod_overlay_screen(&self) -> bool {
        self.title.is_none() && self.screen == Screen::Inventory
    }
    /// What mods read of the screens, once per frame.
    fn sync_mod_ui(&mut self) {
        let mods = &self.mods.mods;
        if !(mods.uses(Capability::Screen)
            || mods.uses(Capability::InventoryEdit)
            || mods.uses(Capability::Overlay)
            || mods.uses(Capability::SlotsRead)
            || mods.uses(Capability::GameRead)
            || mods.uses(Capability::Hud))
        {
            return;
        }
        let Some(size) = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height])
        else {
            return;
        };
        let batch = HudBatch::new(size);
        let screen = self.mod_game_screen();
        let mut slots = Vec::new();
        let mut menu_origin = None;
        if self.screen == Screen::Inventory {
            let (ox, oy) = self.inventory_origin(&batch);
            menu_origin = Some(Point { x: ox, y: oy });
            if self.mods.mods.overlays().next().is_some() {
                for i in self.slots() {
                    let (sx, sy) = self.slot_position(i);
                    if let Some(stack) = self.slot_view(i) {
                        slots.push(([ox + sx, oy + sy, 16.0, 16.0], stack.item.to_string()));
                    }
                }
            }
        } else if self.screen == Screen::Creative {
            let (ox, oy) = self.inventory_origin(&batch);
            menu_origin = Some(Point { x: ox, y: oy });
        }
        let widths = self
            .mods
            .text_widths
            .get_or_init(|| {
                let atlas = &self.gui.atlas;
                Arc::new(
                    (0..horizon_mods::TEXT_WIDTHS as u32)
                        .map(|c| match char::from_u32(c) {
                            Some(ch) if c >= 32 && c != 127 => {
                                HudBatch::text_width(atlas, &ch.to_string())
                            }
                            _ => 0.0,
                        })
                        .collect(),
                )
            })
            .clone();
        let ui = Ui {
            screen,
            menu_origin,
            cursor: Point {
                x: self.cursor.0,
                y: self.cursor.1,
            },
            modifiers: Modifiers {
                shift: self.has_shift_down(),
                ctrl: self.has_control_down(),
                alt: self
                    .physical_keys
                    .contains(&winit::keyboard::KeyCode::AltLeft)
                    || self
                        .physical_keys
                        .contains(&winit::keyboard::KeyCode::AltRight),
            },
            slots,
            text_widths: Some(widths),
            can_give: self.creative && self.has_singleplayer_server() && self.player.is_some(),
            can_move: self.has_singleplayer_server() && self.player.is_some(),
            hud: self.player.is_some().then_some(horizon_mods::HudState {
                screen_open: self.screen != Screen::Playing,
                debug: self.debug,
                hidden: self.ui.hide_gui,
            }),
            bars: self.mods.bars.clone(),
            hovered_slot: self.mod_hovered_slot(&batch),
            keys_down: self
                .physical_keys
                .iter()
                .filter_map(|k| horizon_client::keybinds::key_name(*k))
                .map(str::to_owned)
                .collect(),
            enchantments: self.mods.tooltip_enchantments.take(),
            station: self.mod_station(),
            stack: self.mods.tooltip_stack.take(),
            tooltip: self.mods.tooltip_hovered.take().map(|(item, f)| {
                horizon_mods::TooltipHovered {
                    item,
                    x: f[0],
                    y: f[1],
                    width: f[2],
                    height: f[3],
                }
            }),
            seed: (self.has_singleplayer_server() && self.player.is_some())
                .then(|| self.world_seed()),
            fov: self.mods.projection.map(|p| p.fov),
            size: Some(horizon_mods::ScreenSize {
                width: batch.width,
                height: batch.height,
            }),
        };
        self.mods.mods.set_ui(ui);
    }
    /// The item tags for mods, indexed once.
    fn mod_item_tags(&self) -> Option<Arc<ItemTags>> {
        let rest = self.res.rest.get()?;
        let tags = self.mods.item_tags.get_or_init(|| {
            Arc::new(ItemTags::new(rest.items.item_tags.iter().map(
                |(tag, items)| (tag.clone(), items.iter().cloned().collect()),
            )))
        });
        Some(Arc::clone(tags))
    }
    /// Every item, in the creative search tab's order (API 0.1.5).
    fn mod_items(&self) -> Option<Arc<Vec<Item>>> {
        if let Some(items) = self.mods.items.get() {
            return Some(Arc::clone(items));
        }
        let tags = self.mod_item_tags()?;
        let stacks = self.creative_screen.search_items();
        if stacks.is_empty() {
            return None;
        }
        let items = stacks.iter().map(|stack| {
            let id = stack.item.to_string();
            Item {
                name: self.pack.names.item(&id),
                max_stack: self.max_stack(stack),
                tags: tags.of(&id),
                id,
            }
        });
        let mut seen = std::collections::HashSet::new();
        let items: Vec<Item> = items.filter(|i| seen.insert(i.id.clone())).collect();
        Some(Arc::clone(self.mods.items.get_or_init(|| Arc::new(items))))
    }
    /// Every recipe the browser shows, as `recipes` gives it (API 0.1.5).
    fn mod_recipes(&self) -> Option<Arc<Vec<Recipe>>> {
        use horizon_client::item_browser::{Lookup, Shown};
        if let Some(recipes) = self.mods.recipes.get() {
            return Some(Arc::clone(recipes));
        }
        self.res.rest.get()?;
        let lookup = Lookup::collect(self.res.recipes(), &self.res.stations().recipes);
        let recipes = (0..lookup.len())
            .filter_map(|i| lookup.get(i))
            .map(|shown| {
                let station = shown.station().to_owned();
                match shown {
                    Shown::Crafting {
                        width,
                        height,
                        cells,
                        shapeless,
                        result,
                        count,
                    } => Recipe {
                        kind: if *shapeless {
                            RecipeKind::CraftingShapeless
                        } else {
                            RecipeKind::CraftingShaped
                        },
                        station,
                        inputs: cells.clone(),
                        output: Some(result.clone()),
                        count: *count,
                        width: *width as u32,
                        height: *height as u32,
                        time: 0,
                        experience: 0.0,
                    },
                    Shown::Cooking {
                        kind,
                        input,
                        result,
                        count,
                        time,
                        experience,
                    } => Recipe {
                        kind: match kind.as_str() {
                            "minecraft:blasting" => RecipeKind::Blasting,
                            "minecraft:smoking" => RecipeKind::Smoking,
                            "minecraft:campfire_cooking" => RecipeKind::CampfireCooking,
                            _ => RecipeKind::Smelting,
                        },
                        station,
                        inputs: vec![input.clone()],
                        output: Some(result.clone()),
                        count: *count,
                        width: 0,
                        height: 0,
                        time: *time,
                        experience: *experience,
                    },
                    Shown::Stonecutting {
                        input,
                        result,
                        count,
                    } => Recipe {
                        kind: RecipeKind::Stonecutting,
                        station,
                        inputs: vec![input.clone()],
                        output: Some(result.clone()),
                        count: *count,
                        width: 0,
                        height: 0,
                        time: 0,
                        experience: 0.0,
                    },
                    Shown::Smithing {
                        template,
                        base,
                        addition,
                        result,
                    } => Recipe {
                        kind: if result.is_some() {
                            RecipeKind::SmithingTransform
                        } else {
                            RecipeKind::SmithingTrim
                        },
                        station,
                        inputs: vec![template.clone(), base.clone(), addition.clone()],
                        output: result.clone(),
                        count: 1,
                        width: 0,
                        height: 0,
                        time: 0,
                        experience: 0.0,
                    },
                }
            })
            .collect();
        Some(Arc::clone(
            self.mods.recipes.get_or_init(|| Arc::new(recipes)),
        ))
    }
    /// `inventory.give`: the stack goes to the player, what does not fit is
    /// dropped, as the creative inventory does it.
    fn mod_give(&mut self, id: &str, count: u32) {
        if !self.creative || self.remote.is_some() || self.player.is_none() {
            return;
        }
        if self.res.items().get(id).is_none() {
            return;
        }
        let mut left = count;
        while left > 0 {
            let mut stack = horizon_client::items::ItemStack::new(id, 1);
            let take = left.min(self.max_stack(&stack).max(1));
            stack.count = take;
            left -= take;
            if let Some(rest) = self.give(stack) {
                self.player_drop(rest);
            }
        }
    }
    /// Draws what a mod painted (`screen.paint`); the tooltips go last, over
    /// everything.
    fn draw_mod_elements(
        &mut self,
        batch: &mut HudBatch,
        elements: &[ScreenElement],
        pictures: bool,
    ) {
        self.draw_elements(batch, pictures.then_some(None), elements);
    }
    /// The elements of a mod's overlay, whose images are the overlay's.
    fn draw_mod_elements_of(
        &mut self,
        batch: &mut HudBatch,
        mod_id: Option<&str>,
        elements: &[ScreenElement],
    ) {
        self.draw_elements(batch, Some(mod_id), elements);
    }
    /// `pictures`: none for no images, `Some(None)` for the screen's,
    /// `Some(Some(id))` for the overlay of the mod `id`.
    fn draw_elements(
        &mut self,
        batch: &mut HudBatch,
        pictures: Option<Option<&str>>,
        elements: &[ScreenElement],
    ) {
        let mut tips: Vec<(Vec<(Text, u32)>, f32, f32)> = Vec::new();
        for element in elements {
            match element {
                ScreenElement::Fill(f) => {
                    let atlas = &self.gui.atlas;
                    if f.top == f.bottom {
                        batch.fill(atlas, f.x, f.y, f.width, f.height, argb(f.top));
                    } else {
                        batch.fill_gradient(
                            atlas,
                            (f.x, f.y, f.width, f.height),
                            argb(f.top),
                            argb(f.bottom),
                        );
                    }
                }
                ScreenElement::Label(l) => {
                    let start = batch.vertices.len();
                    batch.text(&self.gui.atlas, &l.text, l.x, l.y, argb(l.color), l.shadow);
                    if l.scale > 0.0 && l.scale != 1.0 && l.scale.is_finite() {
                        batch.scale_since(start, l.x, l.y, l.scale, l.scale);
                    }
                }
                ScreenElement::Sprite(s) => {
                    batch.sprite_scaled(
                        &self.gui.atlas,
                        &s.name,
                        s.x,
                        s.y,
                        s.width,
                        s.height,
                        argb(s.tint),
                    );
                }
                ScreenElement::Picture(p) if pictures.is_some() => {
                    let overlay = matches!(pictures, Some(Some(_)));
                    let slot = match pictures {
                        Some(Some(mod_id)) => self
                            .mods
                            .overlay_slots
                            .get(&(mod_id.to_owned(), p.texture))
                            .copied(),
                        _ => self.mods.screen.slot_of(p.texture),
                    };
                    let Some(slot) = slot else {
                        continue;
                    };
                    let (name, side) = if overlay {
                        (
                            format!("horizon/mod-texture{slot}"),
                            MOD_TEXTURE_SIDE as f32,
                        )
                    } else {
                        (format!("horizon/mod-big{slot}"), MOD_BIG_SIDE as f32)
                    };
                    let Some(rect) = self.gui.atlas.sprites.get(&name).copied() else {
                        continue;
                    };
                    let smooth = !overlay
                        && self.mods.mods.open_screen().is_some_and(|(_, _, screen)| {
                            screen
                                .textures
                                .iter()
                                .any(|t| t.id == p.texture && t.smooth)
                        });
                    let (du, dv) = (rect.u1 - rect.u0, rect.v1 - rect.v0);
                    let part = horizon_client::hud::Rect {
                        u0: rect.u0 + du * (p.u / side),
                        v0: rect.v0 + dv * (p.v / side),
                        u1: rect.u0 + du * ((p.u + p.u_width) / side),
                        v1: rect.v0 + dv * ((p.v + p.v_height) / side),
                        w: p.u_width.max(1.0) as u32,
                        h: p.v_height.max(1.0) as u32,
                    };
                    if smooth {
                        batch.rect_uv_smooth(p.x, p.y, p.width, p.height, part, argb(p.tint));
                    } else {
                        batch.rect_uv(p.x, p.y, p.width, p.height, part, argb(p.tint));
                    }
                }
                ScreenElement::Picture(_) => {}
                ScreenElement::Item(i) => {
                    if self.res.items().get(&i.id).is_none() {
                        continue;
                    }
                    let stack = horizon_client::items::ItemStack::new(&i.id, i.count.max(1));
                    self.draw_item(batch, &stack, i.x, i.y);
                }
                ScreenElement::Tooltip(t) => {
                    if self.res.items().get(&t.id).is_none() {
                        continue;
                    }
                    let stack = horizon_client::items::ItemStack::new(&t.id, t.count.max(1));
                    let mut lines = self.stack_tooltip(&stack);
                    lines.extend(tip_lines(&t.extra));
                    tips.push((lines, t.x, t.y));
                }
                ScreenElement::Tip(t) => tips.push((tip_lines(&t.lines), t.x, t.y)),
            }
        }
        for (lines, x, y) in tips {
            // The box sits 12 pixels right of and above the point it is given.
            self.draw_tooltip_lines(batch, &lines, x - 12.0, y + 12.0);
        }
    }
    /// The panels mods paint over the inventory-like screens.
    pub(super) fn draw_mod_overlays(&mut self, batch: &mut HudBatch) {
        if !self.mod_overlay_screen() {
            return;
        }
        let panels: Vec<Vec<ScreenElement>> = self
            .mods
            .mods
            .overlays()
            .filter(|(_, _, s)| s.overlay.as_ref().is_some_and(|o| o.area.width > 0.0))
            .map(|(_, _, s)| s.elements.clone())
            .collect();
        for elements in panels {
            self.draw_mod_elements(batch, &elements, false);
        }
    }
    /// The panel under the cursor, by the mod's id.
    fn mod_overlay_at(&self) -> Option<String> {
        let (x, y) = self.cursor;
        self.mods
            .mods
            .overlays()
            .find(|(_, _, s)| s.overlay.as_ref().is_some_and(|o| o.contains(x, y)))
            .map(|(id, _, _)| id.to_owned())
    }
    /// A mouse button for a panel over the screen (true: the panel took it).
    pub(super) fn mod_overlay_mouse(
        &mut self,
        button: winit::event::MouseButton,
        pressed: bool,
    ) -> bool {
        if !self.mod_overlay_screen() || self.mods.mods.overlays().next().is_none() {
            return false;
        }
        let id = match (self.mod_overlay_at(), pressed) {
            (Some(id), true) => {
                self.mods.overlay_down = Some(id.clone());
                id
            }
            (_, false) => match self.mods.overlay_down.take() {
                Some(id) => id,
                None => return false,
            },
            (None, true) => return false,
        };
        let number = match button {
            winit::event::MouseButton::Left => 0,
            winit::event::MouseButton::Right => 1,
            winit::event::MouseButton::Middle => 2,
            _ => 3,
        };
        let (x, y) = self.cursor;
        self.mods.mods.push_overlay_input(
            &id,
            ScreenEvent::MouseButton(MousePress {
                button: number,
                pressed,
                x,
                y,
            }),
        );
        true
    }
    /// The wheel over a panel (true: the panel took it).
    pub(super) fn mod_overlay_wheel(&mut self, steps: f32) -> bool {
        if !self.mod_overlay_screen() {
            return false;
        }
        let Some(id) = self.mod_overlay_at() else {
            return false;
        };
        let (x, y) = self.cursor;
        self.mods
            .mods
            .push_overlay_input(&id, ScreenEvent::Wheel(WheelTurn { steps, x, y }));
        true
    }
    /// The mouse moved over a panel.
    pub(super) fn mod_overlay_move(&mut self) {
        if !self.mod_overlay_screen() {
            return;
        }
        if let Some(id) = self.mod_overlay_at() {
            let (x, y) = self.cursor;
            self.mods
                .mods
                .push_overlay_input(&id, ScreenEvent::MouseMove(MouseMove { x, y }));
        }
    }
    /// A key for a panel that asked for it (true: the panel took it; the
    /// game does not see it). The modifier keys reach both.
    pub(super) fn mod_overlay_key(
        &mut self,
        code: winit::keyboard::KeyCode,
        text: Option<&str>,
        pressed: bool,
    ) -> bool {
        use winit::keyboard::KeyCode as K;
        if !self.mod_overlay_screen() {
            return false;
        }
        let Some(key) = horizon_client::keybinds::key_name(code) else {
            return false;
        };
        let Some(id) = self
            .mods
            .mods
            .overlays()
            .find(|(_, _, s)| s.overlay.as_ref().is_some_and(|o| o.takes(key)))
            .map(|(id, _, _)| id.to_owned())
        else {
            return false;
        };
        let modifier = matches!(
            code,
            K::ShiftLeft
                | K::ShiftRight
                | K::ControlLeft
                | K::ControlRight
                | K::AltLeft
                | K::AltRight
                | K::SuperLeft
                | K::SuperRight
        );
        self.mods.mods.push_overlay_input(
            &id,
            ScreenEvent::Key(KeyPress {
                key: key.to_owned(),
                pressed,
                text: text.filter(|_| pressed).map(str::to_owned),
            }),
        );
        !modifier
    }
    /// Whether a panel takes every key (a search box has the focus).
    pub(super) fn mod_overlay_typing(&self) -> bool {
        self.mod_overlay_screen()
            && self.mods.mods.overlays().any(|(_, _, s)| {
                s.overlay
                    .as_ref()
                    .is_some_and(|o| o.takes("*") && o.area.width > 0.0)
            })
    }
    /// `horizon-ctl type`: text for a panel that takes every key.
    pub(super) fn mod_overlay_text(&mut self, text: &str) {
        let Some(id) = self
            .mods
            .mods
            .overlays()
            .find(|(_, _, s)| s.overlay.as_ref().is_some_and(|o| o.takes("*")))
            .map(|(id, _, _)| id.to_owned())
        else {
            return;
        };
        let (key, text) = if text == "\\b" {
            ("key.keyboard.backspace", None)
        } else {
            ("key.keyboard.unknown", Some(text.to_owned()))
        };
        self.mods.mods.push_overlay_input(
            &id,
            ScreenEvent::Key(KeyPress {
                key: key.to_owned(),
                pressed: true,
                text,
            }),
        );
    }
}

/// The sheet of the mods' particle pictures: the atlas' last reserved slot,
/// 256 pixels square, split into 64 pixel cells (API 0.1.6).
/// How many frames the mods' pictures are sent again after a change.
const RESEND_FRAMES: u32 = 180;
const PARTICLE_CELL: u32 = 64;
const PARTICLE_CELLS_PER_ROW: u32 = 4;

impl Game<'_> {
    /// The weather over the player, for `weather` (API 0.1.6).
    fn mod_weather(&self) -> Option<horizon_mods::WeatherState> {
        if !self.mods.mods.uses(Capability::WeatherRead) || self.player.is_none() {
            return None;
        }
        let (rain, thunder) = if self.dim == super::Dim::Overworld {
            self.world.weather.levels(self.clock.fraction() as f32)
        } else {
            (0.0, 0.0)
        };
        Some(horizon_mods::WeatherState { rain, thunder })
    }
    /// What mods defined and spawned for their particles: the pictures go
    /// into the atlas' particle sheet, the particles into the engine.
    fn apply_mod_particles(&mut self) {
        if !self.mods.mods.uses(Capability::Particles) {
            return;
        }
        let (sprites, particles) = self.mods.mods.take_particles();
        if sprites.is_empty() && particles.is_empty() {
            return;
        }
        let sheet = format!("horizon/mod-texture{}", MOD_TEXTURE_SLOTS - 1);
        let Some(sheet_rect) = self.gui.atlas.sprites.get(&sheet).copied() else {
            return;
        };
        let atlas_height = self.gui.atlas.height;
        let slot_x = (MOD_TEXTURE_SLOTS as u32 - 1) * MOD_TEXTURE_SIDE;
        for def in sprites {
            let (col, row) = (
                def.id % PARTICLE_CELLS_PER_ROW,
                def.id / PARTICLE_CELLS_PER_ROW,
            );
            let (cx, cy) = (col * PARTICLE_CELL, row * PARTICLE_CELL);
            let side = MOD_TEXTURE_SIDE as f32;
            let (du, dv) = (sheet_rect.u1 - sheet_rect.u0, sheet_rect.v1 - sheet_rect.v0);
            let rect = horizon_client::hud::Rect {
                u0: sheet_rect.u0 + du * cx as f32 / side,
                v0: sheet_rect.v0 + dv * cy as f32 / side,
                u1: sheet_rect.u0 + du * (cx + def.width) as f32 / side,
                v1: sheet_rect.v0 + dv * (cy + def.height) as f32 / side,
                w: def.width,
                h: def.height,
            };
            self.particles.register_sprite(&def.name, rect);
            let patch = Patch {
                x: slot_x + cx,
                y: atlas_height - MOD_TEXTURE_SIDE + cy,
                width: def.width,
                height: def.height,
                rgba: def.pixels,
            };
            self.mods
                .particle_store
                .retain(|p| (p.x, p.y) != (patch.x, patch.y));
            self.mods.particle_store.push(patch.clone());
            self.mods.particle_patches.push(patch);
            self.mods.atlas_resend = RESEND_FRAMES;
            self.mods.particle_names.insert(def.id, def.name);
        }
        for (p, rotation, style) in particles {
            if let Some(mut m) = self.mod_particle_of(&p) {
                m.rotation = match rotation {
                    horizon_mods::ParticleRotation::Camera => 0,
                    horizon_mods::ParticleRotation::RelativeVelocity => 1,
                    horizon_mods::ParticleRotation::CopyCamera => 2,
                    horizon_mods::ParticleRotation::Vertical => 3,
                    horizon_mods::ParticleRotation::Crossed => 4,
                };
                m.bounciness = style.bounciness;
                m.opaque = matches!(style.render_type, horizon_mods::ParticleRenderType::Opaque);
                m.distance_scaling = style.distance_scaling;
                self.particles.add_mod(&m);
            }
        }
    }
    /// The sprites of the render assets for `particles.get-game-sprite`
    /// (API 0.1.19).
    pub(super) fn sprite_source(
        render: Arc<horizon_client::assets::RenderAssets>,
        gui: Arc<HashMap<String, (u32, u32, Vec<u8>)>>,
    ) -> horizon_mods::SpriteSource {
        Arc::new(move |name: &str| {
            // The weather and the particles of the GUI atlas.
            let short = name.strip_prefix("minecraft:").unwrap_or(name);
            if let Some((w, h, pixels)) = gui.get(short) {
                return Some((*w, *h, pixels.clone()));
            }
            let layer = *render.sprite_layers.get(name)? as usize;
            let side = render.sprite_size as usize;
            let layer_bytes = side * side * 4;
            let from = layer * layer_bytes;
            let pixels = render.pixels.get(from..from + layer_bytes)?.to_vec();
            Some((side as u32, side as u32, pixels))
        })
    }
    /// The blocks the mods' particles fly through, with the tags resolved
    /// (`world.set-weather-passthrough`, API 0.1.25).
    pub(super) fn sync_weather_passthrough(&mut self) {
        if !self.mods.mods.is_running() {
            if !self.particles.passthrough.is_empty() {
                self.particles.passthrough.clear();
                self.mods.passthrough_last.clear();
            }
            return;
        }
        let wanted = self.mods.mods.weather_passthrough();
        if wanted == self.mods.passthrough_last {
            return;
        }
        let mut set = std::collections::HashSet::new();
        for entry in &wanted {
            if let Some(tag) = entry.strip_prefix('#') {
                if let Some(blocks) = self.res.items().block_tags.get(tag) {
                    set.extend(blocks.iter().cloned());
                }
            } else {
                set.insert(entry.clone());
            }
        }
        self.particles.passthrough = set;
        self.mods.passthrough_last = wanted;
    }
    /// `world.fall-block` (API 0.1.24): the block leaves the world and
    /// falls as a falling block.
    fn mod_fall_block(&mut self, pos: [i32; 3], velocity: [f64; 3], drops: bool, place: bool) {
        use horizon_core::geometry::Vec3;
        if self.remote.is_some() {
            return;
        }
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let facts = self.res.facts().get(state).clone();
        if facts.hardness < 0.0 || self.res.facts().is_air(state) {
            return;
        }
        if drops {
            let held = self.held().cloned();
            let harvest = !facts.requires_tool
                || self
                    .res
                    .items()
                    .correct_for_drops(held.as_ref(), &facts.name);
            if harvest {
                for d in self.block_loot_at(pos, state, held.as_ref()) {
                    if &*d.item != "minecraft:air" {
                        self.pop_resource(pos, d);
                    }
                }
            }
        }
        let air = self.world.air;
        if self.level_set(pos, air).is_err() {
            return;
        }
        let start = horizon_core::position::BlockPos::new(pos[0], pos[1], pos[2]);
        let p = Vec3::new(
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]),
            f64::from(pos[2]) + 0.5,
        );
        if !place {
            self.mods.vanish_falls.insert(pos);
        }
        self.falling.push(super::FallingBlock {
            id: 0,
            state,
            start_pos: start,
            position: p,
            previous: p,
            velocity: Vec3::new(velocity[0], velocity[1], velocity[2]),
            age: 0,
            no_gravity: false,
            // As `FallingBlockEntity.dropItem`: a block that cannot be placed where it
            // lands (another one is there) drops as an item instead of vanishing.
            drop_item: place,
            fall_distance: 0.0,
            stuck: Vec3::ZERO,
            damage: Default::default(),
            saved: None,
        });
    }
    /// `world.set-block` (API 0.1.24).
    fn mod_set_block(&mut self, pos: [i32; 3], id: &str, properties: &[(String, String)]) {
        if self.remote.is_some() {
            return;
        }
        let registry = self.pack.assets.registry;
        let Some(default) = registry.default_state(id) else {
            return;
        };
        let Some(mut state) = registry.state(default).cloned() else {
            return;
        };
        for (k, v) in properties {
            if !state.properties.contains_key(k) {
                return;
            }
            state.properties.insert(k.clone(), v.clone());
        }
        let Ok(resolved) = registry.resolve(&state) else {
            return;
        };
        let _ = self.place_state(pos, resolved, false);
    }
    /// `world.spawn-item` (API 0.1.24).
    fn mod_spawn_item(&mut self, pos: [i32; 3], id: &str, count: u32) {
        if self.remote.is_some() || self.res.items().get(id).is_none() {
            return;
        }
        let mut stack = horizon_client::items::ItemStack::new(id, 1);
        stack.count = count.min(self.max_stack(&stack).max(1));
        self.pop_resource(pos, stack);
    }
    /// `world.merge-drops-at` (API 0.1.23): the items and experience dropped
    /// in the last few ticks within 16 blocks of `pos` gather at its middle.
    fn mod_merge_drops(&mut self, pos: [i32; 3]) {
        if self.remote.is_some() {
            return;
        }
        let middle = horizon_core::geometry::Vec3::new(
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.25,
            f64::from(pos[2]) + 0.5,
        );
        let near = |v: horizon_core::geometry::Vec3| {
            let (dx, dy, dz) = (v.x - middle.x, v.y - middle.y, v.z - middle.z);
            dx * dx + dy * dy + dz * dz <= 16.0 * 16.0
        };
        for i in &mut self.dropped {
            if i.age <= 5 && near(i.position) {
                i.position = middle;
                i.previous = middle;
                i.velocity = horizon_core::geometry::Vec3::ZERO;
            }
        }
        for o in &mut self.orbs {
            if o.age <= 5 && near(o.position) {
                o.position = middle;
                o.previous = middle;
                o.velocity = horizon_core::geometry::Vec3::ZERO;
            }
        }
    }
    /// The mouse while playing, for `input` (API 0.1.23).
    pub(super) fn mod_play_mouse(&mut self, button: i32, pressed: bool, wheel: f32) {
        if !self.mods.mods.is_running() || !self.mods.mods.uses(Capability::Input) {
            return;
        }
        let (x, y) = self.cursor;
        self.mods.mods.push_mouse_event(horizon_mods::MouseEvent {
            wheel,
            button,
            pressed,
            x,
            y,
        });
    }
    /// The weather and particle pictures of the GUI atlas by name
    /// (`environment/rain`, `particle/splash_0`), cut to their own size, for
    /// `particles.get-game-sprite` (API 0.1.22).
    pub(super) fn mod_gui_sprites(&self) -> Arc<HashMap<String, (u32, u32, Vec<u8>)>> {
        let atlas = &self.gui.atlas;
        let (aw, ah) = (atlas.width as usize, atlas.height as usize);
        let mut out = HashMap::new();
        for (name, r) in &atlas.sprites {
            if !(name.starts_with("environment/") || name.starts_with("particle/")) {
                continue;
            }
            let (x0, y0) = (
                (r.u0 * aw as f32).round() as usize,
                (r.v0 * ah as f32).round() as usize,
            );
            let (w, h) = (r.w as usize, r.h as usize);
            if w == 0 || h == 0 || x0 + w > aw || y0 + h > ah {
                continue;
            }
            let mut pixels = Vec::with_capacity(w * h * 4);
            for y in 0..h {
                let from = ((y0 + y) * aw + x0) * 4;
                let Some(row) = atlas.pixels.get(from..from + w * 4) else {
                    break;
                };
                pixels.extend_from_slice(row);
            }
            if pixels.len() == w * h * 4 {
                out.insert(name.clone(), (w as u32, h as u32, pixels));
            }
        }
        Arc::new(out)
    }
    /// A mod's particle as the engine takes it; none for a picture that is
    /// not in the atlas (yet).
    fn mod_particle_of(
        &self,
        p: &horizon_mods::ParticleOptions,
    ) -> Option<horizon_client::particles::ModParticle> {
        use horizon_client::particles::{ModFade, ModParticle, ModSize};
        let name = self.mods.particle_names.get(&p.sprite)?;
        Some(ModParticle {
            sprite_name: name.clone(),
            sprite: p.sprite,
            position: [p.x, p.y, p.z],
            velocity: [p.vx, p.vy, p.vz],
            rgba: [p.color.r, p.color.g, p.color.b, p.color.a],
            size: p.size,
            lifetime: p.lifetime as i32,
            gravity: p.gravity,
            wind: [p.wind_x, p.wind_z],
            collide: p.collide,
            size_curve: match p.size_curve {
                horizon_mods::ParticleSizeCurve::Constant => ModSize::Constant,
                horizon_mods::ParticleSizeCurve::Grow => ModSize::Grow,
                horizon_mods::ParticleSizeCurve::Shrink => ModSize::Shrink,
            },
            fade: match p.fade {
                horizon_mods::ParticleFade::None => ModFade::None,
                horizon_mods::ParticleFade::Late => ModFade::Late,
                horizon_mods::ParticleFade::Full => ModFade::Full,
            },
            flat: p.flat,
            linger: p.linger,
            sway: p.sway,
            sway_period: p.sway_period,
            spin: p.spin,
            tumble: p.tumble,
            rotation: 0,
            bounciness: 0.0,
            opaque: false,
            distance_scaling: 0.0,
        })
    }
    /// Whether `name` is a mod's command particle.
    pub(super) fn command_particle_exists(&self, name: &str) -> bool {
        self.mods.command_particles.contains_key(name)
    }
    /// `/particle <mod id>:<name>` (API 0.1.19): spawns the mod's particle
    /// at `pos` with `velocity`; false when the name is not a mod's.
    pub(super) fn spawn_command_particle(
        &mut self,
        name: &str,
        pos: [f64; 3],
        velocity: [f64; 3],
    ) -> bool {
        let Some(template) = self.mods.command_particles.get(name).cloned() else {
            return false;
        };
        if let Some(mut m) = self.mod_particle_of(&template) {
            m.position = pos;
            m.velocity = velocity;
            self.particles.add_mod(&m);
        }
        true
    }
    /// Keeps the commands' particle names (`particles.register-command-
    /// particle`) in the registry the `/particle` argument reads.
    fn sync_command_particles(&mut self) {
        let names: HashMap<String, horizon_mods::ParticleOptions> =
            self.mods.mods.command_particles().into_iter().collect();
        if names.is_empty() && self.mods.command_particles.is_empty() {
            return;
        }
        let entry = self
            .ui
            .registries
            .by_registry
            .entry("minecraft:particle_type".to_owned())
            .or_default();
        if entry.ids.is_empty() {
            entry.ids = horizon_client::particles::ParticleType::ALL
                .iter()
                .map(|t| format!("minecraft:{}", t.name()))
                .collect();
        }
        let old: Vec<String> = self.mods.command_particles.keys().cloned().collect();
        entry.ids.retain(|id| !old.contains(id));
        entry.ids.extend(names.keys().cloned());
        self.mods.command_particles = names;
    }
    /// The particle pictures that wait to go into the atlas.
    pub(super) fn flush_mod_particle_sprites(&mut self, batch: &mut HudBatch) {
        batch.patches.append(&mut self.mods.particle_patches);
        // For a while after a change the atlas gets everything again, so a
        // frame that had no atlas texture to take it does not lose it.
        if self.mods.atlas_resend > 0 {
            self.mods.atlas_resend -= 1;
            batch
                .patches
                .extend(self.mods.particle_store.iter().cloned());
            self.mods.overlay_versions.clear();
        }
    }
    /// The mods' haze in the fog (API 0.1.6): the fog closes in and takes the
    /// colour of the strongest.
    pub(super) fn apply_mod_haze(&self, fog: &mut horizon_client::fog::FogData) {
        let Some(h) = self
            .mods
            .mods
            .hazes()
            .into_iter()
            .max_by(|a, b| a.amount.total_cmp(&b.amount))
        else {
            return;
        };
        fog.environmental_end *= 1.0 - h.density;
        // The haze takes the fog's colour: each part moves towards the
        // haze's own, the fog's alpha stays.
        let haze = [h.red, h.green, h.blue];
        for (i, part) in fog.color.iter_mut().enumerate().take(3) {
            *part += (haze[i] - *part) * h.amount;
        }
    }
}

/// The camera of a frame: where it is, where it looks and its field of view.
#[derive(Clone, Copy, Debug)]
pub(super) struct Projection {
    camera: glam::DVec3,
    yaw: f32,
    pitch: f32,
    fov: f32,
}
/// The overlay's images take the atlas slots after the screen's.
const OVERLAY_FIRST_SLOT: usize = 8;

impl Game<'_> {
    /// The camera the overlay's world elements are projected with.
    pub(super) fn set_mod_projection(
        &mut self,
        camera: glam::DVec3,
        yaw: f32,
        pitch: f32,
        fov: f32,
    ) {
        self.mods.projection = Some(Projection {
            camera,
            yaw,
            pitch,
            fov,
        });
    }
    /// A point of the world in GUI pixels; none behind the camera.
    fn project_to_gui(&self, batch: &HudBatch, p: [f64; 3]) -> Option<(f32, f32)> {
        let c = self.mods.projection?;
        let rel = glam::Vec3::new(
            (p[0] - c.camera.x) as f32,
            (p[1] - c.camera.y) as f32,
            (p[2] - c.camera.z) as f32,
        );
        let v = horizon_client::camera::view_matrix(c.yaw, c.pitch) * rel.extend(1.0);
        let depth = -v.z;
        if depth <= 0.05 {
            return None;
        }
        let tan = (c.fov.to_radians() / 2.0).tan();
        let aspect = batch.width / batch.height.max(1.0);
        let ndc = (v.x / depth / (tan * aspect), v.y / depth / tan);
        if ndc.0.abs() > 1.5 || ndc.1.abs() > 1.5 {
            return None;
        }
        Some((
            (ndc.0 * 0.5 + 0.5) * batch.width,
            (1.0 - (ndc.1 * 0.5 + 0.5)) * batch.height,
        ))
    }
    /// The atlas slot of a mod's overlay image, with the image uploaded.
    fn overlay_slot(&mut self, mod_id: &str, id: u32) -> Option<usize> {
        let key = (mod_id.to_owned(), id);
        if let Some(slot) = self.mods.overlay_slots.get(&key) {
            return Some(*slot);
        }
        let slot = OVERLAY_FIRST_SLOT + self.mods.overlay_slots.len();
        if slot >= MOD_TEXTURE_SLOTS - 1 {
            return None;
        }
        self.mods.overlay_slots.insert(key, slot);
        Some(slot)
    }
    /// What the mods painted in a layer of the HUD (API 0.1.7).
    pub(super) fn draw_mod_overlay(
        &mut self,
        batch: &mut HudBatch,
        layer: horizon_mods::OverlayLayer,
    ) {
        if !self.mods.mods.is_running() || !self.mods.mods.uses(Capability::Overlay) {
            return;
        }
        self.sync_mod_images(batch);
        let layers: Vec<(String, Vec<ScreenElement>)> = self
            .mods
            .mods
            .overlay_layer(layer)
            .into_iter()
            .map(|(id, e)| (id.to_owned(), e.to_vec()))
            .collect();
        for (mod_id, elements) in layers {
            self.draw_mod_elements_of(batch, Some(&mod_id), &elements);
        }
    }
    /// The boxes the mods drew in the world as lines (API 0.1.23): the ones
    /// behind blocks hidden by them first, those through blocks second.
    pub(super) fn mod_world_boxes(
        &self,
    ) -> (
        Vec<horizon_client::gizmos::Line>,
        Vec<horizon_client::gizmos::Line>,
    ) {
        use horizon_client::gizmos::Line;
        let (mut hidden, mut over) = (Vec::new(), Vec::new());
        if !self.mods.mods.is_running() || !self.mods.mods.uses(Capability::Overlay) {
            return (hidden, over);
        }
        for (_, elements) in self.mods.mods.overlay_world() {
            for e in elements {
                let horizon_mods::WorldElement::Box(b) = e else {
                    continue;
                };
                let (x0, y0, z0) = (b.x - 0.5, b.y - 0.5, b.z - 0.5);
                let (x1, y1, z1) = (b.x + 0.5, b.y + 0.5, b.z + 0.5);
                let corners = |i: usize| {
                    [
                        if i & 1 == 0 { x0 } else { x1 },
                        if i & 2 == 0 { y0 } else { y1 },
                        if i & 4 == 0 { z0 } else { z1 },
                    ]
                };
                let target = if b.through_blocks {
                    &mut over
                } else {
                    &mut hidden
                };
                for a in 0..8usize {
                    for bit in [1usize, 2, 4] {
                        let c = a | bit;
                        if c != a {
                            target.push(Line {
                                start: corners(a),
                                end: corners(c),
                                color: b.color,
                                width: b.width.clamp(1.0, 8.0),
                            });
                        }
                    }
                }
            }
        }
        (hidden, over)
    }
    /// What the mods drew at points of the world (API 0.1.7).
    pub(super) fn draw_mod_world(&mut self, batch: &mut HudBatch) {
        if !self.mods.mods.is_running() || !self.mods.mods.uses(Capability::Overlay) {
            return;
        }
        let worlds: Vec<(String, horizon_mods::WorldElement)> = self
            .mods
            .mods
            .overlay_world()
            .into_iter()
            .flat_map(|(id, e)| e.iter().map(move |e| (id.to_owned(), e.clone())))
            .collect();
        for (mod_id, e) in worlds {
            use horizon_mods::WorldElement as W;
            match e {
                W::Label(l) => {
                    let Some((x, y)) = self.project_to_gui(batch, [l.x, l.y, l.z]) else {
                        continue;
                    };
                    let w = HudBatch::text_width(&self.gui.atlas, &l.text) * l.scale.max(0.1);
                    let start = batch.vertices.len();
                    batch.text(
                        &self.gui.atlas,
                        &l.text,
                        x - w / 2.0 / l.scale.max(0.1),
                        y + l.offset_y,
                        argb(l.color),
                        l.shadow,
                    );
                    if l.scale > 0.0 && l.scale != 1.0 && l.scale.is_finite() {
                        batch.scale_since(start, x, y + l.offset_y, l.scale, l.scale);
                    }
                }
                W::Sprite(s) => {
                    let Some((x, y)) = self.project_to_gui(batch, [s.x, s.y, s.z]) else {
                        continue;
                    };
                    batch.sprite_scaled(
                        &self.gui.atlas,
                        &s.name,
                        x - s.width / 2.0,
                        y - s.height / 2.0 + s.offset_y,
                        s.width,
                        s.height,
                        argb(s.tint),
                    );
                }
                W::Picture(p) => {
                    let Some((x, y)) = self.project_to_gui(batch, [p.x, p.y, p.z]) else {
                        continue;
                    };
                    let Some(slot) = self
                        .mods
                        .overlay_slots
                        .get(&(mod_id.clone(), p.texture))
                        .copied()
                    else {
                        continue;
                    };
                    let Some(rect) = self
                        .gui
                        .atlas
                        .sprites
                        .get(&format!("horizon/mod-texture{slot}"))
                        .copied()
                    else {
                        continue;
                    };
                    let side = MOD_TEXTURE_SIDE as f32;
                    let (du, dv) = (rect.u1 - rect.u0, rect.v1 - rect.v0);
                    let part = horizon_client::hud::Rect {
                        u0: rect.u0 + du * (p.u / side),
                        v0: rect.v0 + dv * (p.v / side),
                        u1: rect.u0 + du * ((p.u + p.u_width) / side),
                        v1: rect.v0 + dv * ((p.v + p.v_height) / side),
                        w: p.u_width.max(1.0) as u32,
                        h: p.v_height.max(1.0) as u32,
                    };
                    batch.rect_uv(
                        x - p.width / 2.0,
                        y - p.height / 2.0 + p.offset_y,
                        p.width,
                        p.height,
                        part,
                        argb(p.tint),
                    );
                }
                // Drawn as lines by `mod_world_boxes`.
                W::Box(_) => {}
                W::Fill(f) => {
                    let Some((x, y)) = self.project_to_gui(batch, [f.x, f.y, f.z]) else {
                        continue;
                    };
                    batch.fill(
                        &self.gui.atlas,
                        x - f.width / 2.0,
                        y - f.height / 2.0 + f.offset_y,
                        f.width,
                        f.height,
                        argb(f.color),
                    );
                }
            }
        }
    }
}

/// The entities of the world for mods (API 0.1.8).
impl Game<'_> {
    /// Mobs, items on the ground and experience orbs within 512 blocks of
    /// the player, nearest first, as of this tick.
    fn mod_entities(&self) -> Vec<horizon_mods::WorldEntity> {
        use horizon_mods::{EntityKind, WorldEntity};
        let Some(p) = self.player.as_ref() else {
            return Vec::new();
        };
        let origin = p.position;
        let dist2 = |v: horizon_core::geometry::Vec3| {
            let (dx, dy, dz) = (v.x - origin.x, v.y - origin.y, v.z - origin.z);
            dx * dx + dy * dy + dz * dz
        };
        let near = |v: horizon_core::geometry::Vec3| dist2(v) <= 512.0 * 512.0;
        let mut found: Vec<(f64, WorldEntity)> = Vec::new();
        for m in &self.mobs {
            if !near(m.position) {
                continue;
            }
            let name = m.custom_name.clone().unwrap_or_else(|| {
                let key = format!("entity.{}", m.kind.id.replace(':', "."));
                horizon_client::lang::get_or(&key, &m.kind.id)
            });
            let player = self
                .remote
                .as_ref()
                .and_then(|r| r.level_player(m.id as i32));
            let (kind, name, uuid) = match player {
                Some((uuid, profile)) => (
                    EntityKind::Player,
                    profile.unwrap_or(name),
                    format!("{uuid:032x}"),
                ),
                None => (EntityKind::Mob, name, String::new()),
            };
            found.push((
                dist2(m.position),
                WorldEntity {
                    id: m.id,
                    type_id: m.kind.id.clone(),
                    kind,
                    name,
                    uuid,
                    x: m.position.x,
                    y: m.position.y,
                    z: m.position.z,
                    yaw: m.yaw,
                    height: m.kind.height as f32,
                    health: m.health,
                    max_health: m.attrs.max_health,
                    xp: 0,
                    on_fire: m.fire_ticks > 0,
                    hostile: m.hostile(),
                    held_item: m.extra.main_hand.as_ref().map(|s| s.to_string()),
                    off_hand_item: m.extra.off_hand.as_ref().map(|s| s.to_string()),
                },
            ));
        }
        for i in &self.dropped {
            if !near(i.position) {
                continue;
            }
            let item = i.stack.item.to_string();
            found.push((
                dist2(i.position),
                WorldEntity {
                    id: i.id,
                    type_id: "minecraft:item".into(),
                    kind: EntityKind::Item,
                    name: self.pack.names.item(&item),
                    uuid: String::new(),
                    x: i.position.x,
                    y: i.position.y,
                    z: i.position.z,
                    yaw: 0.0,
                    height: 0.25,
                    health: 0.0,
                    max_health: 0.0,
                    xp: 0,
                    on_fire: i.fire_ticks > 0,
                    hostile: false,
                    held_item: Some(item),
                    off_hand_item: None,
                },
            ));
        }
        for o in &self.orbs {
            if !near(o.position) {
                continue;
            }
            found.push((
                dist2(o.position),
                WorldEntity {
                    id: o.id,
                    type_id: "minecraft:experience_orb".into(),
                    kind: EntityKind::Other,
                    name: horizon_client::lang::get_or(
                        "entity.minecraft.experience_orb",
                        "Experience Orb",
                    ),
                    uuid: String::new(),
                    x: o.position.x,
                    y: o.position.y,
                    z: o.position.z,
                    yaw: 0.0,
                    height: 0.5,
                    health: 0.0,
                    max_health: 0.0,
                    xp: o.value,
                    on_fire: false,
                    hostile: false,
                    held_item: None,
                    off_hand_item: None,
                },
            ));
        }
        for (i, a) in self.creatures.projectiles.iter().enumerate() {
            if !near(a.position) {
                continue;
            }
            let (type_id, item) = projectile_type(&a.kind);
            found.push((
                dist2(a.position),
                WorldEntity {
                    id: tracked_or_synthetic(a.id, 0, i),
                    type_id: type_id.into(),
                    kind: EntityKind::Other,
                    name: horizon_client::lang::get_or(
                        &format!("entity.{}", type_id.replace(':', ".")),
                        type_id,
                    ),
                    uuid: String::new(),
                    x: a.position.x,
                    y: a.position.y,
                    z: a.position.z,
                    yaw: a.yaw,
                    height: 0.25,
                    health: 0.0,
                    max_health: 0.0,
                    xp: 0,
                    on_fire: matches!(
                        a.kind,
                        horizon_client::projectiles::ProjectileKind::SmallFireball
                            | horizon_client::projectiles::ProjectileKind::LargeFireball { .. }
                    ),
                    hostile: false,
                    held_item: item.map(str::to_owned),
                    off_hand_item: None,
                },
            ));
        }
        for (i, t) in self.tnts.iter().enumerate() {
            if !near(t.position) {
                continue;
            }
            found.push((
                dist2(t.position),
                WorldEntity {
                    id: tracked_or_synthetic(t.id, 1, i),
                    type_id: "minecraft:tnt".into(),
                    kind: EntityKind::Other,
                    name: horizon_client::lang::get_or("entity.minecraft.tnt", "Primed TNT"),
                    uuid: String::new(),
                    x: t.position.x,
                    y: t.position.y,
                    z: t.position.z,
                    yaw: 0.0,
                    height: 0.98,
                    health: 0.0,
                    max_health: 0.0,
                    xp: 0,
                    on_fire: false,
                    hostile: false,
                    held_item: Some("minecraft:tnt".into()),
                    off_hand_item: None,
                },
            ));
        }
        for f in &self.item_frames {
            let at = horizon_core::geometry::Vec3::new(
                f64::from(f.pos[0]) + 0.5,
                f64::from(f.pos[1]) + 0.5,
                f64::from(f.pos[2]) + 0.5,
            );
            if !near(at) {
                continue;
            }
            let type_id = if f.glow {
                "minecraft:glow_item_frame"
            } else {
                "minecraft:item_frame"
            };
            found.push((
                dist2(at),
                WorldEntity {
                    id: f.id,
                    type_id: type_id.into(),
                    kind: EntityKind::Other,
                    name: horizon_client::lang::get_or(
                        &format!("entity.{}", type_id.replace(':', ".")),
                        "Item Frame",
                    ),
                    uuid: String::new(),
                    x: at.x,
                    y: at.y,
                    z: at.z,
                    yaw: 0.0,
                    height: 0.75,
                    health: 0.0,
                    max_health: 0.0,
                    xp: 0,
                    on_fire: false,
                    hostile: false,
                    held_item: f.item.as_ref().map(|s| s.item.to_string()),
                    off_hand_item: None,
                },
            ));
        }
        if !p.spectator {
            let feet = p.position;
            let held = |slot: usize| {
                self.inventory
                    .get(slot)
                    .into_iter()
                    .flatten()
                    .next()
                    .map(|s| s.item.to_string())
            };
            found.push((
                0.0,
                WorldEntity {
                    id: u32::MAX,
                    type_id: "minecraft:player".into(),
                    kind: EntityKind::Player,
                    name: String::new(),
                    uuid: String::new(),
                    x: feet.x,
                    y: feet.y,
                    z: feet.z,
                    yaw: self.yaw,
                    height: p.dimensions.height,
                    health: 0.0,
                    max_health: 0.0,
                    xp: 0,
                    on_fire: p.fire_ticks > 0,
                    hostile: false,
                    held_item: held(self.selected),
                    off_hand_item: held(super::shield::OFFHAND_SLOT),
                },
            ));
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found.truncate(1024);
        found.into_iter().map(|(_, e)| e).collect()
    }
    /// `entity.remove`: a mob goes at once, an item too, an orb expires.
    fn mod_remove_entity(&mut self, id: u32) {
        if self.remote.is_some() {
            return;
        }
        self.mobs.retain(|m| m.id != id);
        self.dropped.retain(|i| i.id != id);
        for o in &mut self.orbs {
            if o.id == id {
                o.age = u32::MAX / 2;
            }
        }
    }
}

/// The id of an entity the game does not track (0): a synthetic one above
/// 2^31 from its kind of entity and its place in the list, valid for one tick.
fn tracked_or_synthetic(id: u32, class: u32, index: usize) -> u32 {
    if id != 0 {
        id
    } else {
        0x8000_0000 | (class << 24) | (index as u32 & 0x00ff_ffff)
    }
}
/// The type id of a projectile and the item it is drawn as.
fn projectile_type(
    kind: &horizon_client::projectiles::ProjectileKind,
) -> (&'static str, Option<&'static str>) {
    use horizon_client::projectiles::ProjectileKind as K;
    match kind {
        K::Arrow { .. } => ("minecraft:arrow", Some("minecraft:arrow")),
        K::Snowball => ("minecraft:snowball", Some("minecraft:snowball")),
        K::Egg => ("minecraft:egg", Some("minecraft:egg")),
        K::EnderPearl => ("minecraft:ender_pearl", Some("minecraft:ender_pearl")),
        K::SmallFireball => ("minecraft:small_fireball", Some("minecraft:fire_charge")),
        K::LargeFireball { .. } => ("minecraft:fireball", Some("minecraft:fire_charge")),
        K::LlamaSpit => ("minecraft:llama_spit", None),
        K::WindCharge => ("minecraft:wind_charge", Some("minecraft:wind_charge")),
        K::BreezeWindCharge => (
            "minecraft:breeze_wind_charge",
            Some("minecraft:wind_charge"),
        ),
        K::ShulkerBullet { .. } => ("minecraft:shulker_bullet", None),
        K::WitherSkull { .. } => ("minecraft:wither_skull", None),
        K::Potion { .. } => ("minecraft:splash_potion", Some("minecraft:splash_potion")),
    }
}
/// `entity.get-detail` and `light-events` (API 0.1.20).
impl Game<'_> {
    /// `entity.get-types` (API 0.1.33): the registry's entity types with their
    /// names and tags, sorted by id.
    fn mod_entity_types(&self) -> Arc<Vec<horizon_mods::EntityType>> {
        self.mods
            .entity_types
            .get_or_init(|| {
                let root = &self.options.resources;
                let mut ids: Vec<String> = std::fs::read(root.join("entity-types.json"))
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
                    .unwrap_or_default();
                ids.sort();
                let tags = &self.res.rest().mob_data.entity_tags;
                Arc::new(
                    ids.into_iter()
                        .map(|id| {
                            let key = format!("entity.{}", id.replace(':', "."));
                            let name = horizon_client::lang::get_or(&key, "");
                            let mut tags: Vec<String> = tags
                                .iter()
                                .filter(|(_, members)| members.contains(&id))
                                .map(|(tag, _)| tag.clone())
                                .collect();
                            tags.sort();
                            horizon_mods::EntityType { id, name, tags }
                        })
                        .collect(),
                )
            })
            .clone()
    }
    /// The extra state of the mobs and primed TNT among `entities`, by id.
    fn mod_entity_details(
        &self,
        entities: &[horizon_mods::WorldEntity],
    ) -> std::collections::HashMap<u32, horizon_mods::EntityDetail> {
        use horizon_mods::EntityDetail;
        let mut out = std::collections::HashMap::new();
        for e in entities {
            if let Some(m) = self
                .mobs
                .iter()
                .find(|m| m.id == e.id && e.id < 0x8000_0000)
            {
                let carried = m
                    .extra
                    .carried
                    .map(|s| self.res.facts().get(s).name.to_string())
                    .unwrap_or_default();
                let cube = m.extra.size > 0;
                out.insert(
                    e.id,
                    EntityDetail {
                        swell: if m.kind.id == "minecraft:creeper" {
                            (m.swell as f32 / 28.0).clamp(0.0, 1.0)
                        } else {
                            0.0
                        },
                        glowing: m.glowing,
                        in_water: m.was_eye_in_water,
                        squish: if cube { m.anim.v[0].max(0.0) } else { 0.0 },
                        dark_ticks: 0,
                        carried_block: carried,
                    },
                );
            }
        }
        for (i, t) in self.tnts.iter().enumerate() {
            let id = tracked_or_synthetic(t.id, 1, i);
            if entities.iter().any(|e| e.id == id) {
                out.insert(
                    id,
                    EntityDetail {
                        swell: (t.fuse as f32 / 80.0).clamp(0.0, 1.0),
                        glowing: false,
                        in_water: false,
                        squish: 0.0,
                        dark_ticks: 0,
                        carried_block: String::new(),
                    },
                );
            }
        }
        out
    }
    /// The faces (the 8 by 8 face with the hat over it) of the players
    /// among `entities` whose skin was downloaded, by UUID.
    fn mod_faces(
        &mut self,
        entities: &[horizon_mods::WorldEntity],
    ) -> std::collections::HashMap<String, horizon_mods::Face> {
        let mut out = std::collections::HashMap::new();
        if !entities.iter().any(|e| !e.uuid.is_empty()) {
            return out;
        }
        let Some((players, _, _)) = self.remote_tab_players() else {
            return out;
        };
        for e in entities.iter().filter(|e| !e.uuid.is_empty()) {
            let Ok(uuid) = u128::from_str_radix(&e.uuid, 16) else {
                continue;
            };
            let Some(player) = players.iter().find(|p| p.uuid == uuid) else {
                continue;
            };
            let Some(pixels) = self.skin_pixels.get(&player.skin) else {
                continue;
            };
            if pixels.len() < 64 * 64 * 4 {
                continue;
            }
            let mut face = vec![0u8; 8 * 8 * 4];
            for y in 0..8usize {
                for x in 0..8usize {
                    let at = |sx: usize| ((8 + y) * 64 + sx + x) * 4;
                    let (base, hat) = (at(8), at(40));
                    let src = if pixels[hat + 3] > 0 { hat } else { base };
                    face[(y * 8 + x) * 4..][..4].copy_from_slice(&pixels[src..src + 4]);
                }
            }
            out.insert(
                e.uuid.clone(),
                horizon_mods::Face {
                    width: 8,
                    height: 8,
                    rgba: face,
                },
            );
        }
        out
    }
    /// What shines in the world besides entities: standing beacon beams
    /// and guardian beams within 128 blocks of the player, and the
    /// fireflies and sonic booms spawned since the last call.
    fn mod_flashes(&mut self) -> Vec<horizon_mods::LightFlash> {
        use horizon_client::containers::BlockEntity;
        use horizon_mods::LightFlash;
        let Some(p) = self.player.as_ref() else {
            return Vec::new();
        };
        let origin = p.position;
        let within = |x: f64, y: f64, z: f64| {
            let (dx, dy, dz) = (x - origin.x, y - origin.y, z - origin.z);
            dx * dx + dy * dy + dz * dz <= 128.0 * 128.0
        };
        let mut out = Vec::new();
        for (&(x, y, z), entity) in &self.world.block_entities {
            let BlockEntity::Beacon(b) = entity else {
                continue;
            };
            let height: i32 = b.sections.iter().map(|(_, h)| *h).sum();
            let (fx, fy, fz) = (f64::from(x) + 0.5, f64::from(y) + 1.0, f64::from(z) + 0.5);
            if height > 0 && within(fx, fy, fz) {
                out.push(LightFlash {
                    kind: "minecraft:beacon_beam".into(),
                    x: fx,
                    y: fy,
                    z: fz,
                    end_x: fx,
                    end_y: fy + f64::from(height.min(1024)),
                    end_z: fz,
                    ticks: 2,
                });
            }
        }
        for m in &self.mobs {
            if horizon_client::mobs::guardian::beam(m, 1.0).is_none()
                || !within(m.position.x, m.position.y, m.position.z)
            {
                continue;
            }
            let Some(target) = self.beam_target(m.ai.target, 1.0) else {
                continue;
            };
            out.push(LightFlash {
                kind: "minecraft:guardian_beam".into(),
                x: m.position.x,
                y: m.position.y + m.eye_height(),
                z: m.position.z,
                end_x: target.x,
                end_y: target.y,
                end_z: target.z,
                ticks: 2,
            });
        }
        for (ty, at) in std::mem::take(&mut self.particles.lit) {
            let kind = match ty {
                horizon_client::particles::ParticleType::Firefly => "minecraft:firefly",
                _ => "minecraft:sonic_boom",
            };
            out.push(LightFlash {
                kind: kind.into(),
                x: at[0],
                y: at[1],
                z: at[2],
                end_x: at[0],
                end_y: at[1],
                end_z: at[2],
                ticks: if kind == "minecraft:firefly" { 20 } else { 10 },
            });
        }
        out.truncate(256);
        out
    }
}

impl Game<'_> {
    /// The slots of the container at `pos`, for `world.get-container`
    /// (API 0.1.9); none for a block that is not one.
    fn mod_container(&self, pos: [i32; 3]) -> Option<Vec<Option<horizon_mods::ContainerStack>>> {
        use horizon_client::containers::BlockEntity;
        let stack = |s: &Option<horizon_client::items::ItemStack>| {
            s.as_ref().map(|s| horizon_mods::ContainerStack {
                id: s.item.to_string(),
                count: s.count,
                damage: s.damage,
            })
        };
        let entity = self.world.block_entities.get(&(pos[0], pos[1], pos[2]))?;
        match entity {
            BlockEntity::Chest(items) => Some(items.iter().map(stack).collect()),
            BlockEntity::Furnace(f) => Some(f.items.iter().map(stack).collect()),
            BlockEntity::Crafter(c) => Some(c.items.iter().map(stack).collect()),
            _ => None,
        }
    }
}

/// The inventory, the crafting grid and the slot marks for mods (API 0.1.10).
impl Game<'_> {
    /// A new world was loaded: mods see another session.
    pub(super) fn new_mod_session(&mut self) {
        self.mods.session += 1;
        self.mods.session_started = Some(Instant::now());
    }
    fn mod_item_stack(stack: &horizon_client::items::ItemStack) -> horizon_mods::InventoryStack {
        horizon_mods::InventoryStack {
            id: stack.item.to_string(),
            count: stack.count,
            damage: stack.damage,
            components: stack
                .extra
                .as_ref()
                .map_or_else(String::new, |extra| format!("{extra:?}")),
        }
    }
    /// The tool in hotbar slot `slot` (`inventory.get-tools`, API 0.1.16).
    fn mod_tool(&self, slot: usize) -> Option<horizon_mods::InventoryTool> {
        let stack = self.stack_in(slot)?;
        let damage_per_block = self.res.items().get(&stack.item)?.tool_damage_per_block?;
        Some(horizon_mods::InventoryTool {
            id: stack.item.to_string(),
            has_tool: true,
            damage_per_block,
            enchantments: stack
                .enchantments()
                .iter()
                .map(|(id, level)| (id.to_string(), *level))
                .collect(),
        })
    }
    /// Every slot of the inventory by its index (`inventory.get-slots`):
    /// 0 to 35, the armour from the feet up at 36 to 39, the off hand at 40.
    fn mod_slots(&self) -> Vec<horizon_mods::InventorySlot> {
        let items = self.res.items();
        let slot = |index: usize, stack: Option<&horizon_client::items::ItemStack>| {
            horizon_mods::InventorySlot {
                index: index as u32,
                id: stack.map_or_else(String::new, |s| s.item.to_string()),
                count: stack.map_or(0, |s| s.count),
                damage: stack.map_or(0, |s| s.damage),
                max_damage: stack
                    .and_then(|s| items.get(&s.item))
                    .map_or(0, |i| i.max_damage),
            }
        };
        let mut out: Vec<_> = (0..36)
            .map(|i| slot(i, self.inventory.get(i).and_then(|s| s.as_ref())))
            .collect();
        // `armor` runs head first; the slots run from the feet.
        for i in 0..4 {
            out.push(slot(36 + i, self.armor[3 - i].as_ref()));
        }
        out.push(slot(40, self.offhand.as_ref()));
        out
    }
    /// `inventory.move-item`: moves or swaps two slots of the inventory, in a
    /// world this game runs.
    fn mod_move_item(&mut self, from: usize, to: usize) {
        let allowed = |i: usize| i < 36 || i == 40;
        if self.remote.is_some()
            || self.player.is_none()
            || from == to
            || !allowed(from)
            || !allowed(to)
        {
            return;
        }
        let moved = self.stack_in_mut(from).take();
        if moved.is_none() {
            return;
        }
        let there = std::mem::replace(self.stack_in_mut(to), moved);
        *self.stack_in_mut(from) = there;
    }
    fn mod_inventory(&self) -> horizon_mods::PlayerInventory {
        let stack =
            |s: &Option<horizon_client::items::ItemStack>| s.as_ref().map(Self::mod_item_stack);
        horizon_mods::PlayerInventory {
            slots: self.inventory.iter().take(36).map(stack).collect(),
            armour: self.armor.iter().map(stack).collect(),
            offhand: stack(&self.offhand),
            carried: stack(&self.carried),
        }
    }
    fn mod_crafting(&self) -> Option<horizon_mods::Crafting> {
        if self.screen != Screen::Inventory || self.container.is_some() || self.station.is_some() {
            return None;
        }
        let cells = self.craft_size * self.craft_size;
        Some(horizon_mods::Crafting {
            grid: self
                .craft_grid
                .iter()
                .take(cells)
                .map(|s| s.as_ref().map(Self::mod_item_stack))
                .collect(),
            output: self.craft_result().as_ref().map(Self::mod_item_stack),
        })
    }
    /// The slot under the cursor in the inventory screen, for `slots`.
    fn mod_hovered_slot(&self, batch: &HudBatch) -> Option<u32> {
        if self.screen != Screen::Inventory {
            return None;
        }
        self.slot_at(batch, self.cursor.0, self.cursor.1)
            .map(|slot| slot as u32)
    }
    /// Draws one mark at the slot's top left corner (x, y).
    fn draw_slot_mark(
        &mut self,
        batch: &mut HudBatch,
        x: f32,
        y: f32,
        mod_id: &str,
        shape: &horizon_mods::MarkShape,
    ) {
        use horizon_mods::MarkShape as M;
        match shape {
            M::Frame(f) => {
                let atlas = &self.gui.atlas;
                let c = argb(f.color);
                batch.fill(atlas, x, y, 16.0, 1.0, c);
                batch.fill(atlas, x, y + 15.0, 16.0, 1.0, c);
                batch.fill(atlas, x, y, 1.0, 16.0, c);
                batch.fill(atlas, x + 15.0, y, 1.0, 16.0, c);
            }
            M::Dot(d) => {
                let size = d.size.clamp(0.0, 16.0);
                let atlas = &self.gui.atlas;
                batch.fill(atlas, x + 15.0 - size, y + 1.0, size, size, argb(d.color));
            }
            M::Fill(f) => batch.fill(&self.gui.atlas, x, y, 16.0, 16.0, argb(f.color)),
            M::Square(q) => {
                let size = q.size.clamp(0.0, 16.0);
                batch.fill(&self.gui.atlas, x + q.x, y + q.y, size, size, argb(q.color));
            }
            M::Picture(p) => {
                let Some(slot) = self
                    .mods
                    .overlay_slots
                    .get(&(mod_id.to_owned(), p.texture))
                    .copied()
                else {
                    return;
                };
                let Some(rect) = self
                    .gui
                    .atlas
                    .sprites
                    .get(&format!("horizon/mod-texture{slot}"))
                    .copied()
                else {
                    return;
                };
                let size = self
                    .mods
                    .mods
                    .overlay_textures()
                    .into_iter()
                    .find(|(id, _)| *id == mod_id)
                    .and_then(|(_, t)| {
                        t.iter()
                            .find(|t| t.id == p.texture)
                            .map(|t| (t.width, t.height))
                    });
                let Some((tw, th)) = size else {
                    return;
                };
                let side = MOD_TEXTURE_SIDE as f32;
                let (du, dv) = (rect.u1 - rect.u0, rect.v1 - rect.v0);
                let part = horizon_client::hud::Rect {
                    u0: rect.u0,
                    v0: rect.v0,
                    u1: rect.u0 + du * (tw as f32 / side),
                    v1: rect.v0 + dv * (th as f32 / side),
                    w: tw,
                    h: th,
                };
                batch.rect_uv(x + p.x, y + p.y, p.width, p.height, part, argb(p.color));
            }
        }
    }
    /// The marks of the mods with the id of the mod that set them.
    fn owned_marks(&self, hud: bool) -> Vec<(String, horizon_mods::Mark)> {
        self.mods
            .mods
            .slot_marks_for(hud)
            .into_iter()
            .map(|(id, m)| (id.to_owned(), m.clone()))
            .collect()
    }
    /// Uploads the images the mods changed (the store of `overlay` and of
    /// the slot marks is the same) so `picture` marks can be drawn.
    fn sync_mod_images(&mut self, batch: &mut HudBatch) {
        let textures: Vec<(String, Vec<horizon_mods::Texture>)> = self
            .mods
            .mods
            .overlay_textures()
            .into_iter()
            .map(|(id, t)| (id.to_owned(), t.to_vec()))
            .collect();
        let atlas_height = self.gui.atlas.height;
        for (mod_id, list) in &textures {
            for t in list {
                let Some(slot) = self.overlay_slot(mod_id, t.id) else {
                    continue;
                };
                if self.mods.overlay_versions.get(&slot) == Some(&t.version) {
                    continue;
                }
                self.mods.overlay_versions.insert(slot, t.version);
                batch.patches.push(Patch {
                    x: slot as u32 * MOD_TEXTURE_SIDE,
                    y: atlas_height - MOD_TEXTURE_SIDE,
                    width: t.width,
                    height: t.height,
                    rgba: t.pixels.clone(),
                });
            }
        }
    }
    /// The mods' marks on the hotbar's slots of the HUD.
    pub(super) fn draw_mod_hotbar_marks(&mut self, batch: &mut HudBatch, w: i32, h: i32) {
        if !self.mods.mods.uses(Capability::SlotDecorate) {
            return;
        }
        self.sync_mod_images(batch);
        let center = w / 2;
        for (id, m) in self.owned_marks(true) {
            if m.slot < 9 {
                let x = (center - 90 + m.slot as i32 * 20 + 2) as f32;
                let y = (h - 16 - 3) as f32;
                self.draw_slot_mark(batch, x, y, &id, &m.shape);
            }
        }
    }
    /// The mods' marks on the slots of the open inventory screen.
    pub(super) fn draw_mod_inventory_marks(&mut self, batch: &mut HudBatch, ox: f32, oy: f32) {
        if !self.mods.mods.uses(Capability::SlotDecorate) {
            return;
        }
        self.sync_mod_images(batch);
        let shown = self.slots();
        for (id, m) in self.owned_marks(false) {
            let slot = m.slot as usize;
            if shown.contains(&slot) {
                let (sx, sy) = self.slot_position(slot);
                self.draw_slot_mark(batch, ox + sx, oy + sy, &id, &m.shape);
            }
        }
    }
    /// The mods' marks on the hotbar row of the creative item tab.
    pub(super) fn draw_mod_creative_marks(&mut self, batch: &mut HudBatch, ox: f32, oy: f32) {
        if !self.mods.mods.uses(Capability::SlotDecorate) {
            return;
        }
        self.sync_mod_images(batch);
        for (id, m) in self.owned_marks(false) {
            if m.slot < 9 {
                let x = ox + 9.0 + m.slot as f32 * 18.0;
                self.draw_slot_mark(batch, x, oy + 112.0, &id, &m.shape);
            }
        }
    }
}

/// Food data and the tooltips of items for mods (API 0.1.11).
impl Game<'_> {
    /// What eating each food gives, and whether it can hurt (a Hunger effect).
    fn mod_foods(&self) -> Arc<HashMap<String, (Food, bool)>> {
        use horizon_client::items::ConsumeEffect;
        let built = self.mods.foods.get_or_init(|| {
            let items = self.res.items();
            let mut foods = HashMap::new();
            for stack in self.creative_screen.search_items() {
                let id = stack.item.to_string();
                let Some(info) = items.get(&stack.item) else {
                    continue;
                };
                let Some((nutrition, saturation)) = info.food else {
                    continue;
                };
                let harmful = info.consumable.as_ref().is_some_and(|c| {
                    c.effects.iter().any(|e| {
                        matches!(e, ConsumeEffect::ApplyEffects { effects, .. }
                            if effects.iter().any(|f| f.id.trim_start_matches("minecraft:") == "hunger"))
                    })
                });
                foods.insert(id, (Food { nutrition, saturation }, harmful));
            }
            Arc::new(foods)
        });
        Arc::clone(built)
    }
    /// Draws the tooltip of a stack with what mods add to it; false when no
    /// mod adds anything (the game draws the tooltip itself).
    pub(super) fn draw_mod_tooltip(
        &mut self,
        batch: &mut HudBatch,
        stack: &horizon_client::items::ItemStack,
    ) -> bool {
        if !self.mods.mods.uses(Capability::Tooltip) {
            return false;
        }
        let item = stack.item.to_string();
        let contents: Vec<(String, horizon_mods::TooltipContent)> = self
            .mods
            .mods
            .tooltip_content(&item)
            .into_iter()
            .map(|(id, c)| (id.to_owned(), c.clone()))
            .collect();
        let mut lines = self.stack_tooltip(stack);
        let keys = self.mods.tooltip_keys.take();
        let enchantments = |levels: &[(Arc<str>, u32)], stored: bool| -> Vec<_> {
            levels
                .iter()
                .map(|(id, level)| horizon_mods::TooltipEnchantment {
                    id: id.to_string(),
                    level: *level,
                    stored,
                })
                .collect()
        };
        let mut shown = enchantments(stack.stored_enchantments(), true);
        shown.extend(enchantments(stack.enchantments(), false));
        *self.mods.tooltip_enchantments.borrow_mut() = shown;
        let hovered = hovered_stack(stack);
        let own = self
            .mods
            .mods
            .hovered_tooltip(&format!("{hovered:?}"))
            .map(|(id, t)| (id.to_owned(), t.clone()));
        *self.mods.tooltip_stack.borrow_mut() = Some(hovered);
        let added = self.mods.mods.tooltip_lines();
        // The mods' lines go in the text, right under the line of their key.
        if !added.is_empty() {
            let mut merged = Vec::with_capacity(lines.len() + added.len());
            for (i, line) in lines.into_iter().enumerate() {
                merged.push(line);
                for under in keys.iter().filter(|(at, _)| *at == i).map(|(_, k)| k) {
                    merged.extend(
                        added
                            .iter()
                            .filter(|l| l.under == *under)
                            .map(|l| (Text::legacy(&l.text), l.color)),
                    );
                }
            }
            merged.extend(
                added
                    .iter()
                    .filter(|l| l.under.is_empty())
                    .map(|l| (Text::legacy(&l.text), l.color)),
            );
            lines = merged;
        }
        let mut contents = contents;
        if let Some((mod_id, t)) = own {
            if let Some(own_lines) = t.lines {
                lines = own_lines
                    .into_iter()
                    .map(|l| (Text::legacy(&l.text), l.color))
                    .collect();
            }
            contents.push((
                mod_id,
                horizon_mods::TooltipContent {
                    width: t.width,
                    height: t.height,
                    elements: t.elements,
                },
            ));
        }
        let (width, height) = contents.iter().fold((0.0f32, 0.0f32), |(w, h), (_, c)| {
            (w.max(c.width), h + c.height)
        });
        let (x, y) = self.cursor;
        if !contents.is_empty() {
            let Some((gx, mut gy)) = self.draw_tooltip_box(batch, &lines, x, y, (width, height))
            else {
                return true;
            };
            for (mod_id, c) in &contents {
                let moved: Vec<ScreenElement> = c
                    .elements
                    .iter()
                    .filter_map(|e| translated(e, gx, gy))
                    .collect();
                self.draw_mod_elements_of(batch, Some(mod_id), &moved);
                gy += c.height;
            }
        } else {
            self.draw_tooltip_lines(batch, &lines, x, y);
        }
        self.mods
            .tooltip_hovered
            .set(Some((item, self.mods.tooltip_frame.get())));
        true
    }
}
/// A stack for `tooltip.get-hovered-stack`: its counts and the items of its
/// container component (API 0.1.28).
fn hovered_stack(stack: &horizon_client::items::ItemStack) -> horizon_mods::TooltipHoveredStack {
    use horizon_mods::TooltipContainerItem;
    let mut container = Vec::new();
    if let Some(data) = stack.data().filter(|d| !d.container.is_empty()) {
        let slots = data
            .container
            .iter()
            .map(|(slot, _)| usize::from(*slot) + 1)
            .max()
            .unwrap_or(0)
            .max(27);
        container.resize(slots, None);
        for (slot, item) in &data.container {
            container[usize::from(*slot)] = Some(TooltipContainerItem {
                id: item.item.to_string(),
                count: item.count,
                damage: item.damage,
            });
        }
    }
    horizon_mods::TooltipHoveredStack {
        item: stack.item.to_string(),
        count: stack.count,
        damage: stack.damage,
        container,
    }
}
/// An element moved by (dx, dy); none for the kinds that have their own place.
fn translated(e: &ScreenElement, dx: f32, dy: f32) -> Option<ScreenElement> {
    let mut e = e.clone();
    match &mut e {
        ScreenElement::Fill(f) => (f.x += dx, f.y += dy),
        ScreenElement::Label(l) => (l.x += dx, l.y += dy),
        ScreenElement::Sprite(s) => (s.x += dx, s.y += dy),
        ScreenElement::Picture(p) => (p.x += dx, p.y += dy),
        ScreenElement::Item(i) => (i.x += dx, i.y += dy),
        ScreenElement::Tooltip(_) | ScreenElement::Tip(_) => return None,
    };
    Some(e)
}

/// Mods with the world on an integrated server's thread.
impl Game<'_> {
    /// The world's seed: the server thread's, or this game's own.
    fn world_seed(&self) -> u64 {
        match self.integrated.as_ref() {
            Some(server) => server.seed as u64,
            None => self.level.seed as u64,
        }
    }
    /// On the server thread: what mods asked of the world, carried out as
    /// the integrated server's player, in the order the mods gave it.
    pub(super) fn apply_mod_server_ops(&mut self, ops: Vec<Output>) {
        let Some(owner) = self.serving.as_ref().map(|s| s.owner) else {
            return;
        };
        let conn = self
            .lan
            .as_ref()
            .and_then(|host| host.hub.guests.iter().find(|g| g.uuid == owner))
            .map(|g| g.conn);
        let Some(conn) = conn else {
            return;
        };
        self.for_player(super::commands::Target::Guest(conn), |game| {
            for op in ops {
                match op {
                    Output::Break {
                        pos,
                        drops,
                        with_held,
                    } => game.mod_break(pos, drops, with_held),
                    Output::FallBlock {
                        pos,
                        velocity,
                        drops,
                        place,
                    } => game.mod_fall_block(pos, velocity, drops, place),
                    Output::SetBlock {
                        pos,
                        id,
                        properties,
                    } => game.mod_set_block(pos, &id, &properties),
                    Output::SpawnItem { pos, id, count } => game.mod_spawn_item(pos, &id, count),
                    Output::MergeDrops { pos } => game.mod_merge_drops(pos),
                    Output::Exhaust { amount } => {
                        if let Some(p) = game.player.as_mut() {
                            p.cause_food_exhaustion(amount);
                        }
                    }
                    Output::RemoveEntity { id } => game.mod_remove_entity(id),
                    Output::SetXp { id, xp } => {
                        if let Some(orb) = game.orbs.iter_mut().find(|o| o.id == id) {
                            orb.value = xp;
                        }
                    }
                    Output::Give { id, count } => game.mod_give(&id, count),
                    Output::MoveItem { from, to } => game.mod_move_item(from as usize, to as usize),
                    Output::DamageHeld { amount } => {
                        if game.player.is_some() {
                            game.damage_held(amount);
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}

/// Longer rays, mob effects, fluids and boss bars (API 0.1.34).
impl Game<'_> {
    /// The farthest a `get-*-ex` ray looks.
    const MOD_RAY: f64 = 32.0;
    /// What a ray hits: the first block and the nearest mob, with distances.
    fn mod_ray(&self, origin: [f64; 3], dir: [f64; 3]) -> horizon_mods::RayHits {
        let dist = |p: [f64; 3]| {
            (0..3)
                .map(|a| (p[a] - origin[a]).powi(2))
                .sum::<f64>()
                .sqrt() as f32
        };
        let block = self.clip_ray(origin, dir, Self::MOD_RAY).map(|hit| {
            let face = match hit.face {
                [0, -1, 0] => "down",
                [0, 1, 0] => "up",
                [0, 0, -1] => "north",
                [0, 0, 1] => "south",
                [-1, 0, 0] => "west",
                _ => "east",
            };
            (self.mod_block(hit.pos), face.to_owned(), dist(hit.point))
        });
        let mut best: Option<(usize, f64)> = None;
        for (i, m) in self.mobs.iter().enumerate() {
            if m.alive()
                && let Some(t) = super::ray_box(origin, dir, Self::MOD_RAY, m.bounds())
                && best.is_none_or(|b| t < b.1)
            {
                best = Some((i, t));
            }
        }
        horizon_mods::RayHits {
            block,
            entity: best.map(|(i, t)| (self.mod_entity_of(i), t as f32)),
        }
    }
    /// The target's extras for the snapshot: effects of the mob looked at,
    /// the long rays from the eyes and the camera, and the fluid in front.
    fn mod_target_extras(&mut self, snapshot: &mut Snapshot) {
        if let Some(e) = &snapshot.entity
            && let Some(m) = self.mobs.iter().find(|m| m.id == e.id)
        {
            snapshot.entity_effects = Some(
                m.extra
                    .effects
                    .iter()
                    .map(|&(id, ticks, amplifier)| horizon_mods::TargetEffect {
                        id: id.to_owned(),
                        amplifier: amplifier.max(0) as u32,
                        duration_ticks: if ticks > 1_000_000 { -1 } else { ticks },
                        ambient: false,
                        visible: true,
                    })
                    .collect(),
            );
        }
        let Some(eye) = self.eye_position() else {
            return;
        };
        let dir = super::view_vector(self.yaw, self.pitch);
        snapshot.ray_eye = Some(self.mod_ray(eye, dir));
        snapshot.ray_camera = match self.camera_view(1.0) {
            Some((c, yaw, pitch)) => {
                Some(self.mod_ray([c.x, c.y, c.z], super::view_vector(yaw, pitch)))
            }
            None => snapshot.ray_eye.clone(),
        };
        // The first fluid before the block in reach.
        let stop = snapshot
            .block
            .as_ref()
            .and_then(|_| self.pick())
            .map_or(self.reach(), |h| {
                (0..3)
                    .map(|a| (h.point[a] - eye[a]).powi(2))
                    .sum::<f64>()
                    .sqrt()
            });
        let facts = self.res.facts();
        let mut t = 0.0;
        while t <= stop.min(self.reach()) {
            let at = [0, 1, 2].map(|a| (eye[a] + dir[a] * t).floor() as i32);
            let state = self.world.block(at[0], at[1], at[2]);
            let f = facts.get(state);
            if f.fluid != horizon_client::blocks::Fluid::None {
                let id = match f.fluid {
                    horizon_client::blocks::Fluid::Lava => "minecraft:lava",
                    _ => "minecraft:water",
                };
                let level = self
                    .pack
                    .assets
                    .registry
                    .state(state)
                    .and_then(|s| s.properties.get("level"))
                    .and_then(|l| l.parse().ok())
                    .unwrap_or(0);
                let name =
                    horizon_client::lang::get_or(&format!("block.{}", id.replace(':', ".")), id);
                snapshot.fluid_ex = Some(horizon_mods::FluidEx {
                    id: id.to_owned(),
                    name: name.clone(),
                    level,
                    x: at[0],
                    y: at[1],
                    z: at[2],
                    sprite: format!("{id}_still").replacen("minecraft:", "minecraft:block/", 1),
                });
                snapshot.fluid = Some(horizon_mods::TargetFluid {
                    id: id.to_owned(),
                    name,
                    level,
                });
                break;
            }
            t += 0.1;
        }
    }
    /// The boss bars the HUD draws, with where each ends.
    fn mod_boss_bars(&self) -> Vec<horizon_mods::BossBarInfo> {
        let Some(r) = self.renderer.as_ref() else {
            return Vec::new();
        };
        let h = HudBatch::new([r.config.width, r.config.height]).height as i32;
        let bars = self
            .boss_overlay
            .bars(horizon_client::boss_overlay::now_millis());
        let mut out = Vec::new();
        let mut y = 12;
        for bar in bars {
            out.push(horizon_mods::BossBarInfo {
                name: bar.name.plain(),
                y_bottom: (y + 5) as f32,
            });
            y += 19;
            if y >= h / 3 {
                break;
            }
        }
        out
    }
}

#[cfg(feature = "mods")]
const MOD_UI_TOP: f32 = 40.0;
#[cfg(feature = "mods")]
const MOD_UI_COLUMN: f32 = 200.0;
/// A key row's height with its gap, as the widgets below it space out.
#[cfg(feature = "mods")]
const MOD_UI_KEY_ROW: f32 = 26.0;
/// Entity extras, block entities and properties, item tooltips (API 0.1.35).
impl Game<'_> {
    /// `entity.get-extra`, `get-loot-table`, `get-width` and `is-frozen` for
    /// the entities `nearby` lists; the extras of the 64 nearest only.
    fn mod_entity_extras(&self, entities: &[horizon_mods::WorldEntity], snapshot: &mut Snapshot) {
        use horizon_mods::EntityExtra;
        let uuid = |o: [i32; 4]| {
            format!(
                "{:032x}",
                o.iter()
                    .fold(0u128, |a, &w| (a << 32) | u128::from(w as u32))
            )
        };
        let items = self.res.items();
        let (mut extras, mut loot, mut widths, mut frozen) = (
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            std::collections::HashSet::new(),
        );
        for (rank, e) in entities.iter().enumerate() {
            let near = rank < 64;
            let mut extra = EntityExtra {
                baby: false,
                age: 0,
                in_love: false,
                profession: String::new(),
                profession_level: 0,
                horse_jump: 0.0,
                horse_speed: 0.0,
                pet_armor: None,
                owner: None,
                equipment: Vec::new(),
                frame_item: None,
                frame_rotation: 0,
                painting_motive: None,
                painting_width: 0,
                painting_height: 0,
                falling_block: None,
                item_count: 0,
            };
            if e.id < 0x8000_0000
                && let Some(m) = self.mobs.iter().find(|m| m.id == e.id)
            {
                widths.insert(e.id, m.dims().0 as f32);
                if m.ticks_frozen >= 140 {
                    frozen.insert(e.id);
                }
                if let Some(path) = m.kind.id.strip_prefix("minecraft:") {
                    let table = format!("minecraft:entities/{path}");
                    if items.has_loot_table(&table) {
                        loot.insert(e.id, table);
                    }
                }
                extra.baby = m.is_baby();
                extra.age = m.extra.age;
                extra.in_love = m.extra.in_love > 0;
                let villager = matches!(
                    m.species(),
                    horizon_client::mobs::Species::Villager
                        | horizon_client::mobs::Species::ZombieVillager
                );
                if villager {
                    let list = horizon_client::mobs::render::PROFESSIONS;
                    extra.profession =
                        format!("minecraft:{}", list[m.extra.variant2 as usize % list.len()]);
                    extra.profession_level = u32::from(m.extra.variant3).max(1);
                }
                if horizon_client::mobs::equine::is_equine(m.species()) {
                    extra.horse_jump = m.extra.vector[0] as f32;
                    extra.horse_speed = m.attrs.speed as f32;
                }
                if (horizon_client::mobs::equine::is_equine(m.species())
                    || m.species() == horizon_client::mobs::Species::Wolf)
                    && let Some(armor) = m.extra.chest.as_ref()
                {
                    extra.pet_armor = Some(armor.to_string());
                }
                extra.owner = m.extra.owner.map(uuid);
                extra.equipment = [
                    &m.extra.head,
                    &m.extra.chest,
                    &m.extra.legs,
                    &m.extra.feet,
                    &m.extra.main_hand,
                    &m.extra.off_hand,
                ]
                .iter()
                .map(|s| s.as_ref().map(|s| s.to_string()))
                .collect();
            } else if let Some(i) = self.dropped.iter().find(|i| i.id == e.id) {
                extra.item_count = i.stack.count;
                widths.insert(e.id, 0.25);
            } else if let Some(f) = self.item_frames.iter().find(|f| f.id == e.id) {
                extra.frame_item = f.item.as_ref().map(|s| s.item.to_string());
                extra.frame_rotation = u32::from(f.rotation);
                widths.insert(e.id, 0.5);
            } else if let Some(p) = self.paintings.iter().find(|p| p.id == e.id) {
                extra.painting_motive = Some(p.variant.to_string());
                extra.painting_width = p.width.max(0) as u32 / 16;
                extra.painting_height = p.height.max(0) as u32 / 16;
            } else if let Some(f) = self.falling.iter().find(|f| f.id == e.id && f.id != 0) {
                extra.falling_block = Some(self.res.facts().get(f.state).name.to_string());
                widths.insert(e.id, 0.98);
            }
            if near {
                extras.insert(e.id, extra);
            }
        }
        snapshot.entity_extras = Some(Arc::new(extras));
        snapshot.entity_loot = Some(Arc::new(loot));
        snapshot.entity_widths = Some(Arc::new(widths));
        snapshot.frozen = Some(Arc::new(frozen));
    }
    /// `target.get-block-properties`: `BlockBehaviour.Properties` of a block.
    fn mod_block_properties(&self, block: &Block) -> horizon_mods::BlockProperties {
        let state = self.world.block(block.x, block.y, block.z);
        let facts = self.res.facts().get(state);
        let id = block.id.strip_prefix("minecraft:").unwrap_or(&block.id);
        let (jump, speed) = match id {
            "honey_block" => (0.5, 0.4),
            "soul_sand" => (1.0, 0.4),
            _ => (1.0, 1.0),
        };
        let tagged = |tag: &str| {
            self.res
                .items()
                .block_tags
                .get(tag)
                .is_some_and(|members| members.contains(block.id.as_str()))
        };
        // FireBlock.bootStrap: (ignite odds, burn odds).
        let (ignite, burn) = if tagged("minecraft:planks")
            || tagged("minecraft:wooden_slabs")
            || tagged("minecraft:wooden_stairs")
            || tagged("minecraft:wooden_fences")
            || tagged("minecraft:fence_gates")
        {
            (5, 20)
        } else if tagged("minecraft:logs") || id.ends_with("_wood") || id.ends_with("_hyphae") {
            (5, 5)
        } else if tagged("minecraft:leaves")
            || tagged("minecraft:wool")
            || tagged("minecraft:wool_carpets")
        {
            (30, 60)
        } else {
            match id {
                "bookshelf" => (30, 20),
                "tnt" => (15, 100),
                "hay_block" | "dried_kelp_block" => (60, 20),
                "coal_block" => (5, 5),
                "short_grass" | "fern" | "dead_bush" | "tall_grass" | "large_fern" => (60, 100),
                "vine" => (15, 100),
                "bamboo" => (60, 60),
                "sweet_berry_bush" => (60, 100),
                "lectern" | "composter" | "beehive" | "bee_nest" => (30, 20),
                _ => (0, 0),
            }
        };
        horizon_mods::BlockProperties {
            explosion_resistance: facts.explosion_resistance,
            jump_factor: jump,
            speed_factor: speed,
            ignite_odds: ignite,
            burn_odds: burn,
        }
    }
    /// `world.get-block-entity` for the block at `pos`.
    fn mod_block_entity(&self, pos: [i32; 3]) -> Option<horizon_mods::WorldBlockEntity> {
        use horizon_client::containers::BlockEntity;
        let block_id = self
            .res
            .facts()
            .get(self.world.block(pos[0], pos[1], pos[2]))
            .name
            .to_string();
        let entity = self.world.block_entities.get(&(pos[0], pos[1], pos[2]))?;
        let mut out = horizon_mods::WorldBlockEntity {
            id: block_id,
            items: self.mod_container(pos).unwrap_or_default(),
            bees: 0,
            jukebox_record: None,
            book: None,
            spawner_entity: None,
            spawner_cooldown: 0,
            command: None,
            sign_lines: Vec::new(),
            waxed: false,
        };
        match entity {
            BlockEntity::Beehive(h) => out.bees = h.bees.len() as u32,
            BlockEntity::Jukebox(j) => {
                out.jukebox_record = j.record.as_ref().map(|s| s.item.to_string());
            }
            BlockEntity::Lectern(l) => out.book = l.book.as_ref().map(|s| s.item.to_string()),
            BlockEntity::Spawner(s) => {
                out.spawner_cooldown = s.spawn_delay.max(0) as u32;
                out.spawner_entity = s.potentials.first().and_then(|(data, _)| data.id());
            }
            BlockEntity::CommandBlock(c) => out.command = Some(c.command.clone()),
            BlockEntity::Sign(s) => {
                out.sign_lines = s.front.lines.to_vec();
                out.waxed = s.waxed;
            }
            _ => {}
        }
        Some(out)
    }
    /// The tooltip of each item as a plain stack, built once.
    fn mod_item_tooltips(&self) -> Arc<HashMap<String, Vec<horizon_mods::ItemTooltipLine>>> {
        self.mods
            .item_tooltips
            .get_or_init(|| {
                let items = self.res.items();
                Arc::new(
                    items
                        .order
                        .iter()
                        .map(|id| {
                            let lines = self
                                .stack_tooltip(&horizon_client::items::ItemStack::new(id, 1))
                                .into_iter()
                                .map(|(text, color)| horizon_mods::ItemTooltipLine {
                                    text: text.plain(),
                                    color,
                                })
                                .collect();
                            (id.to_string(), lines)
                        })
                        .collect(),
                )
            })
            .clone()
    }
}
