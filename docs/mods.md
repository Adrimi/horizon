# Horizon mods

Horizon gets its own mod system, written in Rust and run in a sandbox, plus
two ways to bring over what Java players use: native Horizon ports of popular
mods and the data-driven parts of packs and mods. Fabric and NeoForge jars do
**not** run as they are: [research/java-mod-compat.md](research/java-mod-compat.md)
explains why and what we do instead.

Mods are **off and absent by default**. A fresh install has no `horizon-mods`
folder, loads no runtime and plays exactly like vanilla 26.3.

| Part | Where | State |
|---|---|---|
| The contract (WIT) | `crates/horizon-mod-sdk/wit/horizon.wit` | 0.1.4: log, game-read, chat, hud, target, inventory-read, hotbar, world-read, world-edit, keys, storage, screen; tick, frame, chat and command hooks |
| SDK for mod authors | `crates/horizon-mod-sdk` | 0.1 |
| Example mod | `crates/horizon-mod-sdk/examples/coords-hud` | coordinates, facing and time HUD, `/where`, `/coords` |
| Runtime | `crates/horizon-mods` | wasmtime 36 LTS, component model |
| Authoring tool | `crates/cargo-horizon-mod` | `cargo horizon-mod new / build / run` |
| In the game | `crates/horizon-client/src/game/mods.rs`, Options > Mods | client mods |
| Launcher mod browser (Modrinth, CurseForge with a key) | [mod-browser.md](mod-browser.md) | resource packs, shader packs, modpacks |
| `.hmod` index, signing, server mods, registries | this page, [Roadmap](#roadmap) | planned |

## Design goals

1. **Safe to install.** A mod is untrusted code. It runs in a WebAssembly
   sandbox with no file, network, process or clock access, a memory cap and a
   time budget per call. It can only do what its capabilities grant, and the
   player sees those before turning it on.
2. **One file for every platform.** The same `.hmod` runs on Windows, macOS
   and Linux, x86-64 and ARM: authors build once, players never meet "wrong
   build for my OS".
3. **Free when unused.** No mod on: no engine, no thread, no compile, one
   branch per hook. Measured below.
4. **Vanilla stays vanilla.** Mods add; they cannot patch the game's code the
   way Mixins do. What they may change is a list of hooks the game calls at
   fixed points, so the parity harnesses keep testing the same game.
5. **Stable for authors.** The contract is a versioned WIT package; a mod
   built for API 0.1 keeps loading on later 0.1.x games.

## How it works

```
 mod crate (Rust)                         Horizon
 ───────────────                          ───────
 impl Mod for MyMod { ... }               horizon-mods::Mods
 export_mod!(MyMod)                         ├─ Engine (cranelift), made on first mod
        │ cargo horizon-mod build           ├─ one Store per mod: 64 MiB, fuel per call
        ▼                                   ├─ Linker: log, game, chat, hud (WIT imports)
 my_mod.hmod = zip {                        └─ calls the events export: init, tick,
   horizon-mod.toml,                             frame, command, chat
   mod.wasm  (a WebAssembly component)
 }
```

- **WebAssembly components.** Mods compile to `wasm32-unknown-unknown`;
  `cargo horizon-mod build` wraps the module into a
  [component](https://component-model.bytecodealliance.org/) for the
  `horizon:mods/horizon-mod` world. The WIT file is the whole API: host
  functions the mod imports (`log`, `game`, `chat`, `hud`) and the `events`
  interface it exports. No WASI: the component cannot even name a file or a
  socket.
- **Capabilities are imports.** Every imported interface except `log` is a
  capability. The manifest declares them; the game reads the component's
  actual imports and refuses a mod whose code imports one its manifest does not
  declare (`uses capability chat, missing from horizon-mod.toml`). So the list
  the player reads is the truth, not a promise.
- **Hooks on request.** `init` returns a `registration`: the commands the mod
  owns and which of `tick`, `frame` and `chat` it wants. The game calls only
  those. A mod that wants nothing per frame costs nothing per frame.
- **Retained HUD.** A mod sets its panel (corner and coloured lines) when it
  changes; the game draws it every frame with the extras' overlay code. Most
  HUD mods need no frame hook at all.
- **Snapshots, not callbacks into the world.** Once per tick (and before a
  command) the game copies what mods may read (player position, look, health,
  food, dimension, world clock) into a snapshot shared by all mods. Mods never
  hold references into the game, so they cannot stall or corrupt it.

### What a mod can do (API 0.1)

| Interface | Capability | Functions |
|---|---|---|
| `log` | always | `write(level, message)`, in the game log as `[mod <id>]` |
| `game` | `game-read` | `get-player()`: x y z, yaw, pitch, health, food, dimension; `get-clock()`: game time, day time. Since 0.1.2: `get-food-stats()`: food, saturation and exhaustion as `FoodData` keeps them |
| `chat` | `chat` | `show(text)`: a line only the local player sees, after the mod's name (8 per call) |
| `hud` | `hud` | `set-panel(corner, lines)`, `clear()`: up to 16 lines of 256 characters |
| `target` (0.1.1) | `target` | `get-block()`: the block under the crosshair within reach; `get-breaking()`: the block being broken (survival and adventure, from its first tick to its last). Each with its position, id, hardness, `requires-tool` and every block tag it is in (`minecraft:mineable/pickaxe`, `minecraft:needs_iron_tool`, ...). Since 0.1.2: `get-breaking-progress()` (0 to 1), `get-block-name()` (the looked-at block's name in the game's language), `get-entity()` (the mob an attack would hit, before any block: its id, type id, name, health and max health) |
| `inventory` (0.1.1) | `inventory-read` | `get-hotbar()`: slots 0 to 8, each stack's id, count, damage, max damage, Efficiency level, and how it breaks the targeted block (the one being broken, else the one looked at): `mining-speed` (`Tool.getMiningSpeed` plus Efficiency's `mining_efficiency`, as `Player.getDestroySpeed` adds it) and `correct-for-drops`; `get-selected()`. Since 0.1.2: `get-food(slot)`: a hotbar slot's `minecraft:food` component (nutrition and the saturation `FoodData.eat` adds), none for an item that is not food. Since 0.1.3: `item-tags(id)`: every tag an item is in (`minecraft:axes`, `minecraft:pickaxes`, `minecraft:swords`, material tags, ...), empty for an unknown id, which also works for the `tool` of a `block-broken`; `get-tags(slot)`: the same for the item in hotbar slot 0 to 8, empty for an empty slot |
| `hotbar` (0.1.1) | `hotbar` | `select(slot)`: selects slot 0 to 8 as the number keys do, before the next tick (a server hears of it as from the keys); the last call in a tick wins, other values are ignored |
| `world` (0.1.2) | `world-read`, `world-edit` | `get-block(pos)`: the block at a position, with its state properties and every block tag it is in; `broken-blocks()`: the blocks broken since the last tick, with the tool and whether the player was sneaking; `break-block(pos, drops)`, `break-block-with-held(pos)`: breaking a block, with or without the loot rolled with the held item; `damage-held(amount)`: wearing the item in the hand. Since 0.1.4: `get-chunk-surface(cx, cz)`: the map surface of a loaded chunk (256 columns: map colour, height, water depth, biome), `get-biome-names()` and `get-map-colors()`: the biome names and the map colours, by index |
| `keys` (0.1.4) | `keys` | `bind(action, default-key)`: the mod's own key, shown in Controls > Key Binds; `get(action)`: the key in use; `poll()`: the key events since the last call |
| `storage` (upstream; 0.1.5 extended) | `storage` | `get(key)`, `set(key, value)`, `delete(key)`: a per-mod folder of small files (keys `[a-z0-9_.-]{1,64}`), default 64 keys and 4 MiB, at most 1 MiB written per call; the manifest may ask for more (`storage_quota_mib` up to 256, `storage_max_keys` up to 4096); `list-keys()`: the mod's keys. The folder is `.horizon-mod-data/<id>` beside the mod, readable already in `init`. API 0.1.4's `read`/`write`/`remove`/`files` stay as deprecated aliases of the same calls; what a mod saved in 0.1.4's `horizon-mod-data/<id>` is moved to the new folder once. |
| `screen` (0.1.4) | `screen` | `open(title)`: a screen the mod draws and handles itself, with the mouse free; `draw(id, shapes)`: rectangles, text and images; `get-input(id)`: the keys, the mouse and the wheel, and the `closed` event; `upload-texture(width, height, rgba)`: the mod's own picture (8 of 256x256 at most) |
| `screen` (0.1.5) | `screen` | `paint(id, elements)`: `fill` (flat or vertical gradient, 0xAARRGGBB), `label` (alpha, scale), `sprite` (any GUI atlas sprite by texture path, tinted), `picture` (part of an uploaded image), `item` (icon with count), `tooltip` (the game's tooltip of an item plus own lines), `tip` (own tooltip box); `open-panel(title, area, keys)` / `set-panel-area`: a panel over the game's own inventory, container or crafting screen (the mouse is the mod's inside the area; `keys` are the keys it takes, `*` is all but Escape); `get-game-screen()`, `get-menu-origin()`, `get-item-at(x, y)`, `get-cursor()`, `get-modifiers()`, `get-text-width(text)` |
| `items` (0.1.5) | `items-read` | `list()`: every item (id, name, max stack, tags) in the creative search order; `get(id)` |
| `recipes` (0.1.5) | `recipes-read` | `list()`: every recipe with fixed slots (crafting, cooking, stonecutting, smithing), inputs as lists of accepted items |
| `inventory.give` (0.1.5) | `inventory-edit` | `give(id, count)`: as the creative inventory does; false outside creative or another's world |
| `weather` (0.1.6) | `weather-read` | `get-weather()`: rain and thunder levels over the player |
| `settings` (0.1.6) | `settings-read` | `get-particles()`: the Particles setting (0 all, 1 decreased, 2 minimal) |
| `particles` (0.1.6) | `particles` | `define-sprite(sprite)`: the mod's own picture (up to 64x64, 16 in all); `spawn(options)`: a particle with its own motion (gravity, wind, sway, spin, linger, collide, fade, size curve, flat); `landed()`: where its colliding particles landed |
| `atmosphere` (0.1.6) | `atmosphere` | `set-haze(option<haze>)`: closes the fog in and tints it |
| `inventory` / `game` / `hud` (0.1.16) | `inventory-read`, `inventory-edit`, `game-read`, `hud` | `get-tools()`: the hotbar's tools (`damage-per-block`, enchantments); `get-slots()`: all 41 inventory slots; `move-item(source-slot, target-slot)`: moves or swaps two slots (0-35, 40) in a world this game runs; `get-input-state()`: sneaking and the attack button; `show-action-bar(text, color)`: a line over the hotbar |
| `chat` (0.1.15) | `chat` | `get-messages()`: the chat messages received since the last tick, with their sender (name, UUID) when a player wrote them; the `chat` hook now fires for every message the chat shows, server ones included |
| `particles` / `world` (0.1.19) | `particles`, `world-read` | `set-suppressed(types, scale)`, `get-game-sprite(name)`, `register-command-particle(name, options)`; `watch-random-tick-blocks(blocks)`, `removed-blocks()` |
| `entity` / `light-events` (0.1.20) | `entity-read`, `light-events` | `entity.get-detail(id)` (fuse, glowing, in-water, squish, carried block); `nearby` also lists projectiles, item frames and primed TNT; `light-events.poll()` (beacon and guardian beams, fireflies, sonic booms) |
| `slots` / `items` / `world` / `overlay` / `entity` (0.1.21) | `slot-decorate`, `items-read`, `world-read`, `world-edit`, `overlay`, `entity-read` | `slots`: `picture` and `square` marks, `upload-texture`, `set-screen-marks`, `set-hud-marks`; `items.get-name-color`; `world.get-light-level`, `world.run-command`; `overlay` `world-picture`; other players in `entity.nearby`, `entity.get-face` |
| `particles` / `weather` / `world` / `audio` (0.1.22) | `particles`, `weather-read`, `world-read`, `audio` | `particles.spawn-rotated(options, rotation)`; `weather.set-default-weather(visible)`; `world.get-biome-fog(pos)`; `audio.play(sound, place, volume, pitch)`; `get-game-sprite` also gives the GUI atlas' `environment/*` and `particle/*` |
| `input` / `overlay` / `game` / `world` (0.1.23) | `input`, `overlay`, `game-edit`, `world-edit` | `input.poll()` (mouse wheel and buttons while playing), `input.set-wheel-captured(bool)`; `overlay` `world-box` (3D box edges, optionally through blocks); `game.add-exhaustion(amount)`, `game.set-break-speed-modifier(m)`; `world.merge-drops-at(pos)` |
| `world` / `game` (0.1.24) | `world-edit`, `game-read` | `world.fall-block(pos, v, drops, place)`, `world.set-block(pos, id, properties)`, `world.spawn-item(pos, id, count)`; `game.get-game-mode()`, `game.get-player-tags()` |
| `weather` / `particles` / `world` / `game` / `audio` (0.1.25) | `weather-read`, `particles`, `world-read`, `game-read`, `audio` | `weather.set-default-splashing`; `particles.get-particles`, `rotation::crossed`; `world.set-weather-passthrough`; `game.get-cloud-height`; `audio.register(name, ogg)` |
| `particles` / `audio` (0.1.26) | `particles`, `audio` | `particles.remove(sprite, x, y, z)`; `audio.register-file(name)` (plik `sounds/<name>.ogg` z `.hmod`, pakowany przez `cargo-horizon-mod build`) |
| `tooltip` / `screen` / `exchange` (0.1.27) | `tooltip`, `screen`, `exchange` | `tooltip.set-lines`, `get-hovered-enchantments`; `screen.get-station`; `exchange.publish/withdraw/read/list-keys` |
| `tooltip` (0.1.28) | `tooltip` | `get-hovered-stack` (z zawartością kontenera), `set-hovered-tooltip` (własny tooltip stosu, zastępuje linie gry); limity 256×256 px i 128 elementów |
| `screen` images (0.1.31) | `screen` | `upload-texture` up to 512×512 (overlay images stay 256×256), `get-max-texture-side`, `set-texture-filter(id, nearest/linear)` for smooth scaling |
| `world` details, `game` viewport (0.1.32) | `world-read`, `game-read` | `world.get-chunk-details` (top block, ambiguity, light per column), `game.get-viewport` (GUI scale and window size), `game.get-server-view-distance` |
| `chat-decorate` (0.1.18) | `chat-decorate` | `set-decoration(id, decoration)`: like `set-picture` plus a gap, a place in the text (`text(n)`: before character n) and a style (offset, size, colour) for each rectangle; `players` (0.1.18, `players`): `get-players()` lists the player list (name, display name, UUID) |
| `chat-decorate` (0.1.15) | `chat-decorate` | `set-picture(id, picture)`: a picture (a player's skin regions or a GUI sprite) at the start of a message's first line; the game indents the text; set it in the `chat` hook |
| `resources` (0.1.14) | `resource-read` | `namespaces()`, `list-files(namespace, prefix)`, `read-file(namespace, path)`: read-only files of the enabled resource packs (`assets/<namespace>/...`); since 0.1.17 `file-origin(namespace, path)`: `vanilla` or the id of the pack the file comes from |
| `texture-rules` (0.1.14) | `texture-rules` | `upload-sprite(name, png)` adds a picture to the block atlas; `register-rule(rule)` / `clear-rules()`: tiles for block faces by `pattern` (8 neighbour bits), `random`, `position` or `quarters`; the mesher applies them, no mod code runs per face |
| `lighting` (0.1.13) | `lighting` | `set-lights(lights)`: the mod's light sources (x, y, z, level 0-15; the game lights what is near the 16 nearest the camera), `get-block-emission(id)`: a block's `lightEmission`; since 0.1.13 `entity.nearby` also lists the local player (kind `player`, held items, on fire) |
| `world` (0.1.12) | `world-read` | `watch-random-ticks(tags)`, `random-ticked-blocks()`: the blocks in the watched tags that got a random tick since the last tick (the positions the client's ambient effects use, at most 2048), read in `tick`; `particles.options.tumble` turns a particle over about a horizontal axis |
| `world` (0.1.6) | `world-read` | `get-biome(pos)` (temperature, precipitation, downfall, foliage/grass/water colours), `get-surface(pos, start-y)`, `is-solid(pos)` |
| `overlay` (0.1.7) | `overlay` | `paint(layer, elements)` / `clear(layer)`: the `screen.paint` elements in the HUD (`under-bars` before the status bars, `over-bars` after), no mouse or keyboard taken; `paint-world(elements)`: labels, sprites and rectangles at points of the world, projected with the last frame's camera; `get-size()`, `get-text-width(text)`, `upload-texture(id, w, h, rgba)` (7 images), `get-hud-state()` |
| `entity` (0.1.8) | `entity-read`, `entity-edit` | `nearby(type-id, radius)`: mobs, items on the ground and experience orbs around the player (id, type, kind, name, position, yaw, height, health, xp, on-fire, hostile, held items), nearest first; `remove(id)`, `set-xp(id, xp)` with `entity-edit`; `get-face(uuid)` (none for now) |
| `target` (0.1.9) | `target` | `get-entity-armor()`, `get-entity-absorption()` (0 for mobs), `get-block-face()` (`north`..`down`) |
| `inventory` (0.1.10) | `inventory-read` | `get-inventory()`: the 36 slots, the armour, the off hand and the stack on the cursor, each with its components as opaque text; `get-crafting()`: the open inventory's grid and its result; `game.get-session()`: the world's number and the ms since it loaded |
| `game` / `inventory` / `keys` (0.1.11) | `game-read`, `inventory-read`, `keys` | `get-health()`, `get-effects()`, `get-difficulty()`; `get-food-of(id)`, `get-harmful-food(id)`, `get-offhand()`; `is-down(key)` |
| `tooltip` (0.1.11) | `tooltip` | `get-hovered()`: the item whose tooltip the game drew; `set-content(item, content)`: room and `screen` elements added under that item's tooltip |
| `debug` (0.1.11) | `debug` | `set-lines(lines)`: the mod's group in the debug screen (F3) |
| `slots` (0.1.10) | `slots-read`, `slot-decorate` | `get-hovered-slot()`; `set-marks(marks)`: frames, dots and fills the game draws on its slots (hotbar HUD, inventory and containers, creative hotbar row) |
| `world.get-container` (0.1.9) | `world-read` | the slots of the chest, barrel, furnace, crafter... the player looks at or breaks |
| `hud.get-bars()` (0.1.7) | `hud` | the containers of the hunger icons and hearts as the game drew them (after the shaking) |
| `game` (0.1.7) | `game-read` | `get-seed()`, `get-fov()`; `world.get-chunk-columns(cx, cz, y)`: the chunk surface scanned down from `y` (a cave map) |

The target and the hotbar are read into the snapshot only while a running
mod declares `target` or `inventory-read`, so they cost nothing otherwise.
API 0.1.1 adds imports only: a mod built for 0.1.0 links and runs unchanged
(wasmtime matches `@0.1.0` imports and exports to the game's `@0.1.1`).
The same holds for 0.1.2, 0.1.3 and 0.1.4: only new functions in existing
interfaces, no record changes, so mods built for any earlier 0.1.x keep
loading. The item tags of 0.1.3 are indexed once at start and shared by
reference in every snapshot, not copied each tick. The mod's screen images
live in the atlas' bottom rows, which were reserved for them, so the atlas
does not grow for them.

| Hook (export) | When |
|---|---|
| `init() -> result<registration, string>` | once, after loading; an error keeps the mod off and is shown |
| `tick(tick)` | each game tick while in a world (20 per second) |
| `frame(seconds)` | each rendered frame |
| `command(name, args) -> result<option<string>, string>` | `/name args` typed in chat, for a command the mod registered |
| `chat(text)` | each chat line the player receives, as plain text |

Commands: vanilla's command names win. A mod asking for `/give` does not get
it (logged), and two mods cannot share a name (the first one loaded keeps it).
The chat box colours and Tab-completes a mod's commands like vanilla's, for
every player whatever their permission level, and takes any text after the
name (the mod reads it as `args`; API 0.1 has no way to declare argument types).

### Recognising a tool (API 0.1.3)

A mod that wants to know what the player holds asks for the item's tags, not
for its name: a mod's axe is an axe too, and no name ends in `_axe` by
accident. `inventory::item_tags(id)` gives the tags of any item id (also the
`tool` of a `block-broken`), `inventory::get_tags(slot)` those of the item in
a hotbar slot. The lists are complete: nested tags are already expanded, as
vanilla's `TagLoader` does.

```rust
use horizon_mod_sdk::inventory;

/// Whether the item is an axe, by its tags.
fn is_axe(id: &str) -> bool {
    inventory::item_tags(id).iter().any(|t| t == "minecraft:axes")
}
```

The tool classes have tags: `minecraft:axes`, `minecraft:pickaxes`,
`minecraft:shovels`, `minecraft:swords`, `minecraft:hoes`, `minecraft:spears`.
The material tags (`minecraft:iron_tool_materials` and its sisters) hold the
*materials* (`minecraft:iron_ingot`), not the tools made of them, so they say
nothing about a tool's tier.

A tool's tier is not in any tag either. What the game computes is in
`inventory::get-hotbar()`: `correct-for-drops` is `Tool.isCorrectForDrops` for
the block being broken (else the one looked at), so a pickaxe that may mine
the block is the one whose `correct-for-drops` is true. Read it while that
block is being broken: the game recomputes it for the new target as soon as
the block is gone.

### Planned interfaces

Each is a new capability, added in a 0.x minor version so older mods keep
loading. Ordered by what the first native ports need ([research](research/java-mod-compat.md#port-first)):

| Interface | For | Notes |
|---|---|---|
| `keys` | keybinds | 0.1.4: the mod's own binds appear in Controls; the events are polled |
| `target` | block state properties and NBT summary, non-mob entities (0.1.2 has the block, its tags and name, the one being broken and its progress, the mob in the crosshair) | Jade/WTHIT-style tooltips |
| `inventory-read` | the whole inventory, offhand, armour, open container (0.1.2 has the hotbar and its food, 0.1.3 the item tags) | AppleSkin, recipe viewers |
| `recipes` | the recipe registry and lookups | JEI/EMI-style viewers |
| `ui` | widgets: text, item icons, lists, a screen the mod owns, a settings page in Options > Mods (0.1.4 has `screen`: a screen the mod draws and handles itself) | built from vanilla's widgets (`ui`) so it matches the menus |
| `world-read` | blocks and biomes around the player, within the render distance (0.1.4 has the chunk surface, the biomes and the map colours) | minimaps, schematics |
| `render` | world-space lines, boxes and ghost blocks in a per-frame budget | Litematica-style placement, waypoints |
| `storage` | a per-mod folder of key/value files, quota'd | 0.1.4 |
| `registries` | new blocks, items, entities and recipes, mostly data: JSON models, textures, loot and recipes in the `.hmod`'s `assets/` and `data/` | server side; see [Multiplayer](#multiplayer) |
| `server` | world events (block break/place, entity spawn/death, player join), game rules, server commands | runs where the world is simulated |
| `net` | a mod's own client↔server channel (custom payloads) | only between the two halves of the same mod |

There is no file, socket, process or raw-input interface, on purpose.

## Choosing the runtime

| Option | Sandbox | One file for all OSes | Typed API | Cost | Verdict |
|---|---|---|---|---|---|
| **wasmtime + component model** | yes: memory, fuel, no imports but ours | yes | WIT → Rust on both sides | 85 ns per empty hook call; 50 ms to compile a 64 KB mod on load | **chosen** |
| wasmtime, core modules with a hand ABI | yes | yes | no: pointers and lengths by hand | a few ns less per call | not worth the fragile ABI |
| wasmi (interpreter) | yes | yes | no components | no compile time, execution ~10-50x slower than cranelift code | kept in mind for a platform without JIT |
| Lua (mlua) or another script | partial (strip the stdlib, hope) | yes | no | fast to write | not Rust; weak sandbox |
| native `cdylib` plugins | none | no: per OS and arch, unstable Rust ABI | yes | none | unsafe for players |
| an embedded JVM | none per mod | yes | Java | 100+ MB, seconds of startup | see the research |

wasmtime 36 is a long-term-support line (security fixes until August 2027)
that builds on the client's Rust 1.90. Only its `runtime`, `cranelift`,
`component-model` and `std` features are on: no WASI, no async, no GC
proposal, no profiling. Its cranelift backend generates native code for x86-64
and ARM64 on Windows, macOS and Linux, which is why one `.hmod` serves them
all.

### Performance

Measured on the Mac mini (M4 Pro), release build, `cargo test -p
horizon-mods --release --test sandbox -- --ignored --nocapture`:

| | per call |
|---|---|
| tick with no mod loaded | 1.2 ns (one check) |
| empty `tick` hook (call into the sandbox and back, fuel on) | 85.5 ns |
| empty `frame` hook | 87.5 ns |
| coords-hud's `tick` (reads the player and clock, formats 3 lines, compares) | 2.3 µs |
| loading coords-hud (read, compile with cranelift, instantiate, `init`) | 50 ms, once |

In the game, same world and view, `horizon-ctl bench 10` three times each
(4 other sessions shared the machine, so the tails are noisy; the median is
the stable figure):

| | median frame | mean FPS |
|---|---|---|
| no `horizon-mods` folder | 2.62-2.71 ms | 149-163 |
| coords-hud on | 2.72-2.78 ms | 151-254 |

At 20 ticks per second coords-hud costs 46 µs per second of game time: well
under 0.01 ms per frame. The frame-time medians differ by less than the
run-to-run spread.

**Budgets.** Each call gets fuel (about one unit per WebAssembly instruction):
2 billion for `init`, 50 million for `tick`, 10 million for `frame`, 100
million for a command or chat line. A runaway loop ends in a fraction of a
second (the sandbox test's endless `/spin` is cut well within its 5 s limit)
instead of hanging the game. The target budgets a mod should stay under are
tighter: **1 ms per tick and 0.1 ms per frame**. The runtime keeps each mod's
average call time (`Entry::cost`); the Mods page will show it, and a mod over
budget for long will be flagged, then paused.

### Crash isolation

A trap (panic, out-of-bounds access, `unreachable`), running out of fuel or
hitting the 64 MiB memory cap stops **that mod only**. The game says so in
chat (`Mod Test Guest stopped: it took too long (an endless loop?)`), marks it
on the Mods page and carries on; the other mods keep running. Host functions
clip what a mod hands back (chat lines per call, HUD lines and lengths). Tests:
`crates/horizon-mods/tests/sandbox.rs` (an endless loop, a panic and a 1 GiB
allocation, next to a healthy mod).

## Playing with mods

1. Put `.hmod` files (or mod folders holding `horizon-mod.toml` and
   `mod.wasm`) in `horizon-mods/` beside `options.txt` (in the launcher's game
   folder).
2. Options > **Mods** lists them with what each may do ("May see your
   position, health and the time, draw text on your screen, ...") and a
   switch. They start off; the choice is kept in `horizon-mods.txt`
   (`mod.<id>:true`).
3. A mod that failed to load or stopped shows why, in red, on its row.

Two turned-on mods never share an `id` while running (it is also capability
`storage`'s directory): the one `find` lists first starts, the other stays
off with the reason on its row.

Benchmarks (`--benchmark`) never load mods.

## Making a mod

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/ludovic111/horizon cargo-horizon-mod
cargo horizon-mod new my-mod        # Cargo.toml, horizon-mod.toml, src/lib.rs
cd my-mod
cargo horizon-mod run               # build, then start the game with it
cargo horizon-mod build             # again after an edit: the running game reloads it
```

`run` starts `horizon-client` (on `PATH`, or `--game PATH`, or
`HORIZON_CLIENT`) with `--dev-mod target/horizon-mod/<id>.hmod`. A dev mod is
always on, and the game checks its file once a second: rebuild and it restarts
with "Mods reloaded" in chat. Its in-memory state starts over; whatever it
saved with capability `storage` survives the reload.

### Template layout

```
my-mod/
├── Cargo.toml           # cdylib, horizon-mod-sdk, release: opt-level "s", LTO
├── horizon-mod.toml     # the manifest
└── src/lib.rs           # impl Mod, export_mod!
```

A mod is a type implementing `horizon_mod_sdk::Mod`; its state lives in the
type:

```rust
use horizon_mod_sdk::{Mod, Registration, chat};

struct Hello { greeted: u32 }

impl Mod for Hello {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Hello { greeted: 0 }, Registration::new().command("hello")))
    }
    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        self.greeted += 1;
        chat::show(&format!("Hello {args}! ({} so far)", self.greeted));
        Ok(None)
    }
}
horizon_mod_sdk::export_mod!(Hello);
```

Hooks not asked for keep their default bodies. The full example is
[`coords-hud`](../crates/horizon-mod-sdk/examples/coords-hud/src/lib.rs). Logic
that does not touch the API (formatting, maths) is ordinary Rust and unit-tests
on the host with `cargo test`.

### The manifest

```toml
id = "coords_hud"            # [a-z0-9_]{1,64}: settings key, log name
name = "Coordinates HUD"
version = "0.1.0"
description = "..."
authors = ["..."]
api = "0.1"                  # the API the SDK targets
side = "client"              # client | server | both
capabilities = ["game-read", "hud", "chat"]
worldgen_overrides = false   # a content mod that replaces vanilla's world generation
```

Unknown keys are refused, so a typo is an error, not a silently ignored
permission.

### Content mods (API 0.2)

A mod with `api = "0.2"` and capability `registries` may carry `assets/` and
`data/` in the `.hmod`, and needs no `mod.wasm` if it has no code. It adds
blocks and items by declaring them in `data/<id>/horizon/block/*.json` and
`data/<id>/horizon/item/*.json`, each copying a vanilla template
([modpacks.md](modpacks.md#registries-02-data-driven-blocks-and-items)):

```json
{ "template": "minecraft:iron_block",
  "properties": { "hardness": 4.0, "light": 3, "sounds": "minecraft:amethyst_block" },
  "item": true }
```

The fence: files go in the mod's own namespaces (`assets/<id>/`,
`data/<id>/`, and those its manifest lists), plus additions to vanilla tags
(`data/minecraft/tags/`) and to the convention tags NeoForge and Fabric mods
share (`data/c/tags/`).
Templates and `sounds` name vanilla blocks; unknown keys are refused.

Some Java mods ship their files under other names than their mod id (Towns
and Towers, mod id `t_and_t`, under `kaisyn` and `towns_and_towers`). A
content mod lists those in its manifest:

```toml
id = "t_and_t"
api = "0.2"
capabilities = ["registries"]
namespaces = ["kaisyn", "towns_and_towers"]
```

A listed namespace is the mod's as its id is: its `assets/` and `data/`
files load, and the game reads its tags, recipes, names and loot. Blocks and
items are still declared under `data/<id>/horizon/` only. `minecraft`, `c`
and `horizon` cannot be listed. Two mods that are on may not own the same
namespace (one's id or listed namespace being the other's): the game keeps
the first in load order and leaves the other's content out, with
"mods A and B both use the namespace N; turn one of them off" in the log. Paths
use vanilla's resource characters, at most 65,536 files, 16 MiB each and
512 MiB in all. A mod whose content is refused stays off, with the reason on
its row. The game reads this content into its block and item tables
(`horizon_client::mod_content`, docs/modpacks.md). A tag entry naming an id
nobody registers (a block a port left out, another mod's item) is dropped,
so the rest of the tag still loads; vanilla would drop the whole tag.

Tags merge as a data pack's do. Every mod's file for one tag (vanilla's
`data/minecraft/tags/`, the convention `data/c/tags/`, or another mod's) adds
its values after vanilla's and the mods' before it, in load order. A file with
`"replace": true` first drops what came before it, vanilla's entries and the
earlier mods', and the mods after it still add theirs, as a higher data pack
does. Since the result depends on load order, the game says so in the log:
"mod towns replaces data/minecraft/tags/worldgen/biome/has_structure/village_plains.json:
vanilla's entries and those of adds are left out, ...". Tags are the only
`minecraft` files a mod may hold: its other content stays in its own
namespaces.

A mod adds to structure sets and template pools that are not its own
(vanilla's villages, another mod's houses) with a set or pool file in its own
namespace holding `horizon:add_to`, a key vanilla data never has. Its entries
are appended to each set or pool named, after every mod's own files, in mod
and file order; the file is no set or pool itself. A name nobody defines is
skipped.

```json
{ "horizon:add_to": "minecraft:villages",
  "structures": [ { "structure": "mymod:village_beach", "weight": 10 } ] }
```

A pool addition holds `"elements"` as a pool does. This is what lithostitched's
`add_structure_set_entries` and `add_template_pool_elements` modifiers do on
NeoForge and Fabric, and `horizon-port-pack carry` writes them from those.

### World generation in a content mod

A content mod may add world generation of its own: biomes
(`data/<id>/worldgen/biome/<name>.json`, sub-folders included, so
`cave/deep_caves.json` is `<id>:cave/deep_caves`), noise settings, density
functions, noises, surface rules (material rules), carvers, block state
providers, configured and placed features, structures, structure sets, template
pools, processor lists and world presets. They join the bundles the world
generator reads (`worldgen/biomes.json`, `worldgen/densities.json` and the
rest), and the client knows the mod's biomes, so their colours, sounds and
spawns work.

A mod that *replaces* vanilla's world generation (Terralith, Tectonic) says so
in its manifest:

```toml
id = "terralith"
api = "0.2"
capabilities = ["registries"]
worldgen_overrides = true
```

Then, and only then, its files under `data/minecraft/worldgen/` and
`data/minecraft/dimension/` are read: they replace vanilla's of the same name
(the Overworld's noise settings, density functions, surface rules, biomes and
the biome distribution). The fence refuses them without the declaration, so a
mod cannot change the world's terrain by accident. The world preset's entry for
a dimension the mod replaces takes the mod's type and generator; the biome
distribution comes from the mod's
`data/minecraft/worldgen/multi_noise_biome_source_parameter_list/<name>.json`,
which the game reads as the quantized table the biome source uses.

**World types.** Such a mod does not change every world. Its replacements of
vanilla's world generation (the `minecraft:` entries above, the Overworld's
dimension, its dimension type and the biome distribution) are kept out of the
data every world reads, and the mod becomes a **world type**: Create World's
*World Type* button cycles Default, Superflat and then each mod's type, one at
a time, named by the mod's `name`. Default is plain vanilla whatever mods are
on. The choice is saved in the world's `world_gen_settings` as
`horizon:world_type` and holds for that world only (so does the mod's
dimension type, e.g. a taller world); a world whose mod is off does not open
and says which mod it needs. The mod's data in its own namespace (its biomes,
features, structures) is always loaded.

A mod may instead (or also) declare world types as data: a
`data/<namespace>/worldgen/world_preset/<name>.json` in its own namespace is a
world type `<namespace>:<name>` whose dimensions are read from that preset, as
a vanilla preset's. A mod with `worldgen_overrides` and no preset of its own
gets one type named after the mod (id: the mod's id) that applies its
replacements to the Overworld.

A world type's mod may also ship Lithostitched-style data: modifiers
`data/<ns>/lithostitched/worldgen_modifier/*.json` of type
`lithostitched:add_features` append placed features to a decoration step of the
biomes they name (kept in the type's own root when the mod has
`worldgen_overrides`); their `predicate` (`tectonic:config` + `key`) is read from
the mod's configuration. Placed features may use `lithostitched:condition`
(`lithostitched:sample_density`) and `lithostitched:noise_slope` placement
modifiers. `data/<ns>/worldgen_config.json`
(`{"constants": {...}, "noises": {"key": {"scale": .., "multiplier": .., "offset": ..}}}`)
gives the values `tectonic:config_constant`, `config_noise` and `config_clamp`
read, over the game's defaults.

The command line takes `--world-type <id>` for a world made with `--world`.
Vanilla's
tags are never replaced (a mod adds to them), so a mod that meant to narrow
`minecraft:is_forest` widens it instead.

A mod's density functions may also use the types Horizon reads for them:
Tectonic's `tectonic:config_constant` (a number the mod's configuration
holds), `tectonic:config_noise` (a noise the configuration scales) and
`tectonic:config_clamp` (a clamp to the limits of two functions), and the
surface condition `lithostitched:slope` (the height difference between the
column and its neighbours, on either axis, without a sign). They are read
only when a mod's data names them, so a world without such a mod is the
vanilla one, bit for bit.

A structure (and any of its pool elements, in place of the structure's) may
hold `horizon:enhanced_terrain_adaptation`: YUNG's API's and
MoogsStructureLib's terrain kernel, which carves or buries terrain around the
pieces on top of vanilla's `terrain_adaptation`. `kernel_size` and
`kernel_distance` shape the kernel; `top` and `bottom` (`carve`, `bury` or
`none`) act above and below a piece's floor; `bottom_offset`, `padding`
(`x`, `top`, `bottom`, `z`) and MoogsStructureLib's `band` (`bottom`, `top`,
`piece_heights`) move the box it measures from. On a structure,
`rigid_junctions` (YUNG's API) adds rigid pieces' junctions as well and
`inflate` grows the start's box by the kernel radius. `kernel_size` 0 is
none.

```json
{ "horizon:enhanced_terrain_adaptation": { "kernel_size": 24, "kernel_distance": 24,
    "top": "carve", "bottom": "none", "rigid_junctions": true, "inflate": true } }
```

## Versions and compatibility

- The API is the WIT package `horizon:mods@0.1.0`. New functions and
  interfaces arrive in patch versions (0.1.1, ...); a game serving 0.1.N loads
  mods built for 0.1.0 to 0.1.N. A breaking change makes `0.2` (and later
  `1.0`); the game then links both versions side by side for at least one
  release cycle, so existing mods keep working while authors move.
- The manifest's `api` names the line it needs (`0.1`); the game refuses a
  line it does not serve with a clear message.
- Horizon follows Minecraft releases; the mod API does not have to. Vanilla
  concepts mods see (dimensions, block and item ids) are the game's own ids,
  so a version port changes the data, not the API.

## Multiplayer

- **Client mods** (`side = "client"`, all of 0.1) run in the player's game,
  in single player and on any server, vanilla or Horizon. They see only what
  the client already knows (the snapshot), like F3 does, and act only on the
  local screen and chat. A server cannot see them.
- **Server mods** (`side = "server"`) will run where the world is simulated:
  Horizon's integrated server in single player and LAN, and the dedicated
  server. Vanilla clients can join if the mod only uses vanilla content
  (rules, commands, events).
- **Both** (`side = "both"`): a server mod with a client half (new blocks,
  their textures, a custom screen). On join the Horizon server announces its
  mods (id, version, SHA-256) on a `horizon:mods` custom payload channel; a
  Horizon client with matching client halves enables them for that session, or
  offers to download them from the same source the launcher uses. Vanilla
  clients are refused only if the server says its mods need a client.
- Registries (new blocks and items) are server-owned and synced like vanilla's
  registry sync, so ids match on both ends.

## Distribution

- **The file.** `.hmod`: a zip holding `horizon-mod.toml`, `mod.wasm` and,
  later, `assets/` and `data/`. `mod.wasm` is capped at 32 MiB.
- **Integrity.** Every download is checked against the SHA-256 the index
  lists before the file reaches `horizon-mods/`.
- **Signing.** Authors sign `.hmod` files with an Ed25519 key (minisign
  format: a `.hmod.minisig` beside it); the index pins each project's public
  key, so an update signed by another key is refused. Unsigned local files
  stay allowed, with an "unsigned" mark on the Mods page.
- **Where mods come from.** Modrinth is where players already look and it
  stores files with hashes, but its loader list is Modrinth's to extend: we ask
  Modrinth to add a `horizon` loader. Until then (and as a fallback), the
  launcher reads a small index of approved mods: a JSON file in a public Git
  repository (id, versions, file URLs as GitHub release assets, SHA-256,
  signing key). No server of our own is needed.
- **The launcher.** A Mods tab per installation: browse and search the index,
  install, update, turn on and off (it writes the same `horizon-mods.txt`),
  see capabilities before installing. The launcher's CLI and MCP tools gain
  `mods list|search|install|remove|enable|disable`, so agents can set up a
  modded game the same way players do.

## Roadmap

1. Design (this page) and the Java research.
2. SDK, example mod, runtime, `cargo horizon-mod`, the game's Mods page: API 0.1.
3. `storage` (done), `keys`, `target`, `inventory-read`, `ui`: enough for the
   first native ports (zoom presets, AppleSkin-style food HUD, Jade-style
   tooltips).
4. Launcher Mods tab, the index, signatures; the Mods page shows cost and
   update state.
5. `recipes`, `world-read`, `render`: recipe viewer and schematics ports.
6. Server mods, `registries`, `net`, data-driven content from `.hmod`
   `data/` and `assets/`, multiplayer sync.

Modpacks ([modpacks.md](modpacks.md)) move data-driven `registries` first:
API 0.2 is content-only `.hmod`s (blocks, items, recipes, loot, models and
textures), then server hooks (0.3) and block entities, menus, capabilities and
`net` (0.4), in the order the ATM10 inventory needs them.
