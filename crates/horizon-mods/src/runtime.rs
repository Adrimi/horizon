//! Loading and calling mods with wasmtime's component model.
use crate::manifest::{Manifest, Package, Side};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use wasmtime::{
    Config, Engine, Store, StoreLimits, StoreLimitsBuilder, Trap,
    component::{Component, HasSelf, Linker},
};

mod bindings {
    wasmtime::component::bindgen!({
        path: "../horizon-mod-sdk/wit",
        world: "horizon-mod",
    });
}
pub use bindings::horizon::mods::{
    game::{Clock, FoodStats, Player},
    hud::{Corner, Line},
    inventory::{Food, Stack},
    items::Item,
    keys::KeyEvent,
    recipes::{Kind as RecipeKind, Recipe},
    screen::{
        Area, Element as ScreenElement, Event as ScreenEvent, GameScreen, ImageShape, KeyPress,
        Modifiers, MouseMove, MousePress, Point, RectShape, Shape as ScreenShape,
        Size as ScreenSize, TextShape, TipLine, WheelTurn,
    },
    target::{Block, Entity},
    world::{BlockBroken, BlockPos, ChunkSurface, WorldBlock},
};
use bindings::{
    HorizonMod,
    exports::horizon::mods::events::Registration,
    horizon::mods::{
        chat, game, hotbar, hud, inventory, items, keys, log as mod_log, recipes, screen, storage,
        target, world,
    },
};

/// A mod's linear memory cap.
const MEMORY: usize = 64 << 20;
/// Fuel (about one unit per WebAssembly instruction) per call: a bound on
/// runaway loops, far above the hooks' time budgets.
const FUEL_INIT: u64 = 2_000_000_000;
const FUEL_TICK: u64 = 50_000_000;
const FUEL_FRAME: u64 = 10_000_000;
const FUEL_EVENT: u64 = 100_000_000;
/// Chat lines a mod may show per call, HUD lines and their length.
const CHAT_PER_CALL: usize = 8;
const HUD_LINES: usize = 16;
const LINE_CHARS: usize = 256;
/// Total bytes a mod's `storage` may hold on disk, across all its keys.
const STORAGE_QUOTA: u64 = 4 << 20;
/// Distinct keys a mod's `storage` may hold: `STORAGE_QUOTA` alone doesn't
/// bound a flood of zero- or near-zero-byte files (each still a file name,
/// a directory entry and an inode).
const STORAGE_MAX_KEYS: usize = 64;
/// Bytes `storage::set` may write across one hook call: fuel bounds a
/// mod's CPU time per call, not the time a synchronous disk write takes, so
/// a cheap-in-fuel loop replacing one key with large payloads needs its own
/// cap, separate from the on-disk quota it would otherwise stay under.
const STORAGE_WRITE_BUDGET: u64 = 1 << 20;
/// A mod's data, beside its `horizon-mods` folder (`find`'s `path.parent()`
/// never has a manifest there, so `find` never mistakes it for a mod).
const STORAGE_DIR: &str = ".horizon-mod-data";
/// A mod's screen: how many shapes it may draw, how many images it may
/// upload and how big they are.
pub const SCREEN_SHAPES: usize = 4096;
/// How many elements `paint` takes per call, and how many keys a panel names.
pub const SCREEN_ELEMENTS: usize = 4096;
const PANEL_KEYS: usize = 64;
/// How many tooltip lines an element carries.
const TIP_LINES: usize = 32;
/// The characters whose width `text-width` knows (the Latin ones); the others
/// count as 8.
pub const TEXT_WIDTHS: usize = 512;
pub const SCREEN_TEXTURES: usize = 8;
pub const SCREEN_TEXTURE_SIDE: u32 = 256;

/// What mods may read of the game, copied once per tick by the game.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub player: Option<Player>,
    pub clock: Option<Clock>,
    /// The block under the crosshair (`target`).
    pub block: Option<Block>,
    /// The block being broken (`target`).
    pub breaking: Option<Block>,
    /// How far it is broken, 0 to 1 (`target`, API 0.1.2).
    pub breaking_progress: Option<f32>,
    /// The looked-at block's name in the game's language (`target`, API 0.1.2).
    pub block_name: Option<String>,
    /// The mob under the crosshair (`target`, API 0.1.2).
    pub entity: Option<Entity>,
    /// Hotbar slots 0 to 8 (`inventory-read`); empty outside a world.
    pub hotbar: Vec<Option<Stack>>,
    pub selected: u32,
    /// Hunger, saturation and exhaustion (`game-read`, API 0.1.2).
    pub food: Option<FoodStats>,
    /// What eating each hotbar slot gives (`inventory-read`, API 0.1.2);
    /// empty outside a world.
    pub hotbar_food: Vec<Option<Food>>,
    /// Blocks broken since the previous tick (`world-read`, API 0.1.2).
    pub broken: Vec<BlockBroken>,
    /// Every item tag by item, shared rather than copied each tick
    /// (`inventory-read`, API 0.1.3); none unless a mod uses it.
    pub item_tags: Option<Arc<ItemTags>>,
    /// Every item of the game (`items-read`, API 0.1.5); built once by the
    /// game and shared.
    pub items: Option<Arc<Vec<Item>>>,
    /// Every recipe of the game (`recipes-read`, API 0.1.5); built once.
    pub recipes: Option<Arc<Vec<Recipe>>>,
}

/// What mods read of the game's screens (API 0.1.5), copied once per frame by
/// the game: the open screen, where the mouse is, the keys held and the
/// slots on screen.
#[derive(Clone, Debug)]
pub struct Ui {
    pub screen: GameScreen,
    pub menu_origin: Option<Point>,
    pub cursor: Point,
    pub modifiers: Modifiers,
    /// The slots of the open screen with an item in them: their rectangles
    /// `[x, y, width, height]` and the item ids.
    pub slots: Vec<([f32; 4], String)>,
    /// The width of the characters 0 to `TEXT_WIDTHS` at the game's font,
    /// for `text-width`.
    pub text_widths: Option<Arc<Vec<f32>>>,
    /// The player is in creative in a world this game runs: `give` works.
    pub can_give: bool,
}
impl Default for Ui {
    fn default() -> Self {
        Ui {
            screen: GameScreen::None,
            menu_origin: None,
            cursor: Point { x: 0.0, y: 0.0 },
            modifiers: Modifiers {
                shift: false,
                ctrl: false,
                alt: false,
            },
            slots: Vec::new(),
            text_widths: None,
            can_give: false,
        }
    }
}

/// Every item tag by item (API 0.1.3, `inventory-read`), built once from the
/// game's tags, nested ones already expanded as vanilla's `TagLoader` does.
/// [`Snapshot`] holds an `Arc` of it, so ticking and each call only clone
/// the reference.
#[derive(Clone, Debug, Default)]
pub struct ItemTags {
    /// Item id -> the tags it is in, sorted by name.
    by_item: HashMap<String, Vec<String>>,
}

impl ItemTags {
    /// `tags`: each tag's full name (no `#`) and its item ids, as the game
    /// resolves them.
    pub fn new(tags: impl IntoIterator<Item = (String, Vec<String>)>) -> ItemTags {
        let mut all: Vec<(String, Vec<String>)> = tags.into_iter().collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let mut by_item: HashMap<String, Vec<String>> = HashMap::new();
        for (tag, items) in all {
            for item in items {
                by_item.entry(item).or_default().push(tag.clone());
            }
        }
        ItemTags { by_item }
    }

    /// Every tag of `id`, sorted; empty for an unknown item.
    pub fn of(&self, id: &str) -> Vec<String> {
        self.by_item.get(id).cloned().unwrap_or_default()
    }
}

/// The live world `world` reads, lent by the game for the length of one
/// hook call ([`Mods::set_world`]) so blocks are read where they are, not
/// copied every tick.
pub trait WorldAccess: Send + Sync {
    /// The block at `pos`; none where no chunk is loaded.
    fn block(&self, pos: &BlockPos) -> Option<WorldBlock>;
    /// Whether `world-edit` may break the block at `pos`: loaded, not air,
    /// not unbreakable (bedrock), and the world is this game's own.
    fn breakable(&self, pos: &BlockPos) -> bool;
    /// The main hand's wear, `(damage, max_damage)`, when it holds an item
    /// that wears, the player is not in creative and the world is this
    /// game's own.
    fn held_wear(&self) -> Option<(u32, u32)>;
    /// The surface of the chunk (cx, cz), as a map sees it; none when the
    /// chunk is not loaded (API 0.1.4).
    fn chunk_surface(&self, cx: i32, cz: i32) -> Option<ChunkSurface>;
    /// The world's biome names, in the order of their indexes (API 0.1.4).
    fn biome_names(&self) -> Vec<String>;
    /// The map's colours by id, as 0xRRGGBB (API 0.1.4).
    fn map_colors(&self) -> Vec<u32>;
}

/// One of a mod's key bindings (`keys`): the action it named, the key it
/// asked for and the key in use now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBind {
    /// The action the mod named (`map`, `waypoint`, ...).
    pub action: String,
    /// The key the mod asked for (`key.keyboard.m`).
    pub default_key: String,
    /// The key in use now: the player's, or the default.
    pub key: String,
}

/// An image a mod uploaded for its screen.
#[derive(Clone, Debug)]
pub struct Texture {
    /// The id the mod draws it by.
    pub id: u32,
    pub width: u32,
    pub height: u32,
    /// The RGBA pixels, `width * height * 4` bytes.
    pub pixels: Vec<u8>,
    /// Bumped when the image changes, so the game uploads it once.
    pub version: u64,
}

/// A mod's screen (`screen`): what it draws, the images it uploaded and what
/// the player did.
#[derive(Clone, Debug)]
pub struct Screen {
    /// The id the mod knows it by.
    pub id: u32,
    /// The title shown above the screen.
    pub title: String,
    /// False once the screen is closed; the mod can still read its events.
    pub open: bool,
    /// What to draw in the next frame.
    pub shapes: Vec<ScreenShape>,
    /// The images the mod uploaded.
    pub textures: Vec<Texture>,
    /// What the player did, since the last `get-input`.
    pub input: Vec<ScreenEvent>,
    /// What `paint` gave for the next frame (API 0.1.5).
    pub elements: Vec<ScreenElement>,
    /// Set for a panel over the game's own screen (API 0.1.5): where it is
    /// and the keys it takes.
    pub overlay: Option<Overlay>,
}

/// A panel over the game's own screen (`screen.open-panel`, API 0.1.5).
#[derive(Clone, Debug)]
pub struct Overlay {
    pub area: Area,
    /// The key names the panel takes instead of the game; `*` is all.
    pub keys: Vec<String>,
}
impl Overlay {
    /// The mouse at (x, y) is the panel's.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let a = &self.area;
        a.width > 0.0
            && a.height > 0.0
            && x >= a.x
            && y >= a.y
            && x < a.x + a.width
            && y < a.y + a.height
    }
    /// The panel takes this key (a name like `key.keyboard.r`).
    pub fn takes(&self, key: &str) -> bool {
        self.area.width > 0.0
            && self.area.height > 0.0
            && self
                .keys
                .iter()
                .any(|k| k == key || (k == "*" && key != "key.keyboard.escape"))
    }
}

/// What mods asked the game to show since the last [`Mods::drain`].
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// `chat.show`: a line after the mod's name.
    Chat { name: String, text: String },
    /// A mod command's feedback, red when `error`.
    Feedback { text: String, error: bool },
    /// The mod was turned off: it trapped, overran or failed to start.
    Stopped { name: String, reason: String },
    /// `hotbar.select`: the hotbar slot to select, 0 to 8.
    Select { slot: u32 },
    /// `world.break-block` and `break-block-with-held`: break the block at
    /// `pos`, dropping its loot when `drops`, rolled with the held item
    /// when `with_held`.
    Break {
        pos: [i32; 3],
        drops: bool,
        with_held: bool,
    },
    /// `world.damage-held`: wear the main hand's item by `amount` uses.
    DamageHeld { amount: u32 },
    /// `inventory.give` (API 0.1.5): put `count` of the item in the
    /// player's inventory, dropping what does not fit.
    Give { id: String, count: u32 },
}

/// A mod's text panel, drawn every frame until it changes.
#[derive(Clone, Debug)]
pub struct Panel {
    pub corner: Corner,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Installed and turned off by the player.
    Off,
    Running,
    /// Could not load or start, or stopped while running.
    Failed(String),
}

/// One installed mod, for the Mods page.
pub struct Entry {
    pub path: PathBuf,
    /// None when the manifest could not be read (`status` says why).
    pub manifest: Option<Manifest>,
    pub status: Status,
    /// The mod's average time per call of its busiest hook.
    pub cost: Duration,
    run: Option<Running>,
    modified: Option<SystemTime>,
}

struct Running {
    store: Store<State>,
    exports: HorizonMod,
    hooks: Registration,
}

struct State {
    name: String,
    id: String,
    snapshot: Arc<Snapshot>,
    /// The world lent for the current call (`world`).
    world: Option<Arc<dyn WorldAccess>>,
    /// The manifest declares `world-edit`.
    can_edit: bool,
    /// Breaks and wear asked for in the current call, so a second break of
    /// one block says false and wear adds up.
    breaking: Vec<[i32; 3]>,
    worn: u32,
    out: Vec<Output>,
    panel: Option<Panel>,
    /// Capability `storage`'s folder; made on first `set`.
    data_dir: PathBuf,
    /// Bytes `storage::set` may still write before this hook call ends
    /// (reset per call in [`Mods::call`]): fuel bounds a mod's CPU time but
    /// not disk I/O time, so a loop of small-fuel, large-payload `set`s
    /// needs its own budget.
    storage_budget: u64,
    /// What one hook call may write in all, and the mod's quota and key
    /// limit (the manifest may raise the last two).
    storage_write_budget: u64,
    storage_quota: u64,
    storage_max_keys: usize,
    limits: StoreLimits,
    /// The mod's screen (`screen`).
    screen: Option<Screen>,
    /// The mod's panel over the game's own screen (`screen`, API 0.1.5).
    overlay: Option<Screen>,
    /// What the game says of its screens (API 0.1.5).
    ui: Arc<Ui>,
    /// The manifest declares `inventory-edit`.
    can_give: bool,
    /// What the mod has asked to give in the current call.
    given: u32,
    /// The next screen id this mod gets.
    next_screen: u32,
    /// The mod's key bindings (`keys`).
    binds: Vec<KeyBind>,
    /// The key events since the last `poll` (`keys`).
    key_events: Vec<KeyEvent>,
    /// The screen's size in GUI pixels, given by the game.
    screen_size: ScreenSize,
}

/// The installed mods and the engine running them. `Mods::default()` is
/// the state with none: no engine, and every hook a no-op.
#[derive(Default)]
pub struct Mods {
    engine: Option<(Engine, Linker<State>)>,
    entries: Vec<Entry>,
    snapshot: Arc<Snapshot>,
    world: Option<Arc<dyn WorldAccess>>,
    out: Vec<Output>,
    /// Where API 0.1.4's `storage` kept the mods' files, beside the game's
    /// settings: a mod's folder there moves to its `data_dir` once
    /// (`set_storage`).
    storage: Option<PathBuf>,
    /// The screen's size in GUI pixels (`screen`).
    screen_size: (f32, f32),
    /// The game's screens, for `current-screen` and the like (API 0.1.5).
    ui: Arc<Ui>,
}

/// The `.hmod` files and mod folders in `dir`, by name.
pub fn find(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "hmod")
                || p.join(crate::manifest::MANIFEST).is_file()
        })
        .collect();
    found.sort();
    found
}

impl Mods {
    /// Reads every mod at `paths` and starts those `enabled` says are on.
    /// A command that `reserved` (vanilla's commands) or an earlier mod
    /// already uses is refused.
    pub fn load(
        paths: &[PathBuf],
        enabled: impl Fn(&str) -> bool,
        reserved: impl Fn(&str) -> bool,
    ) -> Mods {
        let mut mods = Mods::default();
        for path in paths {
            let modified = modified(path);
            let mut entry = Entry {
                path: path.clone(),
                manifest: None,
                status: Status::Off,
                cost: Duration::ZERO,
                run: None,
                modified,
            };
            match Package::read(path) {
                Err(e) => entry.status = Status::Failed(e),
                Ok(package) => {
                    let on = enabled(&package.manifest.id);
                    entry.manifest = Some(package.manifest.clone());
                    if on {
                        // Content is read before the world loads; a mod whose
                        // content is refused stays off, with the reason.
                        let content = crate::Content::read(path, &package.manifest);
                        entry.status =
                            match content.and_then(|_| mods.start(path, &package, &reserved)) {
                                Ok(run) => {
                                    entry.run = run;
                                    Status::Running
                                }
                                Err(e) => Status::Failed(e),
                            };
                    }
                }
            }
            if let (Status::Failed(reason), Some(m)) = (&entry.status, &entry.manifest) {
                log::warn!("Horizon: mod {} did not start: {reason}", m.id);
            }
            mods.entries.push(entry);
        }
        mods
    }

    fn engine(&mut self) -> Result<&(Engine, Linker<State>), String> {
        if self.engine.is_none() {
            let mut config = Config::new();
            config.wasm_component_model(true).consume_fuel(true);
            let engine = Engine::new(&config).map_err(|e| e.to_string())?;
            let mut linker = Linker::new(&engine);
            HorizonMod::add_to_linker::<State, HasSelf<State>>(&mut linker, |s| s)
                .map_err(|e| e.to_string())?;
            self.engine = Some((engine, linker));
        }
        Ok(self.engine.as_ref().expect("made above"))
    }

    /// Starts a mod's code; `Ok(None)` for a content-only mod, which has
    /// nothing to run (the game reads its content before the world loads).
    /// `path` is the mod's own `.hmod` or folder, for capability `storage`'s
    /// per-mod directory.
    fn start(
        &mut self,
        path: &Path,
        package: &Package,
        reserved: &dyn Fn(&str) -> bool,
    ) -> Result<Option<Running>, String> {
        let m = &package.manifest;
        let Some(wasm) = &package.wasm else {
            return Ok(None);
        };
        if m.side == Side::Server {
            return Err("server mods do not run yet".into());
        }
        // Two running mods with the same id would share one `data_dir`
        // (capability `storage`): the first loaded keeps the id.
        if self
            .entries
            .iter()
            .any(|e| e.run.is_some() && e.manifest.as_ref().is_some_and(|other| other.id == m.id))
        {
            return Err(format!("another running mod already uses id {}", m.id));
        }
        let taken: Vec<String> = self
            .entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| r.hooks.commands.iter().cloned())
            .collect();
        let snapshot = self.snapshot.clone();
        let ui = self.ui.clone();
        let (engine, linker) = self.engine()?;
        let component =
            Component::new(engine, wasm).map_err(|e| format!("not a component: {e}"))?;
        let ty = component.component_type();
        let imports: Vec<String> = ty
            .imports(engine)
            .map(|(name, _)| name.to_string())
            .collect();
        m.check_imports(imports.iter().map(String::as_str))?;
        let data_dir = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(STORAGE_DIR)
            .join(&m.id);
        let quota = m
            .storage_quota_mib
            .map_or(STORAGE_QUOTA, |mib| u64::from(mib) << 20);
        let max_keys = m
            .storage_max_keys
            .map_or(STORAGE_MAX_KEYS, |keys| keys as usize);
        // A mod with a bigger quota writes bigger things in one call.
        let write_budget = STORAGE_WRITE_BUDGET.max(quota / 32);
        let state = State {
            name: m.name.clone(),
            id: m.id.clone(),
            snapshot,
            world: None,
            can_edit: m.capabilities.contains(&crate::Capability::WorldEdit),
            breaking: Vec::new(),
            worn: 0,
            out: Vec::new(),
            panel: None,
            data_dir,
            storage_budget: write_budget,
            storage_write_budget: write_budget,
            storage_quota: quota,
            storage_max_keys: max_keys,
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY)
                .instances(16)
                .build(),
            screen: None,
            overlay: None,
            ui,
            can_give: m.capabilities.contains(&crate::Capability::InventoryEdit),
            given: 0,
            next_screen: 1,
            binds: Vec::new(),
            key_events: Vec::new(),
            screen_size: ScreenSize {
                width: 0.0,
                height: 0.0,
            },
        };
        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(FUEL_INIT).map_err(|e| e.to_string())?;
        let exports =
            HorizonMod::instantiate(&mut store, &component, linker).map_err(|e| reason(&e))?;
        let mut hooks = exports
            .horizon_mods_events()
            .call_init(&mut store)
            .map_err(|e| reason(&e))??;
        hooks.commands.retain(|c| {
            let free = !reserved(c) && !taken.contains(c);
            if !free {
                log::warn!("Horizon: mod {} cannot take /{c}: already a command", m.id);
            }
            free
        });
        self.out.append(&mut store.data_mut().out);
        Ok(Some(Running {
            store,
            exports,
            hooks,
        }))
    }

    /// True when at least one mod runs: the game skips its mod work otherwise.
    pub fn is_running(&self) -> bool {
        self.entries.iter().any(|e| e.run.is_some())
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// Whether a running mod declares `capability`: the game fills the
    /// snapshot's costlier parts (the target, the hotbar) only then.
    pub fn uses(&self, capability: crate::Capability) -> bool {
        self.entries.iter().any(|e| {
            e.run.is_some()
                && e.manifest
                    .as_ref()
                    .is_some_and(|m| m.capabilities.contains(&capability))
        })
    }
    /// What mods read until the next snapshot.
    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshot = Arc::new(snapshot);
    }
    /// The world `world` reads during the next hook calls; the game lends
    /// it around a tick or command and takes it back with `None`.
    pub fn set_world(&mut self, world: Option<Arc<dyn WorldAccess>>) {
        self.world = world;
    }
    /// Where API 0.1.4's `storage` kept the mods' files (`<root>/<id>/`).
    /// A running mod whose own `data_dir` does not exist yet gets that
    /// folder moved there, so what a mod saved before the merge with
    /// upstream's `storage` is still its own. The game sets it beside its
    /// settings.
    pub fn set_storage(&mut self, root: Option<PathBuf>) {
        self.storage = root.clone();
        let Some(root) = root else { return };
        for entry in &self.entries {
            let Some(run) = entry.run.as_ref() else {
                continue;
            };
            let data = run.store.data();
            let legacy = root.join(&data.id);
            if data.data_dir.exists() || !legacy.is_dir() {
                continue;
            }
            if let Some(parent) = data.data_dir.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::rename(&legacy, &data.data_dir) {
                log::warn!(
                    "Horizon: mod {} storage not moved from {}: {e}",
                    data.id,
                    legacy.display()
                );
            }
        }
    }
    /// The screen's size in GUI pixels (`screen`), given by the game.
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_size = (width, height);
    }
    /// What mods read of the game's screens until the next call (API 0.1.5).
    pub fn set_ui(&mut self, ui: Ui) {
        self.ui = Arc::new(ui);
    }
    /// The panels mods have over the game's screen (API 0.1.5): the mod's
    /// id, its name and the panel.
    pub fn overlays(&self) -> impl Iterator<Item = (&str, &str, &Screen)> {
        self.entries.iter().filter_map(|entry| {
            let data = entry.run.as_ref()?.store.data();
            let panel = data.overlay.as_ref().filter(|s| s.open)?;
            Some((data.id.as_str(), data.name.as_str(), panel))
        })
    }
    /// Gives the mod `id` an event for its panel (API 0.1.5).
    pub fn push_overlay_input(&mut self, id: &str, event: ScreenEvent) {
        for entry in &mut self.entries {
            let Some(run) = entry.run.as_mut() else {
                continue;
            };
            let data = run.store.data_mut();
            if data.id != id {
                continue;
            }
            if let Some(panel) = data.overlay.as_mut().filter(|s| s.open) {
                panel.input.push(event);
            }
            return;
        }
    }
    /// The mod with a screen open: its id, its name and the screen; none
    /// when no mod has one open.
    pub fn open_screen(&self) -> Option<(&str, &str, &Screen)> {
        for entry in &self.entries {
            let Some(run) = entry.run.as_ref() else {
                continue;
            };
            let data = run.store.data();
            let Some(screen) = data.screen.as_ref().filter(|s| s.open) else {
                continue;
            };
            return Some((data.id.as_str(), data.name.as_str(), screen));
        }
        None
    }
    /// The screen of the mod `id`, open or not; none when it has none.
    pub fn screen_of(&self, id: &str) -> Option<&Screen> {
        self.entries.iter().find_map(|entry| {
            let run = entry.run.as_ref()?;
            let data = run.store.data();
            (data.id == id).then_some(data.screen.as_ref()).flatten()
        })
    }
    /// Gives the mod `id` an event for its screen.
    pub fn push_screen_input(&mut self, id: &str, event: ScreenEvent) {
        for entry in &mut self.entries {
            let Some(run) = entry.run.as_mut() else {
                continue;
            };
            let data = run.store.data_mut();
            if data.id != id {
                continue;
            }
            if let Some(screen) = data.screen.as_mut() {
                screen.input.push(event);
            }
            return;
        }
    }
    /// Closes the mod `id`'s screen, as the player did with Escape: the mod
    /// gets the `closed` event.
    pub fn close_screen(&mut self, id: &str) {
        for entry in &mut self.entries {
            let Some(run) = entry.run.as_mut() else {
                continue;
            };
            let data = run.store.data_mut();
            if data.id != id {
                continue;
            }
            if let Some(screen) = data.screen.as_mut()
                && screen.open
            {
                screen.open = false;
                screen.input.push(ScreenEvent::Closed);
            }
            return;
        }
    }
    /// Gives the mod `id` a key event (`keys`).
    pub fn push_key_event(&mut self, id: &str, event: KeyEvent) {
        for entry in &mut self.entries {
            let Some(run) = entry.run.as_mut() else {
                continue;
            };
            let data = run.store.data_mut();
            if data.id != id {
                continue;
            }
            data.key_events.push(event);
            return;
        }
    }
    /// The key bindings of the mod `id` (`keys`).
    pub fn key_binds(&self, id: &str) -> Vec<KeyBind> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let run = entry.run.as_ref()?;
                let data = run.store.data();
                (data.id == id).then(|| data.binds.clone())
            })
            .flatten()
            .collect()
    }
    /// Every running mod's key bindings, with the mod's id.
    pub fn all_key_binds(&self) -> Vec<(String, KeyBind)> {
        let mut out = Vec::new();
        for entry in &self.entries {
            let Some(run) = entry.run.as_ref() else {
                continue;
            };
            let data = run.store.data();
            for bind in &data.binds {
                out.push((data.id.clone(), bind.clone()));
            }
        }
        out
    }
    /// The key the player bound to the mod's action; false when the mod has
    /// no such action.
    pub fn set_key(&mut self, id: &str, action: &str, key: &str) -> bool {
        for entry in &mut self.entries {
            let Some(run) = entry.run.as_mut() else {
                continue;
            };
            let data = run.store.data_mut();
            if data.id != id {
                continue;
            }
            let Some(bind) = data.binds.iter_mut().find(|b| b.action == action) else {
                return false;
            };
            bind.key = key.to_owned();
            return true;
        }
        false
    }

    /// Calls `f` on every running mod whose hooks `wants`, with `fuel`.
    fn each(
        &mut self,
        fuel: u64,
        wants: impl Fn(&Registration) -> bool,
        mut f: impl FnMut(&HorizonMod, &mut Store<State>) -> wasmtime::Result<()>,
    ) {
        for i in 0..self.entries.len() {
            if self.entries[i]
                .run
                .as_ref()
                .is_some_and(|r| wants(&r.hooks))
            {
                self.call(i, fuel, &mut f);
            }
        }
    }

    /// One call into mod `i`; a trap or an overrun turns the mod off.
    fn call<R>(
        &mut self,
        i: usize,
        fuel: u64,
        f: impl FnOnce(&HorizonMod, &mut Store<State>) -> wasmtime::Result<R>,
    ) -> Option<R> {
        let entry = &mut self.entries[i];
        let run = entry.run.as_mut()?;
        let data = run.store.data_mut();
        data.snapshot = self.snapshot.clone();
        data.world = self.world.clone();
        data.screen_size = ScreenSize {
            width: self.screen_size.0,
            height: self.screen_size.1,
        };
        data.ui = self.ui.clone();
        data.storage_budget = data.storage_write_budget;
        let started = Instant::now();
        let result = run
            .store
            .set_fuel(fuel)
            .and_then(|()| f(&run.exports, &mut run.store));
        // The world is only lent for the call.
        let data = run.store.data_mut();
        data.world = None;
        data.breaking.clear();
        data.worn = 0;
        data.given = 0;
        // An average over about the last 64 calls.
        entry.cost = (entry.cost * 63 + started.elapsed()) / 64;
        self.out.append(&mut run.store.data_mut().out);
        match result {
            Ok(r) => Some(r),
            Err(e) => {
                let reason = reason(&e);
                log::warn!("Horizon: mod {} stopped: {reason}", run.store.data().id);
                let name = run.store.data().name.clone();
                self.out.push(Output::Stopped {
                    name,
                    reason: reason.clone(),
                });
                entry.status = Status::Failed(reason);
                entry.run = None;
                None
            }
        }
    }

    /// A game tick.
    pub fn tick(&mut self, tick: u64) {
        self.each(
            FUEL_TICK,
            |h| h.tick,
            |m, s| m.horizon_mods_events().call_tick(s, tick),
        );
    }
    /// A rendered frame, `seconds` after the last.
    pub fn frame(&mut self, seconds: f32) {
        self.each(
            FUEL_FRAME,
            |h| h.frame,
            |m, s| m.horizon_mods_events().call_frame(s, seconds),
        );
    }
    /// A chat line the player received, as plain text.
    pub fn chat(&mut self, text: &str) {
        self.each(
            FUEL_EVENT,
            |h| h.chat,
            |m, s| m.horizon_mods_events().call_chat(s, text),
        );
    }
    /// Whether a running mod owns `/name`.
    pub fn has_command(&self, name: &str) -> bool {
        self.command_owner(name).is_some()
    }
    fn command_owner(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| {
            e.run
                .as_ref()
                .is_some_and(|r| r.hooks.commands.iter().any(|c| c == name))
        })
    }
    /// Runs `line` (without its slash) if a mod owns its command; false
    /// when none does.
    pub fn command(&mut self, line: &str) -> bool {
        let (name, args) = line.split_once(' ').unwrap_or((line, ""));
        let Some(i) = self.command_owner(name) else {
            return false;
        };
        let result = self.call(i, FUEL_EVENT, |m, s| {
            m.horizon_mods_events().call_command(s, name, args)
        });
        match result {
            Some(Ok(Some(text))) => self.out.push(Output::Feedback { text, error: false }),
            Some(Err(text)) => self.out.push(Output::Feedback { text, error: true }),
            Some(Ok(None)) | None => {}
        }
        true
    }
    /// The commands running mods own, for suggestions.
    pub fn commands(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| r.hooks.commands.iter().map(String::as_str))
    }

    /// The running mods' HUD panels.
    pub fn panels(&self) -> impl Iterator<Item = &Panel> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref()?.store.data().panel.as_ref())
    }
    /// What mods asked to show since the last call.
    pub fn drain(&mut self) -> Vec<Output> {
        std::mem::take(&mut self.out)
    }

    /// Restarts the mods whose file changed since they loaded (mod
    /// development: rebuild, and the game picks it up). True when any did.
    pub fn reload_changed(&mut self, reserved: impl Fn(&str) -> bool) -> bool {
        let mut any = false;
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let now = modified(&entry.path);
            let was_on = entry.status != Status::Off;
            if now == entry.modified || !was_on {
                continue;
            }
            any = true;
            self.entries[i].modified = now;
            self.entries[i].run = None;
            let path = self.entries[i].path.clone();
            let started = Package::read(&path).and_then(|p| {
                self.entries[i].manifest = Some(p.manifest.clone());
                self.start(&path, &p, &reserved)
            });
            let entry = &mut self.entries[i];
            let id = entry.manifest.as_ref().map_or("?", |m| m.id.as_str());
            match started {
                Ok(run) => {
                    log::info!("Horizon: mod {id} reloaded");
                    entry.run = run;
                    entry.status = Status::Running;
                }
                Err(e) => {
                    log::warn!("Horizon: mod {id} did not reload: {e}");
                    entry.status = Status::Failed(e);
                }
            }
        }
        any
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.is_dir() {
        // A folder mod changes when its component does.
        return std::fs::metadata(path.join(crate::manifest::WASM))
            .ok()?
            .modified()
            .ok();
    }
    meta.modified().ok()
}

/// A trap or error, as the player reads it.
fn reason(e: &wasmtime::Error) -> String {
    match e.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => "it took too long (an endless loop?)".into(),
        Some(Trap::UnreachableCodeReached) => "it crashed (a panic)".into(),
        Some(trap) => format!("it crashed: {trap}"),
        None => format!("{e:#}"),
    }
}

fn clip(text: String) -> String {
    match text.char_indices().nth(LINE_CHARS) {
        Some((at, _)) => text[..at].to_string(),
        None => text,
    }
}
impl mod_log::Host for State {
    fn write(&mut self, level: mod_log::Level, message: String) {
        let level = match level {
            mod_log::Level::Debug => log::Level::Debug,
            mod_log::Level::Info => log::Level::Info,
            mod_log::Level::Warn => log::Level::Warn,
            mod_log::Level::Error => log::Level::Error,
        };
        log::log!(level, "[mod {}] {}", self.id, clip(message));
    }
}
impl game::Host for State {
    fn get_player(&mut self) -> Option<Player> {
        self.snapshot.player.clone()
    }
    fn get_clock(&mut self) -> Option<Clock> {
        self.snapshot.clock
    }
    fn get_food_stats(&mut self) -> Option<FoodStats> {
        self.snapshot.food
    }
}
impl chat::Host for State {
    fn show(&mut self, text: String) {
        let shown = self
            .out
            .iter()
            .filter(|o| matches!(o, Output::Chat { .. }))
            .count();
        if shown < CHAT_PER_CALL {
            let name = self.name.clone();
            self.out.push(Output::Chat {
                name,
                text: clip(text),
            });
        }
    }
}
impl target::Host for State {
    fn get_block(&mut self) -> Option<Block> {
        self.snapshot.block.clone()
    }
    fn get_breaking(&mut self) -> Option<Block> {
        self.snapshot.breaking.clone()
    }
    fn get_breaking_progress(&mut self) -> Option<f32> {
        self.snapshot.breaking_progress
    }
    fn get_block_name(&mut self) -> Option<String> {
        self.snapshot.block_name.clone()
    }
    fn get_entity(&mut self) -> Option<Entity> {
        self.snapshot.entity.clone()
    }
}
impl inventory::Host for State {
    fn get_hotbar(&mut self) -> Vec<Option<Stack>> {
        self.snapshot.hotbar.clone()
    }
    fn get_selected(&mut self) -> u32 {
        self.snapshot.selected
    }
    fn get_food(&mut self, slot: u32) -> Option<Food> {
        self.snapshot
            .hotbar_food
            .get(slot as usize)
            .copied()
            .flatten()
    }
    fn item_tags(&mut self, id: String) -> Vec<String> {
        self.snapshot
            .item_tags
            .as_ref()
            .map_or_else(Vec::new, |tags| tags.of(&id))
    }
    fn give(&mut self, id: String, count: u32) -> bool {
        self.queue_give(id, count)
    }
    fn get_tags(&mut self, slot: u8) -> Vec<String> {
        let Some(tags) = self.snapshot.item_tags.as_ref() else {
            return Vec::new();
        };
        self.snapshot
            .hotbar
            .get(slot as usize)
            .and_then(|stack| stack.as_ref())
            .map_or_else(Vec::new, |stack| tags.of(&stack.id))
    }
}
impl State {
    fn queue_give(&mut self, id: String, count: u32) -> bool {
        // A stack above a few hundreds is not a thing a mod gives at once.
        let ok = self.can_give
            && self.ui.can_give
            && (1..=6400).contains(&count)
            && self.given < 64
            && self
                .snapshot
                .items
                .as_ref()
                .is_some_and(|items| items.iter().any(|i| i.id == id));
        if ok {
            self.given += 1;
            self.out.push(Output::Give { id, count });
        }
        ok
    }
}
impl hotbar::Host for State {
    fn select(&mut self, slot: u32) {
        if slot <= 8 {
            self.out.retain(|o| !matches!(o, Output::Select { .. }));
            self.out.push(Output::Select { slot });
        }
    }
}
impl keys::Host for State {
    fn bind(&mut self, action: String, default_key: String) -> Result<(), String> {
        if action.is_empty() || action.len() > 64 {
            return Err("an action's name is 1 to 64 characters".into());
        }
        if !default_key.starts_with("key.") {
            return Err(format!("{default_key} is not a key"));
        }
        self.binds.retain(|b| b.action != action);
        self.binds.push(KeyBind {
            action,
            default_key: default_key.clone(),
            key: default_key,
        });
        Ok(())
    }
    fn get(&mut self, action: String) -> Option<String> {
        self.binds
            .iter()
            .find(|b| b.action == action)
            .map(|b| b.key.clone())
    }
    fn poll(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.key_events)
    }
}
impl State {
    /// The mod's screen or panel with this id.
    fn surface(&mut self, id: u32) -> Option<&mut Screen> {
        [self.screen.as_mut(), self.overlay.as_mut()]
            .into_iter()
            .flatten()
            .find(|s| s.id == id)
    }
}
/// A panel's keys: at most `PANEL_KEYS`, only names the game knows how to
/// write (`key.…`) or `*`.
fn panel_keys(keys: Vec<String>) -> Vec<String> {
    keys.into_iter()
        .filter(|k| k == "*" || k.starts_with("key."))
        .take(PANEL_KEYS)
        .collect()
}
/// A sane area: finite numbers, a size of 0 or more.
fn clean_area(a: Area) -> Area {
    let fix = |v: f32| if v.is_finite() { v } else { 0.0 };
    Area {
        x: fix(a.x),
        y: fix(a.y),
        width: fix(a.width).max(0.0),
        height: fix(a.height).max(0.0),
    }
}
/// Tooltip lines the game draws: capped in number and length.
fn clean_tip(lines: Vec<TipLine>) -> Vec<TipLine> {
    lines
        .into_iter()
        .take(TIP_LINES)
        .map(|l| TipLine {
            text: clip(l.text),
            color: l.color,
        })
        .collect()
}
impl screen::Host for State {
    fn open(&mut self, title: String) -> Result<u32, String> {
        // A mod has one screen: the old one is closed first.
        self.screen = None;
        let id = self.next_screen;
        self.next_screen += 1;
        self.screen = Some(Screen {
            id,
            title,
            open: true,
            shapes: Vec::new(),
            textures: Vec::new(),
            input: Vec::new(),
            elements: Vec::new(),
            overlay: None,
        });
        Ok(id)
    }
    fn get_size(&mut self, id: u32) -> Option<ScreenSize> {
        let size = self.screen_size;
        self.surface(id).map(|_| size)
    }
    fn draw(&mut self, id: u32, shapes: Vec<ScreenShape>) {
        if let Some(screen) = self.surface(id) {
            screen.shapes = shapes.into_iter().take(SCREEN_SHAPES).collect();
        }
    }
    fn get_input(&mut self, id: u32) -> Vec<ScreenEvent> {
        self.surface(id)
            .map_or_else(Vec::new, |s| std::mem::take(&mut s.input))
    }
    fn close(&mut self, id: u32) {
        let Some(screen) = self.surface(id) else {
            return;
        };
        if !screen.open {
            return;
        }
        screen.open = false;
        screen.input.push(ScreenEvent::Closed);
    }
    fn upload_texture(
        &mut self,
        id: u32,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<(), String> {
        if id as usize >= SCREEN_TEXTURES {
            return Err(format!("an image's id is 0 to {}", SCREEN_TEXTURES - 1));
        }
        if width == 0 || height == 0 || width > SCREEN_TEXTURE_SIDE || height > SCREEN_TEXTURE_SIDE
        {
            return Err(format!(
                "an image is 1 to {SCREEN_TEXTURE_SIDE} pixels wide and high"
            ));
        }
        if rgba.len() != (width * height * 4) as usize {
            return Err("an image has width * height * 4 bytes".into());
        }
        let Some(screen) = self.screen.as_mut() else {
            return Err("the mod has no screen".into());
        };
        match screen.textures.iter_mut().find(|t| t.id == id) {
            Some(texture) => {
                texture.width = width;
                texture.height = height;
                texture.pixels = rgba;
                texture.version += 1;
            }
            None => screen.textures.push(Texture {
                id,
                width,
                height,
                pixels: rgba,
                version: 1,
            }),
        }
        Ok(())
    }
    fn paint(&mut self, id: u32, elements: Vec<ScreenElement>) {
        let Some(screen) = self.surface(id) else {
            return;
        };
        screen.elements = elements
            .into_iter()
            .take(SCREEN_ELEMENTS)
            .map(|e| match e {
                ScreenElement::Label(mut l) => {
                    l.text = clip(l.text);
                    ScreenElement::Label(l)
                }
                ScreenElement::Tooltip(mut t) => {
                    t.extra = clean_tip(t.extra);
                    ScreenElement::Tooltip(t)
                }
                ScreenElement::Tip(mut t) => {
                    t.lines = clean_tip(t.lines);
                    ScreenElement::Tip(t)
                }
                other => other,
            })
            .collect();
    }
    fn get_text_width(&mut self, text: String) -> f32 {
        let Some(widths) = self.ui.text_widths.as_ref() else {
            return text.chars().count() as f32 * 6.0;
        };
        clip(text)
            .chars()
            .map(|c| widths.get(c as usize).copied().unwrap_or(8.0))
            .sum()
    }
    fn open_panel(&mut self, title: String, area: Area, keys: Vec<String>) -> Result<u32, String> {
        // A mod has one panel: the old one is closed first.
        self.overlay = None;
        let id = self.next_screen;
        self.next_screen += 1;
        self.overlay = Some(Screen {
            id,
            title,
            open: true,
            shapes: Vec::new(),
            textures: Vec::new(),
            input: Vec::new(),
            elements: Vec::new(),
            overlay: Some(Overlay {
                area: clean_area(area),
                keys: panel_keys(keys),
            }),
        });
        Ok(id)
    }
    fn set_panel_area(&mut self, id: u32, area: Area, keys: Vec<String>) {
        let Some(overlay) = self.surface(id).and_then(|s| s.overlay.as_mut()) else {
            return;
        };
        overlay.area = clean_area(area);
        overlay.keys = panel_keys(keys);
    }
    fn get_game_screen(&mut self) -> GameScreen {
        self.ui.screen
    }
    fn get_menu_origin(&mut self) -> Option<Point> {
        self.ui.menu_origin
    }
    fn get_item_at(&mut self, x: f32, y: f32) -> Option<String> {
        self.ui
            .slots
            .iter()
            .find(|([sx, sy, w, h], _)| x >= *sx && y >= *sy && x < sx + w && y < sy + h)
            .map(|(_, id)| id.clone())
    }
    fn get_cursor(&mut self) -> Point {
        self.ui.cursor
    }
    fn get_modifiers(&mut self) -> Modifiers {
        self.ui.modifiers
    }
}
impl items::Host for State {
    fn list(&mut self) -> Vec<Item> {
        self.snapshot
            .items
            .as_ref()
            .map_or_else(Vec::new, |items| items.as_ref().clone())
    }
    fn get(&mut self, id: String) -> Option<Item> {
        self.snapshot
            .items
            .as_ref()?
            .iter()
            .find(|item| item.id == id)
            .cloned()
    }
}
impl recipes::Host for State {
    fn list(&mut self) -> Vec<Recipe> {
        self.snapshot
            .recipes
            .as_ref()
            .map_or_else(Vec::new, |recipes| recipes.as_ref().clone())
    }
}
impl world::Host for State {
    fn get_block(&mut self, pos: BlockPos) -> Option<WorldBlock> {
        self.world.as_ref()?.block(&pos)
    }
    fn broken_blocks(&mut self) -> Vec<BlockBroken> {
        self.snapshot.broken.clone()
    }
    fn break_block(&mut self, pos: BlockPos, drops: bool) -> bool {
        self.queue_break(pos, drops, false)
    }
    fn break_block_with_held(&mut self, pos: BlockPos) -> bool {
        self.queue_break(pos, true, true)
    }
    fn damage_held(&mut self, amount: u32) -> Option<u32> {
        if !self.can_edit {
            return None;
        }
        let (damage, max) = self.world.as_ref()?.held_wear()?;
        self.worn = self.worn.saturating_add(amount);
        self.out.push(Output::DamageHeld { amount });
        Some(max.saturating_sub(damage.saturating_add(self.worn)))
    }
    fn get_chunk_surface(&mut self, cx: i32, cz: i32) -> Option<ChunkSurface> {
        self.world.as_ref()?.chunk_surface(cx, cz)
    }
    fn get_biome_names(&mut self) -> Vec<String> {
        self.world
            .as_ref()
            .map_or_else(Vec::new, |w| w.biome_names())
    }
    fn get_map_colors(&mut self) -> Vec<u32> {
        self.world
            .as_ref()
            .map_or_else(Vec::new, |w| w.map_colors())
    }
}
impl State {
    fn queue_break(&mut self, pos: BlockPos, drops: bool, with_held: bool) -> bool {
        let at = [pos.x, pos.y, pos.z];
        let ok = self.can_edit
            && !self.breaking.contains(&at)
            && self.world.as_ref().is_some_and(|w| w.breakable(&pos));
        if ok {
            self.breaking.push(at);
            self.out.push(Output::Break {
                pos: at,
                drops,
                with_held,
            });
        }
        ok
    }
}
impl hud::Host for State {
    fn set_panel(&mut self, corner: Corner, lines: Vec<Line>) {
        let lines = lines
            .into_iter()
            .take(HUD_LINES)
            .map(|l| Line {
                text: clip(l.text),
                color: l.color & 0xFF_FFFF,
            })
            .collect();
        self.panel = Some(Panel { corner, lines });
    }
    fn clear(&mut self) {
        self.panel = None;
    }
}
impl State {
    fn storage_get(&mut self, key: &str) -> Option<Vec<u8>> {
        storage_key_ok(key)
            .then(|| std::fs::read(self.data_dir.join(key)).ok())
            .flatten()
    }
    fn storage_set(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        if !storage_key_ok(key) {
            return Err(format!("{key:?} is not [a-z0-9_.-]{{1,64}}"));
        }
        let len = value.len() as u64;
        if len > self.storage_budget {
            return Err("too much storage traffic in one call".into());
        }
        // Everything already on disk but the file this call replaces.
        let existing: Vec<std::fs::DirEntry> = std::fs::read_dir(&self.data_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy() != key)
            .collect();
        if existing.len() >= self.storage_max_keys {
            return Err(format!("would hold over {} keys", self.storage_max_keys));
        }
        let used: u64 = existing
            .iter()
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();
        if used + len > self.storage_quota {
            return Err(format!(
                "would use over {} MiB of storage",
                self.storage_quota >> 20
            ));
        }
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        std::fs::write(self.data_dir.join(key), value).map_err(|e| e.to_string())?;
        self.storage_budget -= len;
        Ok(())
    }
    fn storage_delete(&mut self, key: &str) -> bool {
        storage_key_ok(key) && std::fs::remove_file(self.data_dir.join(key)).is_ok()
    }
    fn storage_keys(&mut self) -> Vec<String> {
        let mut keys: Vec<String> = std::fs::read_dir(&self.data_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.metadata().is_ok_and(|m| m.is_file()))
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                storage_key_ok(&name).then_some(name)
            })
            .collect();
        keys.sort();
        keys
    }
}
impl storage::Host for State {
    fn get(&mut self, key: String) -> Option<Vec<u8>> {
        self.storage_get(&key)
    }
    fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), String> {
        self.storage_set(&key, &value)
    }
    fn delete(&mut self, key: String) {
        self.storage_delete(&key);
    }
    fn list_keys(&mut self) -> Vec<String> {
        self.storage_keys()
    }
    // API 0.1.4's names, kept so mods built on it still link: the same
    // files, the same limits.
    fn read(&mut self, name: String) -> Option<Vec<u8>> {
        self.storage_get(&name)
    }
    fn write(&mut self, name: String, data: Vec<u8>) -> Result<(), String> {
        self.storage_set(&name, &data)
    }
    fn remove(&mut self, name: String) -> bool {
        self.storage_delete(&name)
    }
    fn files(&mut self) -> Vec<String> {
        self.storage_keys()
    }
}

/// A storage key: one path segment of `[a-z0-9_.-]{1,64}`, not `.` or `..`.
/// No `/`, so a key can never leave the mod's own directory.
fn storage_key_ok(key: &str) -> bool {
    let chars_ok = (1..=64).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
    chars_ok && key != "." && key != ".."
}
