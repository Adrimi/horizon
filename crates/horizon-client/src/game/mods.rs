//! Horizon mods in the game (`docs/mods.md`): loading the ones turned on,
//! their tick, frame, chat and command hooks, their HUD panels and what
//! they print. With no mod on, `horizon_mods::Mods` holds no engine and
//! every hook here returns after one check.
use super::mod_scan::Palette;
use super::{Game, Screen};
use horizon_client::{
    command::{Dispatcher, vanilla},
    extras::Corner,
    extras_hud::{self, Corners},
    font::Patch,
    hud::{HudBatch, MOD_TEXTURE_SIDE, MOD_TEXTURE_SLOTS},
    mod_list::{self, Installed},
    text::{self, Text},
};
use horizon_mods::{
    Block, Capability, Clock, Food, FoodStats, GameScreen, Item, ItemTags, KeyEvent, Modifiers,
    Mods, Output, Player, Point, Recipe, RecipeKind, ScreenElement, ScreenEvent, ScreenShape,
    Snapshot, Stack, Status, Ui,
};
use horizon_mods::{KeyPress, MouseMove, MousePress, WheelTurn};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct ModsState {
    pub(super) mods: Mods,
    /// `--dev-mod PATH`: always on, reloaded when rebuilt.
    pub(super) dev: Vec<PathBuf>,
    /// When dev mods were last checked for a rebuild.
    checked: Option<Instant>,
    /// The chat's command tree while a running mod owns commands.
    tree: Option<Dispatcher>,
    /// Blocks broken since the last tick, for `world.broken-blocks`
    /// (`mod_world.rs`).
    pub(super) broken: Vec<horizon_mods::BlockBroken>,
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
    /// The screen a mod has open (API 0.1.4).
    pub(super) screen: ModScreen,
    /// Every item of the game, for `items.list` (API 0.1.5); built on first use.
    items: std::sync::OnceLock<Arc<Vec<Item>>>,
    /// Every recipe of the game, for `recipes.list` (API 0.1.5); built on first use.
    recipes: std::sync::OnceLock<Arc<Vec<Recipe>>>,
    /// The width of the first characters of the font, for `screen.get-text-width`.
    text_widths: std::sync::OnceLock<Arc<Vec<f32>>>,
    /// The mod whose panel took a mouse press, so it gets the release too.
    overlay_down: Option<String>,
    /// Particle pictures waiting to go into the atlas' particle sheet, and
    /// the names the mods' numbers stand for (API 0.1.6).
    particle_patches: Vec<Patch>,
    particle_names: HashMap<u32, String>,
    /// The bars the game drew in the last frame, and the camera it drew with
    /// (`hud.get-bars`, `overlay`; API 0.1.7).
    pub(super) bars: Option<horizon_mods::Bars>,
    projection: Option<Projection>,
    /// The atlas slot of each mod's overlay image, by (mod, image).
    overlay_slots: HashMap<(String, u32), usize>,
    /// The version uploaded to each overlay slot.
    overlay_versions: HashMap<usize, u64>,
}
impl ModsState {
    /// The chat's command tree: vanilla's, plus the running mods' commands.
    pub(super) fn dispatcher(&self) -> &Dispatcher {
        self.tree
            .as_ref()
            .unwrap_or_else(|| vanilla::game_dispatcher())
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
const RELOAD_CHECK: Duration = Duration::from_secs(1);

/// A vanilla command name: mods may not take it.
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
            out
        }
        Err(e) => {
            log::warn!("Horizon: content mods not loaded: {e}");
            root.to_owned()
        }
    }
}

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
        let mods = Mods::load(&paths, on, vanilla_command);
        self.mods.mods = mods;
        // The mods' own files, in a folder of their own beside the
        // settings (API 0.1.4).
        self.mods.mods.set_storage(Some(
            Self::options_path().with_file_name(mod_list::DATA_DIR),
        ));
        self.sync_mod_keys();
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
    /// What mods read this tick.
    fn mod_snapshot(&self) -> Snapshot {
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
        let mut snapshot = Snapshot {
            weather: self.mod_weather(),
            particles_setting: u32::from(self.settings.particles),
            player,
            clock,
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
        }
        if mods.uses(Capability::RecipesRead) {
            snapshot.recipes = self.mod_recipes();
        }
        if mods.uses(Capability::EntityRead) && self.player.is_some() {
            snapshot.entities = Some(Arc::new(self.mod_entities()));
            snapshot.entities_editable = self.remote.is_none();
        }
        if snapshot.player.is_none() {
            return snapshot;
        }
        if !target && !inventory {
            return snapshot;
        }
        let looked = self.pick().map(|hit| hit.pos);
        let breaking = self.mining.as_ref().map(|m| m.pos);
        if target {
            snapshot.block = looked.map(|pos| self.mod_block(pos));
            snapshot.breaking = breaking.map(|pos| self.mod_block(pos));
            snapshot.breaking_progress = self.mining.as_ref().map(|m| m.progress.min(1.0));
            snapshot.block_name = snapshot.block.as_ref().map(|b| {
                horizon_client::lang::get_or(&format!("block.{}", b.id.replace(':', ".")), &b.id)
            });
            snapshot.entity = self.mod_entity();
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
        let m = &self.mobs[i];
        let name = m.custom_name.clone().unwrap_or_else(|| {
            let key = format!("entity.{}", m.kind.id.replace(':', "."));
            horizon_client::lang::get_or(&key, &m.kind.id)
        });
        Some(horizon_mods::Entity {
            id: m.id,
            type_id: m.kind.id.clone(),
            name,
            health: m.health,
            max_health: m.attrs.max_health,
        })
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
        if !self.mods.mods.is_running() {
            return;
        }
        let mut snapshot = self.mod_snapshot();
        snapshot.broken = std::mem::take(&mut self.mods.broken);
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
        self.show_mod_output();
        self.apply_mod_particles();
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
                self.list_mods();
                self.sync_mod_commands();
                self.system_message(Text::literal("Mods reloaded").with_color(text::GRAY));
            }
        }
        if !self.mods.mods.is_running() {
            return;
        }
        self.sync_mod_ui();
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
                x: slot as u32 * MOD_TEXTURE_SIDE,
                y: atlas.height - MOD_TEXTURE_SIDE,
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
                        .get(&format!("horizon/mod-texture{slot}"))
                        .copied()
                    else {
                        continue;
                    };
                    // The image may be smaller than the slot: draw only
                    // the part that was uploaded.
                    let texture = screen.textures.iter().find(|t| t.id == i.texture);
                    let (w, h) = texture.map_or((1, 1), |t| (t.width, t.height));
                    let rect = horizon_client::hud::Rect {
                        u1: rect.u0 + (rect.u1 - rect.u0) * (w as f32 / MOD_TEXTURE_SIDE as f32),
                        v1: rect.v0 + (rect.v1 - rect.v0) * (h as f32 / MOD_TEXTURE_SIDE as f32),
                        w,
                        h,
                        ..rect
                    };
                    batch.rect_uv(i.x, i.y, i.width, i.height, rect, u32::MAX);
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
        let snapshot = self.mod_snapshot();
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
    /// A chat line the player received.
    pub(super) fn mods_chat(&mut self, line: &str) {
        self.mods.mods.chat(line);
        self.show_mod_output();
    }
    /// Chat lines, command feedback and stopped mods, into the chat.
    fn show_mod_output(&mut self) {
        for output in self.mods.mods.drain() {
            let line = match output {
                Output::Chat { name, text } => {
                    let mut line = Text::literal(format!("[{name}] ")).with_color(text::GRAY);
                    line.push(Text::literal(text));
                    line
                }
                Output::Feedback { text, error: false } => Text::literal(text),
                Output::Feedback { text, error: true } => Text::literal(text).with_color(text::RED),
                Output::Stopped { name, reason } => {
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
            };
            // Not through system_message: a mod must not hear its own lines.
            let focused = self.screen == Screen::Chat;
            self.ui
                .chat
                .add(&self.gui.atlas, line, None, self.ticks, focused);
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
    }
}

/// 0xAARRGGBB, as mods give colours, in the HUD batch's byte order.
fn argb(c: u32) -> u32 {
    let [a, r, g, b] = c.to_be_bytes();
    horizon_client::hud::rgba(r, g, b, a)
}
fn tip_lines(lines: &[horizon_mods::TipLine]) -> Vec<(String, u32)> {
    lines
        .iter()
        .map(|l| (l.text.clone(), argb(l.color)))
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
            can_give: self.creative && self.remote.is_none() && self.player.is_some(),
            hud: self.player.is_some().then_some(horizon_mods::HudState {
                screen_open: self.screen != Screen::Playing,
                debug: self.debug,
                hidden: self.ui.hide_gui,
            }),
            bars: self.mods.bars.clone(),
            seed: (self.remote.is_none() && self.player.is_some())
                .then_some(self.level.seed as u64),
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
        let mut tips: Vec<(Vec<(String, u32)>, f32, f32)> = Vec::new();
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
                    batch.rect_uv(p.x, p.y, p.width, p.height, part, argb(p.tint));
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
            self.mods.particle_patches.push(Patch {
                x: slot_x + cx,
                y: atlas_height - MOD_TEXTURE_SIDE + cy,
                width: def.width,
                height: def.height,
                rgba: def.pixels,
            });
            self.mods.particle_names.insert(def.id, def.name);
        }
        for p in particles {
            let Some(name) = self.mods.particle_names.get(&p.sprite) else {
                continue;
            };
            use horizon_client::particles::{ModFade, ModParticle, ModSize};
            self.particles.add_mod(&ModParticle {
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
            });
        }
    }
    /// The particle pictures that wait to go into the atlas.
    pub(super) fn flush_mod_particle_sprites(&mut self, batch: &mut HudBatch) {
        batch.patches.append(&mut self.mods.particle_patches);
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
        fog.color = horizon_client::rain::mix_fog(fog.color, [h.red, h.green, h.blue], h.amount);
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
        // Upload the images the mods changed.
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
    /// What the mods drew at points of the world (API 0.1.7).
    pub(super) fn draw_mod_world(&mut self, batch: &mut HudBatch) {
        if !self.mods.mods.is_running() || !self.mods.mods.uses(Capability::Overlay) {
            return;
        }
        let worlds: Vec<horizon_mods::WorldElement> = self
            .mods
            .mods
            .overlay_world()
            .into_iter()
            .flat_map(|(_, e)| e.to_vec())
            .collect();
        for e in worlds {
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
            found.push((
                dist2(m.position),
                WorldEntity {
                    id: m.id,
                    type_id: m.kind.id.clone(),
                    kind: EntityKind::Mob,
                    name,
                    uuid: String::new(),
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
