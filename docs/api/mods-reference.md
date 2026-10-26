<!-- Generated from crates/horizon-mod-sdk/wit/horizon.wit and the manifest's capabilities by crates/horizon-mods/tests/api_reference.rs; a test fails when this file is out of date. Update it with: HORIZON_WRITE_DOCS=1 cargo test -p horizon-mods --features pack --test api_reference -->
# Mod API reference

The contract between Horizon and its mods: the WIT package `horizon:mods@0.1.34`, every interface, type, function and hook in it, and the capabilities a manifest may list. The guide is [mods.md](../mods.md); stability and versions: [stability.md](stability.md). Manifest `api` lines this game serves: `0.1`, `0.2`.

## Capabilities

| In `capabilities` | Grants | The player reads "May …" |
| --- | --- | --- |
| `game-read` | [`game`](#game) | see your position, health and the time |
| `chat` | [`chat`](#chat) | read the chat and show messages of your own |
| `hud` | [`hud`](#hud) | draw text on your screen |
| `target` | [`target`](#target) | see the block you look at and the one you break |
| `inventory-read` | [`inventory`](#inventory) | see your hotbar |
| `hotbar` | [`hotbar`](#hotbar) | change your selected hotbar slot |
| `world-read` | [`world`](#world) | see the blocks around you and the ones that break |
| `world-edit` | [`world`](#world) | break blocks and wear the item in your hand |
| `keys` | [`keys`](#keys) | add keys you can press |
| `storage` | [`storage`](#storage) | save its own settings to disk |
| `screen` | [`screen`](#screen) | draw a screen of its own |
| `items-read` | [`items`](#items) | see the items of the game |
| `recipes-read` | [`recipes`](#recipes) | see the recipes of the game |
| `inventory-edit` | [`inventory`](#inventory) | give you items |
| `weather-read` | [`weather`](#weather) | see the weather |
| `settings-read` | [`settings`](#settings) | read your settings (particles) |
| `particles` | [`particles`](#particles) | add particles of its own |
| `atmosphere` | [`atmosphere`](#atmosphere) | change the haze in the fog |
| `overlay` | [`overlay`](#overlay) | draw over the world on your HUD |
| `entity-read` | [`entity`](#entity) | see the mobs, items and players around you |
| `entity-edit` | [`entity`](#entity) | remove entities and change experience orbs |
| `slots-read` | [`slots`](#slots) | see which slot your cursor is over |
| `slot-decorate` | [`slots`](#slots) | draw marks on your slots |
| `tooltip` | [`tooltip`](#tooltip) | add to the tooltips of items |
| `debug` | [`debug`](#debug) | add lines to the debug screen |
| `lighting` | [`lighting`](#lighting) | add light to the world |
| `resource-read` | [`resources`](#resources) | read the files of your resource packs |
| `chat-decorate` | [`chat-decorate`](#chat-decorate) | draw pictures in your chat |
| `players` | [`players`](#players) | see who is playing |
| `light-events` | [`light-events`](#light-events) | see the beams and sparks that shine in the world |
| `audio` | [`audio`](#audio) | play sounds |
| `exchange` | [`exchange`](#exchange) | share data with your other mods |
| `input` | [`input`](#input) | see the mouse wheel and buttons while you play |
| `game-edit` | [`game`](#game) | change your hunger and your mining speed |
| `texture-rules` | [`texture-rules`](#texture-rules) | change the textures of blocks |
| `command` | [`command`](#command) | run commands as you, with your permissions |
| `registries` | data files (`assets/`, `data/`), no interface | add blocks and items to your worlds |
| `ui` | [`ui`](#ui) | show its own screens and a settings page |
| `render` | [`render`](#render) | draw on your screen, shake and zoom the camera |
| `controls` | [`controls`](#controls) | see the keys you hold and your clicks, take over the use button on its own items and the mouse wheel, and add keys |

## World `horizon-mod`

- Imports (what the game offers): `log`, `game`, `chat`, `hud`, `target`, `inventory`, `hotbar`, `keys`, `storage`, `screen`, `items`, `recipes`, `weather`, `settings`, `particles`, `atmosphere`, `overlay`, `entity`, `slots`, `tooltip`, `debug`, `lighting`, `chat-decorate`, `players`, `light-events`, `audio`, `input`, `exchange`, `resources`, `texture-rules`, `world`, `command`
- Exports (what the mod provides): `events`

## World `horizon-ui-mod`

A mod with widgets (capability `ui`): the SDK builds every mod against this world. Mods built for `horizon-mod` alone (no `ui-events` export) still load and simply hear nothing of their widgets.

- Imports (what the game offers): `ui`, `render`, `controls`, `log`, `game`, `chat`, `hud`, `target`, `inventory`, `hotbar`, `keys`, `storage`, `screen`, `items`, `recipes`, `weather`, `settings`, `particles`, `atmosphere`, `overlay`, `entity`, `slots`, `tooltip`, `debug`, `lighting`, `chat-decorate`, `players`, `light-events`, `audio`, `input`, `exchange`, `resources`, `texture-rules`, `world`, `command`
- Exports (what the mod provides): `ui-events`, `events`

## World `ui-host`

The game's side of `horizon-ui-mod` beyond `horizon-mod`.

- Imports (what the game offers): `ui`
- Exports (what the mod provides): `ui-events`

## World `render-host`

The game's side of capability `render` (imports only).

- Imports (what the game offers): `render`, `controls`
- Exports (what the mod provides): 

## `log`

Access: always granted.

The mod's log, in the game's log under the mod's id. Always granted.

### enum `level`

| Case | |
| --- | --- |
| `debug` |  |
| `info` |  |
| `warn` |  |
| `error` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `write` | `func(level: level, message: string)` | One line in the game's log as `[mod <id>] message`, at that level. |

## `game`

Access: capability `game-read`.

Capability `game-read`: the local player and the world clock.

### record `input-state`

What the player's keys do now (API 0.1.16).

| Field | Type | |
| --- | --- | --- |
| `sneaking` | `bool` | The player is sneaking. |
| `attacking` | `bool` | The attack button (the left mouse button) is held. |

### record `player`

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` | Feet position, in blocks. |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `yaw` | `f32` | Degrees, as F3 shows them. |
| `pitch` | `f32` | Degrees: -90 looks straight up, 90 straight down. |
| `health` | `f32` | Hit points as vanilla counts them: 20 is full health without effects. |
| `food` | `u32` | 0 to 20, as the hunger bar. |
| `dimension` | `string` | `minecraft:overworld`, `minecraft:the_nether`, ... |

### record `clock`

| Field | Type | |
| --- | --- | --- |
| `game-time` | `u64` | Ticks since the world was created. |
| `day-time` | `u64` | 24000 ticks per day; 0 is 6:00 on the first day. |

### record `food-stats`

Hunger as `FoodData` keeps it (API 0.1.2).

| Field | Type | |
| --- | --- | --- |
| `food` | `u32` | 0 to 20, as `player.food`. |
| `saturation` | `f32` | 0 to `food`: what is spent before hunger drops. |
| `exhaustion` | `f32` | 0 to 40; every 4 takes one saturation (or food) point. |

### record `viewport`

The window as the game draws it (API 0.1.32).

| Field | Type | |
| --- | --- | --- |
| `gui-scale` | `f32` | Screen pixels in one GUI pixel (4.0 at GUI scale 4). |
| `width` | `f32` | The window's width in screen pixels. |
| `height` | `f32` | The window's height in screen pixels. |

### record `session`

The world the player is in (API 0.1.10).

| Field | Type | |
| --- | --- | --- |
| `id` | `u64` | A number that changes with every world loaded and every server joined. |
| `elapsed-ms` | `u64` | The milliseconds since that world was loaded. |

### record `health`

The player's health (API 0.1.11).

| Field | Type | |
| --- | --- | --- |
| `health` | `f32` |  |
| `max-health` | `f32` | The most health the player can have. |
| `absorption` | `f32` | The absorption (the golden hearts). |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-player` | `func() -> option<player>` | None on the title screen. |
| `get-input-state` | `func() -> option<input-state>` | The player's sneaking and attack button; none outside a world (API 0.1.16). |
| `get-clock` | `func() -> option<clock>` | None on the title screen. |
| `get-food-stats` | `func() -> option<food-stats>` | None on the title screen (API 0.1.2). |
| `get-game-mode` | `func() -> option<string>` | `game-read` (API 0.1.24): the player's game mode: `survival`, `creative`, `adventure` or `spectator`; none outside a world. |
| `get-player-tags` | `func() -> list<string>` | `game-read` (API 0.1.24): the player's tags, the ones `/tag` sets, as of the last tick; empty outside a world. |
| `get-cloud-height` | `func() -> option<s32>` | `game-read` (API 0.1.25): the height of the clouds in the world the player is in, in blocks; none outside a world and in a dimension with no clouds. |
| `get-viewport` | `func() -> option<viewport>` | `game-read` (API 0.1.32): the window's size and GUI scale; none when the game has not drawn a frame yet. |
| `get-server-view-distance` | `func() -> u32` | `game-read` (API 0.1.32): the view distance, in chunks, of the world the player is in when this game runs it (its own or a LAN host's: the render distance); 0 when it is not known (another's server). |
| `add-exhaustion` | `func(amount: f32)` | `game-edit` (API 0.1.23): adds exhaustion to the player, as `Player.causeFoodExhaustion` does (4 makes one hunger point go); 0 or less does nothing, at most 40 a call. Made after the hook, in a world this game runs. |
| `set-break-speed-modifier` | `func(multiplier: f64)` | `game-edit` (API 0.1.23): the mod's modifier of the player's mining speed, added to the base as `Attributes.BLOCK_BREAK_SPEED`'s `ADD_MULTIPLIED_BASE` does: speed times `1 + sum of the mods' modifiers`, at least 0. 0 is no change; it replaces the mod's earlier one and goes with the mod. |
| `get-seed` | `func() -> option<u64>` | The world's seed; none when the game does not know it, as in another's world (API 0.1.7). |
| `get-fov` | `func() -> option<f32>` | The field of view the game drew the last frame with, in degrees, the vertical one; none outside a world (API 0.1.7). |
| `get-session` | `func() -> option<session>` | The world the player is in; none outside a world (API 0.1.10). |
| `get-health` | `func() -> option<health>` | The player's health; none outside a world (API 0.1.11). |
| `get-effects` | `func() -> list<string>` | The ids of the effects on the player (`minecraft:poison`, `minecraft:regeneration`, ...); empty when there is none (API 0.1.11). |
| `get-difficulty` | `func() -> option<string>` | The world's difficulty: `peaceful`, `easy`, `normal` or `hard`; none outside a world (API 0.1.11). |
| `get-main-hand` | `func() -> string` | The hand the player holds the main item in, `left` or `right` (the Main Hand option; API 0.1.34; `game-read`). |

## `chat`

Access: capability `chat`.

Capability `chat`: messages only the local player sees.

### record `sender`

The player who wrote a chat message (API 0.1.15).

| Field | Type | |
| --- | --- | --- |
| `name` | `string` | Their name; empty when the game does not know it (they are not in the player list). |
| `uuid` | `string` | Their UUID as 32 hexadecimal characters, no hyphens. |

### record `message`

A chat message the player received (API 0.1.15).

| Field | Type | |
| --- | --- | --- |
| `id` | `u64` | The message's number in this session: grows, never repeats. |
| `text` | `string` | Its text, without formatting, as the `chat` hook gets it. |
| `sender` | `option<sender>` | Who wrote it; none for a system message, `/say` and a message whose sender the server hid. |

| Function | Signature | What it does |
| --- | --- | --- |
| `show` | `func(text: string)` | Shown in chat after the mod's name, like a system message. |
| `get-messages` | `func() -> list<message>` | The messages received since the last tick, oldest first, the same for every mod. The game adds a message to this list *before* it calls the `chat` hook, so a mod that reads it in the hook finds that message last (API 0.1.15). |

## `chat-decorate`

Access: capability `chat-decorate`.

Capability `chat-decorate`: a picture before a chat message and the room the game leaves for it (API 0.1.15). The mod says once which picture a message gets; the game draws it and indents the text, so no mod code runs when the chat is drawn.

### variant `source`

Where a picture comes from.

| Case | Payload | |
| --- | --- | --- |
| `sender-skin` |  | The skin of the player who wrote the message (64 by 64): the one the player list has downloaded, else the default skin for their UUID. |
| `player-skin` | `string` | The skin of the player with this UUID (32 hexadecimal characters), 64 by 64. |
| `sprite` | `string` | A sprite of the game's GUI atlas by name (`hud/heart/full`). |

### record `region`

A rectangle of the source, in pixels.

| Field | Type | |
| --- | --- | --- |
| `x` | `u32` |  |
| `y` | `u32` |  |
| `width` | `u32` |  |
| `height` | `u32` |  |

### record `picture`

| Field | Type | |
| --- | --- | --- |
| `source` | `source` |  |
| `regions` | `list<region>` | The rectangles to draw, one over the other in the same place (a face and its hat); empty draws the sprite whole. At most 8. |
| `width` | `u32` | The picture's size on screen, in GUI pixels (1 to 64). |
| `height` | `u32` |  |

### variant `place`

Where a picture stands (API 0.1.18).

| Case | Payload | |
| --- | --- | --- |
| `line-start` |  | At the start of the first line; the text of every line of the message is indented by the picture's width and the gap. |
| `text` | `u32` | Before this character of the message's text (counted from 0 in the text `chat.get-messages` gives): the first line is cut there, the picture stands between the parts and the rest of the line follows it. A number past the first line puts the picture at its end. |

### record `style`

How one rectangle of a picture is drawn (API 0.1.18).

| Field | Type | |
| --- | --- | --- |
| `offset-x` | `s32` | Offset on the screen, in GUI pixels, from the picture's place. |
| `offset-y` | `s32` |  |
| `width` | `u32` | Size on the screen, in GUI pixels; 0 is the picture's own size. |
| `height` | `u32` |  |
| `color` | `u32` | 0xRRGGBB multiplied with the texture (a shadow is a dark one); 0 is the line's colour. The line's fading applies to it. |

### record `decoration`

A picture with all of how it is drawn (API 0.1.18).

| Field | Type | |
| --- | --- | --- |
| `picture` | `picture` |  |
| `gap` | `option<u32>` | The room left after the picture, in GUI pixels; none is 2, as `set-picture` leaves it, and 0 lets the text touch it. |
| `place` | `place` |  |
| `styles` | `list<style>` | How to draw each rectangle of `picture`, by index, in order (a shadow is the first rectangle again, offset and dark); missing entries draw a rectangle as it is. |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-picture` | `func(id: u64, picture: option<picture>)` | The picture before the message with this id, or none for no picture. Set it in the `chat` hook, before the game adds the message to the chat; the picture then starts the message's first line and the text of all its lines is indented by `width + 2` pixels. The first mod to give a picture for a message wins. |
| `set-decoration` | `func(id: u64, decoration: option<decoration>)` | The picture of the message with this id, with all of how it is drawn, or none for no picture; `set-picture` is this with a gap of 2, the start of the line and no styles. Skin pictures of a player who hangs upside down are drawn upside down (API 0.1.18). |

## `players`

Access: capability `players`.

Capability `players`: the players of the world, from the player list (API 0.1.18).

### record `player`

| Field | Type | |
| --- | --- | --- |
| `name` | `string` | Their profile name. |
| `display-name` | `string` | The name the player list shows (a team's prefix, a nickname); empty when it is the profile name. |
| `uuid` | `string` | Their UUID as 32 hexadecimal characters, no hyphens. |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-players` | `func() -> list<player>` | The players in the player list; empty outside a world. |

## `light-events`

Access: capability `light-events`.

Capability `light-events` (API 0.1.20): things that shine in the world without being an entity, as data the mod lights as it likes.

### record `flash`

| Field | Type | |
| --- | --- | --- |
| `kind` | `string` | What it is: `minecraft:beacon_beam`, `minecraft:guardian_beam`, `minecraft:firefly`, `minecraft:sonic_boom`. |
| `x` | `f64` | Where it is: a beam's start, a particle's position. |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `end-x` | `f64` | A beam's end (the top of a beacon's beam, a guardian's target); equal to the start for a particle. |
| `end-y` | `f64` |  |
| `end-z` | `f64` |  |
| `ticks` | `u32` | How long it shines from now, in ticks (a standing beam: 2, to be reported again while it stands). |

| Function | Signature | What it does |
| --- | --- | --- |
| `poll` | `func() -> list<flash>` | The flashes since the last tick of the mod: the beams that stand within 128 blocks of the player and the particles spawned since. Empty when nothing shines. At most 256. |

## `input`

Access: capability `input`.

Capability `input` (API 0.1.23): the mouse while the player plays, with no screen open.

### record `mouse-event`

| Field | Type | |
| --- | --- | --- |
| `wheel` | `f32` | The wheel: positive up, negative down; 0 for a button. |
| `button` | `s32` | 0 left, 1 right, 2 middle; -1 for the wheel. |
| `pressed` | `bool` | The button went down (false: up). Always false for the wheel. |
| `x` | `f32` | Where the cursor is, in GUI pixels. |
| `y` | `f32` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `poll` | `func() -> list<mouse-event>` | The events since the last call, oldest first (at most 64 are kept). |
| `set-wheel-captured` | `func(captured: bool)` | Whether the game leaves the wheel to the mod: while true the wheel does not change the hotbar slot (the roulette of a mod). Stays until the mod changes it, and goes with the mod. |

## `audio`

Access: capability `audio`.

Capability `audio` (API 0.1.22): sounds of the game the mod plays.

### record `sound-place`

Where a sound is heard from, in the world.

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` |  |
| `y` | `f64` |  |
| `z` | `f64` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `play` | `func(sound: string, place: option<sound-place>, volume: f32, pitch: f32) -> bool` | Plays a sound of the game (`minecraft:weather.rain`, `minecraft:item.snowball.throw`) after the hook; at the place when there is one, else as if from the player. The volume is 0 to 4, the pitch 0.5 to 2. False for a name that is not `namespace:path`, a sound the game does not have, or more than 16 sounds in one hook. |
| `register` | `func(name: string, ogg: list<u8>) -> result<_, string>` | Adds the mod's own sound, an OGG Vorbis file, to the game (API 0.1.25) and names it `<mod id>:<name>` to `play` (a name is `[a-z0-9_.-]`, up to 64 characters). The file goes to the mod's own folder, so it is there after a restart; at most 16 sounds of 2 MiB each. Since API 0.1.29 the bytes may also be a PCM WAV file (16-bit, any rate; a mod can write one itself, no encoder needed). An error for a file that is neither or does not decode. |
| `register-file` | `func(name: string) -> result<_, string>` | Adds the sound file `sounds/<name>.ogg` (or, since API 0.1.29, `sounds/<name>.wav`, PCM) that the mod carries in its own `.hmod` (API 0.1.26; `cargo-horizon-mod build` packs the `sounds/` folder of the crate) under `<mod id>:<name>`, as `register` does. An error for a name the mod has no file for, or a file that is not OGG or does not decode. |

## `hud`

Access: capability `hud`.

Capability `hud`: one panel of text lines per mod, drawn by the game every frame until the mod changes or clears it (no call per frame).

### enum `corner`

| Case | |
| --- | --- |
| `top-left` |  |
| `top-right` |  |
| `bottom-left` |  |
| `bottom-right` |  |

### record `line`

| Field | Type | |
| --- | --- | --- |
| `text` | `string` | Plain text, up to 256 characters. |
| `color` | `u32` | 0xRRGGBB. |

### record `bar-icon`

One icon of a status bar as the game drew it (API 0.1.7).

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` | The top left corner in GUI pixels, after the shaking of the bar. |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `name` | `string` | The sprite the game drew (`hud/food_empty`, `hud/heart/container`, ...). |

### record `bars`

The health and hunger bars as the game drew them in the last frame.

| Field | Type | |
| --- | --- | --- |
| `food` | `list<bar-icon>` | The container of each hunger icon, in the order the game drew them. |
| `health` | `list<bar-icon>` | The container of each heart, in the order the game drew them. |

### record `boss-bar`

A boss bar the game draws at the top of the screen (API 0.1.34).

| Field | Type | |
| --- | --- | --- |
| `name` | `string` | The name above the bar, without formatting. |
| `y-bottom` | `f32` | Where the bar ends, in GUI pixels from the top of the screen (the bar's 5 pixels; the next starts 19 below the one before). |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-panel` | `func(corner: corner, lines: list<line>)` | Replaces the mod's panel: up to 16 lines in that corner of the screen. |
| `clear` | `func()` | Removes the mod's panel. |
| `show-action-bar` | `func(text: string, color: u32)` | Shows a line over the hotbar, as vanilla's action bar does (API 0.1.16); it fades after about three seconds, and a new one replaces it. `color` is 0xRRGGBB; the text may have `§` formatting. An empty text does nothing. |
| `get-bars` | `func() -> option<bars>` | Where the bars are; none when the game draws none (creative, spectator, the HUD hidden). One frame old (API 0.1.7). |
| `get-boss-bars` | `func() -> list<boss-bar>` | The boss bars the game draws now, from the top; empty when there is none (API 0.1.34; `hud`). A panel under them starts below the last. |

## `target`

Access: capability `target`.

Capability `target`: the block the player looks at, and the one they are breaking (API 0.1.1).

### record `block`

| Field | Type | |
| --- | --- | --- |
| `x` | `s32` |  |
| `y` | `s32` |  |
| `z` | `s32` |  |
| `id` | `string` | `minecraft:stone`, ... |
| `hardness` | `f32` | Vanilla's destroy time; -1 for a block that cannot be broken. |
| `requires-tool` | `bool` | Drops only when broken with the correct tool. |
| `tags` | `list<string>` | Every block tag it is in: `minecraft:mineable/pickaxe`, `minecraft:needs_iron_tool`, `minecraft:leaves`, ... |

### record `entity`

A living entity under the crosshair (API 0.1.2).

| Field | Type | |
| --- | --- | --- |
| `id` | `u32` | Its id in the world this tick. |
| `type-id` | `string` | `minecraft:zombie`, ... |
| `name` | `string` | Its custom name, else its type's name in the game's language. |
| `health` | `f32` |  |
| `max-health` | `f32` |  |

### record `effect`

A status effect on a mob (API 0.1.34).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | The effect's id, e.g. `minecraft:speed`. |
| `amplifier` | `u32` | 0 for level I. |
| `duration-ticks` | `s32` | The ticks left; -1 when the effect lasts for ever. |
| `ambient` | `bool` | Given by a beacon or a conduit: its particles are faint. |
| `visible` | `bool` | Shown in the game's own effect icons and particles. |

### record `fluid`

A fluid under the crosshair (API 0.1.34).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:water` or `minecraft:lava`. |
| `name` | `string` | The name in the game's language. |
| `level` | `u32` | The block state's `level`: 0 for a source, 1 to 7 for flowing fluid (more is lower), 8 and up for falling fluid. |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-block` | `func() -> option<block>` | The block under the crosshair within reach; none on air, an entity or the title screen. |
| `get-breaking` | `func() -> option<block>` | The block the player is breaking (the attack button held on it in survival or adventure), from its first tick to its last. |
| `get-breaking-progress` | `func() -> option<f32>` | How far that block is broken, 0 to 1; none when none is (API 0.1.2). |
| `get-block-name` | `func() -> option<string>` | The looked-at block's name in the game's language (`Block.getName`), its id when the language has none; none as `get-block` (API 0.1.2). |
| `get-entity` | `func() -> option<entity>` | The mob the attack button would hit (`EntityHitResult`) within reach, before any block; none otherwise (API 0.1.2). |
| `get-entity-armor` | `func() -> option<f32>` | The armour points of the mob under the crosshair (`LivingEntity.getArmorValue`: its type's, not what it wears); none as `get-entity` (API 0.1.9). |
| `get-entity-absorption` | `func() -> option<f32>` | The absorption of the mob under the crosshair; none as `get-entity`. The game does not track a mob's absorption: 0 (API 0.1.9). |
| `get-block-face` | `func() -> option<string>` | The face of the block under the crosshair that the player looks at: `north`, `south`, `east`, `west`, `up` or `down`; none as `get-block` (API 0.1.9). |
| `get-entity-effects` | `func() -> option<list<effect>>` | The effects on the mob under the crosshair; none as `get-entity` (API 0.1.34; `target`). The game knows the effects of the mobs it simulates or is told of: a mob it was not told of has none. |
| `get-block-ex` | `func(reach: f32, from-eye: bool) -> option<block>` | The block under a ray of `reach` blocks (0 to 32; the game's own reach is 4.5, 5 in creative) from the player's eyes, or, with `from-eye` false, from the camera and along its view (the third-person perspectives); none as `get-block` (API 0.1.34). The first block along the ray, as `get-block` finds it. |
| `get-block-face-ex` | `func(reach: f32, from-eye: bool) -> option<string>` | The face of that block, as `get-block-face` (API 0.1.34). |
| `get-entity-ex` | `func(reach: f32, from-eye: bool) -> option<entity>` | The mob under that ray, nearer than the block it hits (API 0.1.34). |
| `get-fluid` | `func(mode: string) -> option<fluid>` | The fluid the crosshair is on, within the game's reach (API 0.1.34). `mode`: `any` gives the first fluid on the ray, even before a block; `fallback` gives it only when the ray hits no block; `none` (and any other word) gives none, as a game that ignores fluids does. |

## `inventory`

Access: capability `inventory-read`.

Capability `inventory-read`: the player's hotbar (API 0.1.1).

### record `stack`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:iron_pickaxe`, ... |
| `count` | `u32` |  |
| `damage` | `u32` |  |
| `max-damage` | `u32` | 0 for an item that does not wear out. |
| `efficiency` | `u32` | Its Efficiency level, 0 without. |
| `mining-speed` | `f32` | How fast it breaks the targeted block (the one being broken, else the one looked at): `Tool.getMiningSpeed` plus Efficiency's `mining_efficiency`, as `Player.getDestroySpeed` adds it. 1 for a hand, or with no block targeted. |
| `correct-for-drops` | `bool` | `Tool.isCorrectForDrops` for that block. |

### record `food`

What eating a stack gives: its `minecraft:food` component (API 0.1.2).

| Field | Type | |
| --- | --- | --- |
| `nutrition` | `u32` |  |
| `saturation` | `f32` | Added to saturation, as `FoodData.eat` adds it. |

### record `item-stack`

A stack with everything that tells it apart from another one (API 0.1.10).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:iron_pickaxe`, ... |
| `count` | `u32` |  |
| `damage` | `u32` | The durability used; 0 for an item that does not wear out. |
| `components` | `string` | The item's other components (enchantments, a custom name, a potion's contents, a trim, a container's items): opaque text, the same for stacks that are the same thing, empty for a plain stack. |

### record `player-inventory`

The player's whole inventory.

| Field | Type | |
| --- | --- | --- |
| `slots` | `list<option<item-stack>>` | The 36 slots: the hotbar (0 to 8) and the rest (9 to 35). |
| `armour` | `list<option<item-stack>>` | The armour: head, chest, legs, feet. |
| `offhand` | `option<item-stack>` | The off hand. |
| `carried` | `option<item-stack>` | The stack on the cursor. |

### record `crafting`

A crafting grid and what it makes.

| Field | Type | |
| --- | --- | --- |
| `grid` | `list<option<item-stack>>` | The grid's cells in row order: 4 for the player's 2x2, 9 for a crafting table's 3x3. |
| `output` | `option<item-stack>` | What the grid makes; none when it makes nothing. |

### record `tool`

An item of the hotbar as a tool (API 0.1.16).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:iron_pickaxe`, ... |
| `has-tool` | `bool` | Always true: only items with a `minecraft:tool` component are listed (`ItemStack.has(DataComponents.TOOL)`). |
| `damage-per-block` | `u32` | `Tool.damagePerBlock`: the wear one block it breaks gives it. |
| `enchantments` | `list<tuple<string, u32>>` | Its enchantments: id (`minecraft:efficiency`) and level. |

### record `slot`

A slot of the player's inventory (API 0.1.16).

| Field | Type | |
| --- | --- | --- |
| `index` | `u32` | 0 to 8 the hotbar, 9 to 35 the rest, 36 to 39 the armour (feet, legs, chest, head), 40 the off hand. |
| `id` | `string` | The item; an empty string for an empty slot. |
| `count` | `u32` |  |
| `damage` | `u32` | The durability used. |
| `max-damage` | `u32` | 0 for an item that does not wear out. |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-hotbar` | `func() -> list<option<stack>>` | Slots 0 to 8; none for an empty slot. |
| `get-selected` | `func() -> u32` | The selected slot, 0 to 8. |
| `get-food` | `func(slot: u32) -> option<food>` | The food in hotbar slot 0 to 8; none for an empty slot, an item that is not food or another slot (API 0.1.2). |
| `item-tags` | `func(id: string) -> list<string>` | Every item tag `id` is in, as full names without `#`: `minecraft:axes`, `minecraft:pickaxes`, `minecraft:swords`, ... Nested tags are already expanded, as vanilla's `TagLoader` does. Empty for an unknown id (API 0.1.3). |
| `get-tags` | `func(slot: u8) -> list<string>` | Every item tag the item in hotbar slot 0 to 8 is in (API 0.1.3); empty for an empty slot, another slot or an unknown item. |
| `get-inventory` | `func() -> option<player-inventory>` | The player's whole inventory; none outside a world (API 0.1.10). |
| `get-crafting` | `func() -> option<crafting>` | The crafting grid and its result while the player's inventory or a crafting table is open; none otherwise (API 0.1.10). |
| `give` | `func(id: string, count: u32) -> bool` | Capability `inventory-edit` (API 0.1.5): gives the player `count` of the item `id`, as the creative inventory does: the stack goes into the inventory, and what does not fit is dropped. False (and nothing happens) without the capability, for an unknown id, a count of 0, when the player is not in creative, or in a world another game runs. |
| `get-food-of` | `func(id: string) -> option<food>` | What eating the item with that id gives (`minecraft:cooked_beef`, ...); none when it is not food (API 0.1.11). |
| `get-harmful-food` | `func(id: string) -> bool` | Whether eating the item can hurt: one of its effects is Hunger (rotten flesh, a pufferfish, raw chicken); the game draws such food with the "hunger" icons (API 0.1.11). |
| `get-offhand` | `func() -> option<stack>` | The stack in the off hand; none when it is empty (API 0.1.11). |
| `get-tools` | `func() -> list<option<tool>>` | The tool in each of the hotbar's slots 0 to 8 (the same indices as `get-hotbar`); none for an empty slot and for an item without a `minecraft:tool` component (a block, food, ...) (API 0.1.16). |
| `get-slots` | `func() -> list<slot>` | All 41 slots of the player's inventory, in the order of their index; empty outside a world (API 0.1.16). |
| `move-item` | `func(source-slot: u32, target-slot: u32) -> bool` | Capability `inventory-edit` (API 0.1.16): moves the item of slot `source-slot` to slot `target-slot` (hotbar, the rest of the inventory, 9 to 35, or the off hand, 40): into an empty slot, or swapping with what is there. True when the game will do it before its next tick; false without the capability, for a slot that is out of range, an armour slot, an empty source slot, or in a world another game runs. |

## `hotbar`

Access: capability `hotbar`.

Capability `hotbar`: changing the selected slot, as the number keys do (API 0.1.1).

| Function | Signature | What it does |
| --- | --- | --- |
| `select` | `func(slot: u32)` | Selects slot 0 to 8 before the game's next tick; other values are ignored. The last call in a tick wins. |

## `keys`

Access: capability `keys`.

Capability `keys`: the mod's own key bindings, shown in Controls (API 0.1.4). The mod names its actions; the player rebinds them.

### record `key-event`

A key event since the last `poll`.

| Field | Type | |
| --- | --- | --- |
| `action` | `string` | The mod's action (`map`, `waypoint`, ...), as it was bound. |
| `pressed` | `bool` | Pressed (true) or released (false). |
| `repeat` | `bool` | The key repeats itself while it is held. |

| Function | Signature | What it does |
| --- | --- | --- |
| `bind` | `func(action: string, default-key: string) -> result<_, string>` | Binds the mod's `action` to a key (`key.keyboard.m`); the player may rebind it in Controls. Binding an action twice replaces the old key. |
| `get` | `func(action: string) -> option<string>` | The key bound to the mod's `action` now (`key.keyboard.m`); none when the action is not bound or unbound by the player. |
| `poll` | `func() -> list<key-event>` | The key events since the last call; empty when nothing happened. |
| `is-down` | `func(key: string) -> bool` | Whether the key is held down now (`key.keyboard.left.shift`, ...): the names are the game's, as in `bind` and `options.txt`. Cheap enough to ask every frame (API 0.1.11). |

## `storage`

Access: capability `storage`.

Capability `storage`: a per-mod folder of small key/value files, quota'd (docs/mods.md). Keys are `[a-z0-9_.-]{1,64}`, one path segment. The folder is `.horizon-mod-data/<id>` in the folder the mod is loaded from; a mod that needs more than the default 4 MiB / 64 keys asks for it in its manifest (`storage_quota_mib`, `storage_max_keys`). It is there in `init`.

| Function | Signature | What it does |
| --- | --- | --- |
| `get` | `func(key: string) -> option<list<u8>>` | None if `key` has never been `set`, or the key is not well-formed. |
| `set` | `func(key: string, value: list<u8>) -> result<_, string>` | Errs on a malformed key or over quota (bytes for the whole mod). |
| `delete` | `func(key: string)` | No error if `key` was already absent. |
| `list-keys` | `func() -> list<string>` | The mod's keys, sorted (API 0.1.5). |
| `read` | `func(name: string) -> option<list<u8>>` | Deprecated, API 0.1.4's name of `get`. |
| `write` | `func(name: string, data: list<u8>) -> result<_, string>` | Deprecated, API 0.1.4's name of `set`. |
| `remove` | `func(name: string) -> bool` | Deprecated, API 0.1.4's name of `delete`; true when it was there. |
| `files` | `func() -> list<string>` | Deprecated, API 0.1.4's name of `list-keys`. |

## `screen`

Access: capability `screen`.

Capability `screen`: a screen the mod draws and handles itself (API 0.1.4). The game draws it over the world, without the mouse grabbed, and gives it the player's input while it is open.

### record `size`

The screen's size in GUI pixels.

| Field | Type | |
| --- | --- | --- |
| `width` | `f32` |  |
| `height` | `f32` |  |

### record `rect-shape`

A filled rectangle.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `color` | `u32` | 0xRRGGBB. |

### record `text-shape`

A line of text; (x, y) is its top left corner.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `text` | `string` |  |
| `color` | `u32` | 0xRRGGBB. |
| `shadow` | `bool` | The text's shadow. |

### record `image-shape`

An uploaded image, scaled into the rectangle.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `texture` | `u32` | The image's id, from `upload-texture`. |

### variant `shape`

What to draw.

| Case | Payload | |
| --- | --- | --- |
| `rect` | `rect-shape` | A filled rectangle. |
| `text` | `text-shape` | A line of text. |
| `image` | `image-shape` | An uploaded image, scaled into the rectangle. |

### record `key-press`

A key; `text` is the character it typed.

| Field | Type | |
| --- | --- | --- |
| `key` | `string` | The key's name, as the game's (`key.keyboard.m`). |
| `pressed` | `bool` | Pressed (true) or released (false). |
| `text` | `option<string>` | The character the key typed, when it typed one. |

### record `mouse-move`

The mouse moved; the place is in GUI pixels.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |

### record `mouse-press`

A mouse button: 0 left, 1 right, 2 middle.

| Field | Type | |
| --- | --- | --- |
| `button` | `u32` |  |
| `pressed` | `bool` | Pressed (true) or released (false). |
| `x` | `f32` |  |
| `y` | `f32` |  |

### record `wheel-turn`

The wheel; positive is up.

| Field | Type | |
| --- | --- | --- |
| `steps` | `f32` |  |
| `x` | `f32` |  |
| `y` | `f32` |  |

### variant `event`

What the player did while the screen was open.

| Case | Payload | |
| --- | --- | --- |
| `key` | `key-press` | A key. |
| `mouse-move` | `mouse-move` | The mouse moved. |
| `mouse-button` | `mouse-press` | A mouse button. |
| `wheel` | `wheel-turn` | The wheel. |
| `closed` |  | The screen was closed. |

### record `point`

A point in GUI pixels (API 0.1.5).

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |

### record `area`

A rectangle in GUI pixels (API 0.1.5).

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |

### record `modifiers`

The modifier keys held now (API 0.1.5).

| Field | Type | |
| --- | --- | --- |
| `shift` | `bool` |  |
| `ctrl` | `bool` |  |
| `alt` | `bool` |  |

### enum `game-screen`

The screen of the game itself that is open (API 0.1.5).

| Case | |
| --- | --- |
| `none` | None: the world is being played. |
| `inventory` | The player's inventory, with its 2x2 crafting grid. |
| `crafting` | A crafting table. |
| `container` | A container: a chest, a barrel, a furnace, a brewing stand, a crafter, a hopper, a dispenser. |
| `station` | A workstation: an enchanting table, an anvil, a grindstone, a smithing table, a stonecutter, a beacon, a cartography table, a loom, a villager's trades. |
| `mount` | A mount's inventory (a horse, a donkey, a llama). |
| `creative` | The creative inventory. |
| `chat` | The chat. |
| `options` | The options. |
| `other` | Every other screen of the game. |

### record `tip-line`

A line of a tooltip.

| Field | Type | |
| --- | --- | --- |
| `text` | `string` |  |
| `color` | `u32` | 0xAARRGGBB. |

### record `fill-element`

A rectangle; `top` and `bottom` are the colours of its top and bottom edges (equal: flat).

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `top` | `u32` |  |
| `bottom` | `u32` |  |

### record `label-element`

A line of text; (x, y) is its top left corner.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `text` | `string` |  |
| `color` | `u32` | 0xAARRGGBB. |
| `shadow` | `bool` |  |
| `scale` | `f32` | 1.0 is the game's size; the text grows from (x, y). |

### record `sprite-element`

A sprite of the game's atlas by name: the texture's path under `assets/minecraft/textures/` without `.png` (`gui/sprites/hud/heart/full`, `gui/sprites/container/slot`, `item/apple`, ...). It is stretched into the rectangle (no nine-slice). An unknown name draws nothing.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `name` | `string` |  |
| `tint` | `u32` | Multiplies the sprite's colour: 0xFFFFFFFF leaves it as it is, the alpha makes it translucent (the blinking of a bar). |

### record `picture-element`

A part of an image from `upload-texture` (in the full screen), in the image's own pixels, drawn into the rectangle.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `texture` | `u32` |  |
| `u` | `f32` |  |
| `v` | `f32` |  |
| `u-width` | `f32` |  |
| `v-height` | `f32` |  |
| `tint` | `u32` |  |

### record `item-element`

An item's icon (16 by 16 GUI pixels) at (x, y).

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `id` | `string` | `minecraft:oak_planks`, ...; an unknown id draws nothing. |
| `count` | `u32` | Drawn in the corner when above 1. |

### record `tooltip-element`

The game's own tooltip of an item, as the inventory shows it (name, stats, modifiers), its corner at (x, y), kept inside the screen; `extra` are the lines under it.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `id` | `string` |  |
| `count` | `u32` |  |
| `extra` | `list<tip-line>` |  |

### record `tip-element`

A tooltip box of lines, its corner at (x, y), kept inside the screen.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `lines` | `list<tip-line>` |  |

### variant `element`

What to paint (API 0.1.5): everything `shape` draws and what a port of a Java mod's interface needs. Colours are 0xAARRGGBB.

| Case | Payload | |
| --- | --- | --- |
| `fill` | `fill-element` | A rectangle, flat or a vertical gradient. |
| `label` | `label-element` | A line of text. |
| `sprite` | `sprite-element` | A sprite of the game's GUI atlas (a heart, a hunger shank, a button, an icon of the game's own), stretched into a rectangle. |
| `picture` | `picture-element` | A part of an uploaded image. |
| `item` | `item-element` | An item's icon, with its count and durability bar, as the inventory draws it. |
| `tooltip` | `tooltip-element` | The game's own tooltip of an item. |
| `tip` | `tip-element` | A tooltip box of the mod's own lines. |

### enum `texture-filter`

How an image is scaled (API 0.1.31).

| Case | |
| --- | --- |
| `nearest` |  |
| `linear` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `paint` | `func(id: u32, elements: list<element>)` | What to paint in the next frame (API 0.1.5), after what `draw` gave; the elements of the last frame stay until the mod paints again. |
| `get-text-width` | `func(text: string) -> f32` | The width of the text in GUI pixels at the game's font (API 0.1.5); letters the game's bitmap fonts do not have count as 8. |
| `open-panel` | `func(title: string, area: area, keys: list<string>) -> result<u32, string>` | Opens a panel over the game's own screen (API 0.1.5): the inventory, a container, a crafting table. The game keeps drawing and running that screen; the panel is painted over it, and the mouse is the mod's inside `area`, the game's outside. A panel is not the mod's screen: a mod may have both. It is painted only while the game has the inventory, a container, a crafting table or a station open. `keys` are the keys the mod takes instead of the game (`key.keyboard.r`, ...); `*` is every key. The mod reads the panel's input with `get-input`, like the screen's. |
| `set-panel-area` | `func(id: u32, area: area, keys: list<string>)` | Moves or resizes the panel and changes its keys. An area of width or height 0 hides the panel. |
| `get-game-screen` | `func() -> game-screen` | Which screen of the game is open (API 0.1.5). |
| `get-station` | `func() -> option<string>` | The workstation screen that is open (API 0.1.27): `minecraft:enchanting_table`, `minecraft:anvil`, `minecraft:grindstone`, `minecraft:smithing_table`, `minecraft:stonecutter`, `minecraft:beacon`, `minecraft:cartography_table`, `minecraft:loom`, `minecraft:villager` (the trades); none when no workstation screen is open. |
| `get-menu-origin` | `func() -> option<point>` | The top left corner of the open screen's menu (the inventory's, the container's), in GUI pixels; none when there is no menu (API 0.1.5). |
| `get-item-at` | `func(x: f32, y: f32) -> option<string>` | The item in the slot of the open screen under that point, none over no slot or an empty one (API 0.1.5). |
| `get-cursor` | `func() -> point` | Where the mouse is now, in GUI pixels (API 0.1.5). |
| `get-modifiers` | `func() -> modifiers` | The modifier keys held now (API 0.1.5). |
| `open` | `func(title: string) -> result<u32, string>` | Opens the mod's screen and returns its id; the title is shown above it. A mod has one screen: opening another closes the first. |
| `get-size` | `func(id: u32) -> option<size>` | The screen's size in GUI pixels; none when it is closed. |
| `draw` | `func(id: u32, shapes: list<shape>)` | What to draw in the next frame; the shapes of the last frame stay until the mod draws again. |
| `get-input` | `func(id: u32) -> list<event>` | What the player did since the last call; empty when nothing happened. A closed screen keeps its last events, so the mod can read the `closed` one. |
| `close` | `func(id: u32)` | Closes the screen; the mod gets the `closed` event. |
| `upload-texture` | `func(id: u32, width: u32, height: u32, rgba: list<u8>) -> result<_, string>` | Uploads an RGBA image (`width * height * 4` bytes) under the id `id` (0 to 7); the mod draws it with an `image` shape. Uploading under the same id again replaces the image. At most 8 images of 512x512 pixels each (API 0.1.31; 256x256 before). |
| `get-max-texture-side` | `func() -> u32` | The largest side of an image `upload-texture` takes (API 0.1.31): 512. |
| `set-texture-filter` | `func(id: u32, filter: texture-filter)` | How the image `id` is scaled when it is drawn (API 0.1.31): `linear` blends the four nearest pixels (smooth maps, like the Java mods' textures), `nearest` (the default) keeps hard pixels. |

## `world`

Access: capability `world-read`.

Capability `world-read`: blocks of the loaded world by position, and the ones broken since the last tick (API 0.1.2). With capability `world-edit` as well, breaking blocks and wearing the held item, in a world this game runs (single player, or one it opened to LAN); in another's world the edits do nothing and return false or none.

### record `block-pos`

| Field | Type | |
| --- | --- | --- |
| `x` | `s32` |  |
| `y` | `s32` |  |
| `z` | `s32` |  |

### record `world-block`

| Field | Type | |
| --- | --- | --- |
| `pos` | `block-pos` |  |
| `id` | `string` | `minecraft:oak_log`, ... |
| `properties` | `list<tuple<string, string>>` | The state's properties: `axis`=`y`, `distance`=`7`, `persistent`=`false`, ... |
| `tags` | `list<string>` | Every block tag it is in: `minecraft:logs`, `minecraft:leaves`, ... |

### record `block-broken`

| Field | Type | |
| --- | --- | --- |
| `pos` | `block-pos` |  |
| `block` | `world-block` | The block as it was before it broke. |
| `by-player` | `bool` | Broken by the player's hand or tool; false for a block a mod broke. |
| `tool` | `option<string>` | What the player broke it with (`minecraft:iron_axe`, ...); none for a bare hand or a block a mod broke. |
| `sneaking` | `bool` | The player was sneaking as they broke it. |

### record `random-tick`

A block that got a random tick (API 0.1.12).

| Field | Type | |
| --- | --- | --- |
| `pos` | `block-pos` |  |
| `block` | `world-block` | The block as it is now. |

### record `block-removed`

A block that is gone (API 0.1.19).

| Field | Type | |
| --- | --- | --- |
| `pos` | `block-pos` |  |
| `block` | `world-block` | The block as it was before it went. |
| `by-world` | `bool` | The world took it (a leaf decayed, a block died), not the player's hand or a mod: what the player or a mod broke is in `broken-blocks` and comes here with `by-world` false. |

### record `chunk-surface`

The surface of one chunk: what a map shows of its columns (API 0.1.4). The lists hold 256 values, in the order `z * 16 + x`.

| Field | Type | |
| --- | --- | --- |
| `color` | `list<u8>` | Each column's map colour (`MapColor`, 0 = none), as in `get-map-colors`. |
| `y` | `list<s32>` | Each column's height: the Y of the topmost coloured block. |
| `depth` | `list<u8>` | Each column's water (or lava) depth over the surface. |
| `biome` | `list<u16>` | Each column's biome, as an index in `get-biome-names`. |

### record `chunk-details`

What a map shows of a chunk's columns beyond `chunk-surface` (API 0.1.32); the lists hold 256 values, in the order `z * 16 + x`, over the same columns `get-chunk-surface` scans.

| Field | Type | |
| --- | --- | --- |
| `block` | `list<string>` | The id of each column's top coloured block (`minecraft:grass_block`, `minecraft:red_stained_glass`, `minecraft:poppy`, ...); empty when the column has none. |
| `ambiguous` | `list<bool>` | Whether a block that does not hide the top block (leaves, a pane, a plant, a carpet, a snow layer) stands right on it, so the height is ambiguous as a map shows it. |
| `light` | `list<u8>` | The light over each column's top block, 0 to 15: the brighter of its block light and sky light, as the world stores them. |

### record `light-level`

How bright a place is, as the game lights it (API 0.1.20).

| Field | Type | |
| --- | --- | --- |
| `block` | `u32` | The light of blocks, 0 to 15. |
| `sky` | `u32` | The light of the sky, 0 to 15 (before the time of day dims it). |

### record `biome`

A biome as a mod reads it (API 0.1.6).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:desert`, ... |
| `temperature` | `f32` | `Biome.getBaseTemperature`. |
| `precipitation` | `bool` | `Biome.hasPrecipitation`: it gets rain or snow at all. |
| `downfall` | `f32` | How much precipitation falls in it (`downfall`). |
| `foliage` | `u32` | The foliage colour 0xRRGGBB (`FoliageColor`), the grass colour (`GrassColor`, before a grass modifier) and the water colour. |
| `grass` | `u32` |  |
| `water` | `u32` |  |

### record `container-stack`

One stack in a container (API 0.1.9).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:stone`, ... |
| `count` | `u32` |  |
| `damage` | `u32` | The durability used; 0 for an item that does not wear out. |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-block` | `func(pos: block-pos) -> option<world-block>` | The block at `pos`; none where no chunk is loaded (and outside a world). Air is `minecraft:air`. |
| `random-ticked-blocks` | `func() -> list<random-tick>` | The blocks in the watched tags that got a random tick since the previous tick, oldest first: the same positions the client's ambient effects use (667 within 16 and 32 blocks of the player, `ClientLevel.animateTick`), at most 2048 a tick. Empty until `watch-random-ticks` is called (API 0.1.12). |
| `watch-random-ticks` | `func(tags: list<string>)` | Watches the blocks of these tags (`minecraft:leaves`): only they are reported by `random-ticked-blocks`. Calling it again replaces the earlier tags; an empty list stops the listening. Works in `init` too (API 0.1.12). |
| `watch-random-tick-blocks` | `func(blocks: list<string>)` | Watches blocks by id as well (API 0.1.19): `minecraft:pink_wool`, or with the state's properties it must have (`minecraft:bamboo[leaves=large]`). `random-ticked-blocks` reports these and the blocks of the watched tags. Calling it again replaces the earlier list; an empty list stops it. |
| `removed-blocks` | `func() -> list<block-removed>` | The blocks that went since the previous tick, oldest first (the blocks that became air; at most 256 a tick) (API 0.1.19). |
| `broken-blocks` | `func() -> list<block-broken>` | The blocks broken since the previous tick, oldest first: the player's and those mods broke. The same list for every mod, read during `tick`. |
| `break-block` | `func(pos: block-pos, drops: bool) -> bool` | `world-edit`: breaks the block as `Level.destroyBlock` does (its particles and sound), with its drops (its loot table without a tool) when `drops`. True when a breakable block was there. The block breaks after the call returns: until then `get-block` still sees it. |
| `break-block-with-held` | `func(pos: block-pos) -> bool` | `world-edit`: as `break-block` with drops, the loot rolled with the item in the player's hand (Fortune, Silk Touch, the correct tool), which is not worn by it. |
| `damage-held` | `func(amount: u32) -> option<u32>` | `world-edit`: wears the item in the player's main hand by `amount` uses (Unbreaking may spare some, as in vanilla); it may break. The durability left after the full `amount`, none when the hand holds nothing that wears or the player is in creative. |
| `get-chunk-surface` | `func(cx: s32, cz: s32) -> option<chunk-surface>` | The surface of the loaded chunk (cx, cz); none when the chunk is not loaded. The scan is the game's, as the minimap's is. |
| `get-chunk-details` | `func(cx: s32, cz: s32) -> option<chunk-details>` | The details of the loaded chunk (cx, cz); none when it is not loaded. |
| `get-biome-names` | `func() -> list<string>` | The world's biome names, in the order of their indexes (the ones `get-chunk-surface` gives); empty outside a world. |
| `get-light-level` | `func(pos: block-pos) -> option<light-level>` | `world-read` (API 0.1.20): the light at a block; none where no chunk is loaded. |
| `get-biome-fog` | `func(pos: block-pos) -> option<u32>` | `world-read` (API 0.1.22): the colour of the fog of the biome at a block, 0xRRGGBB (the biome's `fog_color`, the overworld's when it has none); none where no chunk is loaded. |
| `run-command` | `func(command: string) -> bool` | `world-edit` (API 0.1.20): runs a command as the player typed it, without the slash, after the hook. False when the command is empty, the mod has no `world-edit` or the world is not this game's own. |
| `merge-drops-at` | `func(pos: block-pos) -> bool` | `world-edit` (API 0.1.23): after the hook, moves the items and the experience that blocks broken lately by `break-block` dropped (the last few ticks, within 16 blocks of `pos`) to the middle of the block at `pos`. False in a world this game does not run. |
| `fall-block` | `func(pos: block-pos, vx: f64, vy: f64, vz: f64, drops: bool, place: bool) -> bool` | `world-edit` (API 0.1.24): takes the block at `pos` out of the world and makes it fall as a falling block with that velocity (blocks per tick), after the hook. `drops`: the loot the block gives to the held item falls out where it stood. `place`: where it lands it stays a block, or, when it cannot be placed there (another block is in the way), drops as an item like vanilla's falling blocks; false: it vanishes there. False when no breakable block is at `pos` or the world is not this game's own. |
| `set-block` | `func(pos: block-pos, id: string, properties: list<tuple<string, string>>) -> bool` | `world-edit` (API 0.1.24): puts the block `id` (`minecraft:oak_log`) at `pos` with those properties (`axis` = `y`); the ones not given are the default state's. The neighbours are updated as for a command's `setblock`. False for an unknown block or a bad property, no loaded chunk, or a world this game does not run; at most 64 a hook. |
| `spawn-item` | `func(pos: block-pos, id: string, count: u32) -> bool` | `world-edit` (API 0.1.24): lets an item fall out at `pos`, the way an item comes out of a broken block, after the hook. False for an unknown item or a count of 0 or more than 64; at most 64 a hook. |
| `set-weather-passthrough` | `func(blocks: list<string>)` | `world-read` (API 0.1.25): the blocks the mod's colliding particles fly through: ids (`minecraft:oak_fence`) and tags (`#minecraft:fences`). Replaces the mod's earlier list; an empty list lets them hit every block that stops a particle. |
| `get-map-colors` | `func() -> list<u32>` | The colours of the map (`MapColor`) by id, as 0xRRGGBB; the id is the one `get-chunk-surface` gives. Empty outside a world. |
| `get-chunk-columns` | `func(cx: s32, cz: s32, y: s32) -> option<chunk-surface>` | As `get-chunk-surface`, but the scan goes down from the height `y` instead of the top of the chunk: the highest block with a map colour at `y` or below, for a cave map (API 0.1.7). |
| `get-biome` | `func(pos: block-pos) -> option<biome>` | The biome at `pos`; none where no chunk is loaded (API 0.1.6). |
| `get-surface` | `func(pos: block-pos, start-y: s32) -> option<s32>` | Where the ground is in the column of `pos`: the y of the first free space above the first block that stops motion (solid, leaves or a fluid) found going down from `start-y` (`Heightmap.MOTION_BLOCKING`'s idea); none where no chunk is loaded (API 0.1.6). |
| `is-solid` | `func(pos: block-pos) -> bool` | Whether the block at `pos` has a full collision box (API 0.1.6). |
| `get-container` | `func(pos: block-pos) -> option<list<option<container-stack>>>` | The items of the container at `pos` (a chest, a barrel, a shulker box, a hopper, a dropper, a dispenser, a crafter; a furnace, a blast furnace or a smoker: input, fuel, result), in slot order, an empty slot none. Only for the block the player looks at or is breaking, in a world this game runs or whose contents the game knows; none for any other position or block (API 0.1.9). |

## `items`

Access: capability `items-read`.

Capability `items-read`: the items of the game, vanilla's and the content mods' (API 0.1.5).

### record `item`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:oak_planks`, ... |
| `name` | `string` | Its name in the game's language; the id when it has none. |
| `max-stack` | `u32` | The stack size: 64 for most items, 1 for tools and armour. |
| `tags` | `list<string>` | Every tag it is in, as full names without `#`. |

| Function | Signature | What it does |
| --- | --- | --- |
| `list` | `func() -> list<item>` | Every item of the game, in the order of the creative inventory's search tab. |
| `get` | `func(id: string) -> option<item>` | One item by id; none for an unknown id. |
| `get-name-color` | `func(id: string) -> option<u32>` | The colour the game shows the item's name in (API 0.1.20), 0xRRGGBB: by its rarity, white for common, yellow, aqua or light purple above. None for an unknown id. |

## `recipes`

Access: capability `recipes-read`.

Capability `recipes-read`: the recipes of the game, vanilla's and the content mods' (API 0.1.5). The recipes that run code (dyeing, copying a map) have no fixed slots and are not listed.

### enum `kind`

| Case | |
| --- | --- |
| `crafting-shaped` |  |
| `crafting-shapeless` |  |
| `smelting` |  |
| `blasting` |  |
| `smoking` |  |
| `campfire-cooking` |  |
| `stonecutting` |  |
| `smithing-transform` |  |
| `smithing-trim` |  |

### record `recipe`

| Field | Type | |
| --- | --- | --- |
| `kind` | `kind` |  |
| `station` | `string` | The block it is made at, as an item id: `minecraft:crafting_table`, `minecraft:furnace`, ... |
| `inputs` | `list<list<string>>` | The slots that take items, one list per slot: the items it accepts (one item, or the items of a tag or a list). A crafting recipe's are the grid's cells in row order, the empty ones as empty lists (shapeless: 2 wide for up to 4 ingredients, else 3 wide); cooking and stonecutting have one slot, smithing three (template, base, addition). |
| `output` | `option<string>` | The item made (the recipe's result); none for a trim, which keeps its base. |
| `count` | `u32` | How many of the result it makes. |
| `width` | `u32` | A crafting recipe's grid; 0 for the other kinds. |
| `height` | `u32` |  |
| `time` | `u32` | A cooking recipe's time in ticks and the experience it gives. |
| `experience` | `f32` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `list` | `func() -> list<recipe>` | Every recipe of the game. |

## `weather`

Access: capability `weather-read`.

Capability `weather-read`: the weather over the player (API 0.1.6).

### record `state`

| Field | Type | |
| --- | --- | --- |
| `rain` | `f32` | 0 to 1: how hard it rains (`Level.getRainLevel`). |
| `thunder` | `f32` | 0 to 1: how hard the thunderstorm is (`Level.getThunderLevel`). |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-weather` | `func() -> option<state>` | The weather over the player; none outside a world and in a dimension that has none. |
| `set-default-weather` | `func(visible: bool)` | Whether the game draws its own rain and snow columns (API 0.1.22). A mod that draws the weather with its own particles turns them off; they stay off while any running mod says so, and come back when the mod says `true` or stops. |
| `set-default-splashing` | `func(visible: bool)` | Whether the game draws its own rain splashes on the ground (API 0.1.25). A mod that draws its own splashes turns them off; they stay off while any running mod says so. Smoke over lava and campfires stays. |

## `settings`

Access: capability `settings-read`.

Capability `settings-read`: the client's own settings (API 0.1.6).

| Function | Signature | What it does |
| --- | --- | --- |
| `get-particles` | `func() -> u32` | The Particles setting: 0 all, 1 decreased, 2 minimal. |

## `particles`

Access: capability `particles`.

Capability `particles`: the mod's own particles with its own pictures (API 0.1.6).

### record `sprite`

A picture the mod's particles are drawn with.

| Field | Type | |
| --- | --- | --- |
| `name` | `string` | The mod's own name for it (`rain_drop`); the same name replaces the old picture. |
| `width` | `u32` | 1 to 64 pixels. |
| `height` | `u32` |  |
| `pixels` | `list<u8>` | RGBA, four bytes a pixel, row by row from the top left. |

### record `color`

A colour, each part 0 to 1; white leaves the picture's own colours.

| Field | Type | |
| --- | --- | --- |
| `r` | `f32` |  |
| `g` | `f32` |  |
| `b` | `f32` |  |
| `a` | `f32` |  |

### enum `size-curve`

How the particle's size changes as it lives.

| Case | |
| --- | --- |
| `constant` | The same size the whole time. |
| `grow` | Grows from a third of its size to its size. |
| `shrink` | Shrinks to nothing. |

### enum `fade`

How the particle fades out.

| Case | |
| --- | --- |
| `none` | It stays as bright as it is. |
| `late` | It fades at the end of its life, as vanilla's particles do. |
| `full` | It fades over its whole life. |

### record `options`

One particle.

| Field | Type | |
| --- | --- | --- |
| `sprite` | `u32` | The picture (the number `define-sprite` gave). |
| `x` | `f64` | Where it starts, in blocks. |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `vx` | `f64` | How fast it moves at the start, in blocks per tick. |
| `vy` | `f64` |  |
| `vz` | `f64` |  |
| `color` | `color` |  |
| `size` | `f32` | The width of its quad, in blocks. |
| `lifetime` | `u32` | How long it lives, in ticks (1 to 1200). |
| `gravity` | `f32` | The downward speed it gains every tick. |
| `wind-x` | `f64` | The sideways speed (x and z, blocks per tick) it is pulled towards: the wind. 0 and 0 leaves its own. |
| `wind-z` | `f64` |  |
| `collide` | `bool` | Whether it stops at the blocks and fluids it hits (and is then reported by `landed`). |
| `size-curve` | `size-curve` |  |
| `fade` | `fade` |  |
| `flat` | `bool` | Whether it lies flat (a ring on the water). |
| `linger` | `u32` | How many ticks it lies where it landed before it vanishes. |
| `sway` | `f32` | How far it sways sideways, in blocks per tick at most (0: no sway), and how many ticks a sway takes (at least 1). |
| `sway-period` | `u32` |  |
| `spin` | `f32` | How fast it turns about its own axis and about a horizontal one, in radians per tick. |
| `tumble` | `f32` |  |

### record `landing`

Where one of the mod's colliding particles landed.

| Field | Type | |
| --- | --- | --- |
| `sprite` | `u32` | The picture of the particle that landed. |
| `x` | `f64` |  |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `water` | `bool` | It hit a fluid (the point is on its surface), not a solid block (the point is on the block's top). |

### enum `rotation`

How a particle's picture is turned (API 0.1.22).

| Case | |
| --- | --- |
| `camera` | Faces the camera (what `spawn` does). |
| `relative-velocity` | Turns about the view axis so that the picture's up points along the particle's velocity as the camera sees it: a raindrop's streak leans with the wind. |
| `copy-camera` | Keeps the camera's orientation and ignores `spin` and `tumble`. |
| `vertical` | Stands upright in a vertical plane that turns to face the camera about the vertical axis only (a sheet of fog). |
| `crossed` | Two upright quads crossing at right angles, the model of a plant like `minecraft:dead_bush`: the picture is the plant's texture (from `get-game-sprite`), `spin` turns it about the vertical axis and `tumble` over about a horizontal one (API 0.1.25). |

### enum `render-type`

How a particle is drawn (API 0.1.30).

| Case | |
| --- | --- |
| `opaque` | Drawn solid: its colour's alpha and fade are ignored. |
| `translucent` | Drawn with its alpha (the default of `spawn`). |
| `blended` | Drawn with its alpha, as a glow; the game has no additive pass yet, so it looks like `translucent`. |

### record `style`

What `spawn-styled` adds to `options` and `rotation` (API 0.1.30).

| Field | Type | |
| --- | --- | --- |
| `bounciness` | `f32` | A colliding particle that lands fast enough bounces back up with this share of its speed (0 = it lands, as before). |
| `render-type` | `render-type` |  |
| `distance-scaling` | `f32` | 0: the size of a particle follows the perspective; 1: it keeps the size it has 8 blocks away, whatever its distance (a constant size on the screen); between, a mix. |

### record `particle`

One of the mod's own living particles (API 0.1.25).

| Field | Type | |
| --- | --- | --- |
| `sprite` | `u32` | The picture's number, as `spawn` took it. |
| `x` | `f64` |  |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `vx` | `f64` |  |
| `vy` | `f64` |  |
| `vz` | `f64` |  |
| `remaining` | `u32` | How many ticks it still lives. |

### record `game-image`

A picture of the game's block, item or GUI atlas (API 0.1.19).

| Field | Type | |
| --- | --- | --- |
| `width` | `u32` |  |
| `height` | `u32` |  |
| `pixels` | `list<u8>` | RGBA, four bytes a pixel, row by row from the top left, as `define-sprite` takes it. |

| Function | Signature | What it does |
| --- | --- | --- |
| `define-sprite` | `func(sprite: sprite) -> result<u32, string>` | Adds a picture and gives its number; an error when it is refused (too big, wrong size of pixels, no room: 16 pictures in all, shared by the mods). |
| `spawn` | `func(options: options) -> bool` | Spawns one particle; false when it was refused (an unknown picture, or more than 512 particles in this call). At most a few thousand of the mods' particles live at once. |
| `spawn-rotated` | `func(options: options, rotation: rotation) -> bool` | As `spawn`, with the way the picture is turned (API 0.1.22). |
| `spawn-styled` | `func(options: options, rotation: rotation, style: style) -> bool` | As `spawn-rotated`, with a style. |
| `get-particles` | `func() -> list<particle>` | The mod's own particles that live now, as of the last tick (at most 4096); empty when the mod has none. |
| `remove` | `func(sprite: u32, x: f64, y: f64, z: f64) -> u32` | Removes the mod's own particles with the picture number `sprite` that are within a block of (`x`, `y`, `z`) (API 0.1.26). They are gone after the hook. How many of the particles `get-particles` listed match. |
| `landed` | `func() -> list<landing>` | The mod's own particles that landed since the last call, oldest first. |
| `get-game-sprite` | `func(name: string) -> option<game-image>` | The picture of a sprite of the game's block atlas (`minecraft:block/oak_leaves`), as the enabled resource packs give it, or none when there is no such sprite. A mod draws its particles with it by giving the pixels to `define-sprite` (API 0.1.19). |
| `set-suppressed` | `func(types: list<string>, scale: f32)` | The particles of the game that the mod silences or scales (API 0.1.19): their ids (`minecraft:tinted_leaves`, `minecraft:cherry_leaves`, `minecraft:pale_oak_leaves`) and a multiplier of the chance with which the game's blocks spawn them (0: none). The list replaces the mod's earlier one; with several mods the smallest multiplier wins; it ends with the mod. Only particles the game spawns from blocks by chance (falling leaves) can be scaled. |
| `register-command-particle` | `func(name: string, options: options) -> result<_, string>` | Makes `/particle <mod id>:<name>` spawn this particle (API 0.1.19): `options` are the template (its position is where the command puts it, its velocity the command's delta), and the picture must have been defined before. An error for a bad name; the same name replaces the old one. |

## `atmosphere`

Access: capability `atmosphere`.

Capability `atmosphere`: the mod's own weather in the fog (API 0.1.6).

### record `haze`

| Field | Type | |
| --- | --- | --- |
| `amount` | `f32` | 0 to 1: how much of the fog's colour the haze takes. |
| `red` | `f32` | The colour it takes, each part 0 to 1. |
| `green` | `f32` |  |
| `blue` | `f32` |  |
| `density` | `f32` | How much closer the fog comes: its end distance times `1 - density` (0 to 0.9). |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-haze` | `func(haze: option<haze>)` | The haze of the mod; none for none. The game keeps it until the mod sets it again and drops it when the mod stops. With several mods the strongest haze counts. |

## `overlay`

Access: capability `overlay`.

Capability `overlay`: the mod's own graphics over the world, in the HUD's layer, without taking the mouse or the keyboard (API 0.1.7). It draws the elements `screen.paint` does.

### type `element`

`type element = element`

### type `size`

`type size = size`

### enum `layer`

Where in the HUD an element goes.

| Case | |
| --- | --- |
| `under-bars` | Before the game draws its status bars (health, hunger, armour, air, experience): the bars are drawn over it. |
| `over-bars` | After them, over every other part of the HUD but the chat and the screens. |

### record `world-box`

The twelve edges of a box 1 by 1 by 1 around a point, as the game draws a block outline.

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` | The middle of the box. |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `color` | `u32` | 0xAARRGGBB. |
| `width` | `f32` | The line's width in pixels. |
| `through-blocks` | `bool` | Drawn over the blocks, not hidden by them. |

### record `world-picture`

A part of an image of `upload-texture` drawn at a point of the world, its middle at the point.

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` |  |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `texture` | `u32` | The image's id. |
| `u` | `f32` | The part of the image, in the image's pixels. |
| `v` | `f32` |  |
| `u-width` | `f32` |  |
| `v-height` | `f32` |  |
| `tint` | `u32` | 0xAARRGGBB multiplying the image's colours. |
| `offset-y` | `f32` |  |

### record `world-label`

A line of text drawn at a point of the world, always facing the camera, at a size that does not change with the distance. Its middle is at the point, `offset-y` GUI pixels down.

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` |  |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `text` | `string` |  |
| `color` | `u32` | 0xAARRGGBB. |
| `shadow` | `bool` |  |
| `scale` | `f32` |  |
| `offset-y` | `f32` |  |

### record `world-sprite`

A sprite of the GUI atlas drawn at a point of the world, its middle at the point.

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` |  |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `name` | `string` |  |
| `tint` | `u32` | 0xAARRGGBB. |
| `offset-y` | `f32` |  |

### record `world-fill`

A filled rectangle drawn at a point of the world, its middle at the point.

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` |  |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `color` | `u32` | 0xAARRGGBB. |
| `offset-y` | `f32` |  |

### variant `world-element`

A point of the world with a picture drawn at it (a waypoint).

| Case | Payload | |
| --- | --- | --- |
| `label` | `world-label` |  |
| `sprite` | `world-sprite` |  |
| `fill` | `world-fill` |  |
| `picture` | `world-picture` | A part of an image from `upload-texture` (API 0.1.20). |
| `box` | `world-box` | The edges of a block-sized box in the world (API 0.1.23). |

### record `hud-state`

What the HUD is doing.

| Field | Type | |
| --- | --- | --- |
| `screen-open` | `bool` | A screen (the inventory, the chat, the pause menu) is open. |
| `debug` | `bool` | The debug screen (F3) is on. |
| `hidden` | `bool` | The HUD is hidden (F1). |

| Function | Signature | What it does |
| --- | --- | --- |
| `paint` | `func(layer: layer, elements: list<element>)` | What to paint in a layer in the next frames; the elements of the last call stay until the mod paints again or clears. `picture` elements use the images of `upload-texture`. |
| `clear` | `func(layer: layer)` | Removes what the mod painted in the layer. |
| `paint-world` | `func(elements: list<world-element>)` | What to draw at points of the world; stays until the next call. The game projects the points with the camera of the last frame and leaves out those behind it. |
| `clear-world` | `func()` | Removes what the mod drew in the world. |
| `get-size` | `func() -> option<size>` | The screen's size in GUI pixels; none when the game has no window. |
| `get-text-width` | `func(text: string) -> f32` | The width of the text in GUI pixels at the game's font. |
| `upload-texture` | `func(id: u32, width: u32, height: u32, rgba: list<u8>) -> result<_, string>` | Uploads an RGBA image (`width * height * 4` bytes) under the id `id` (0 to 6); it is drawn by `picture` elements of the overlay. At most 7 images of 256x256 pixels shared by all the mods' overlays. |
| `get-hud-state` | `func() -> option<hud-state>` | What the HUD is doing; none outside a world. |
| `is-tab-list-open` | `func() -> bool` | The player list (Tab) is open (API 0.1.34; `overlay`). A separate function: `hud-state` is a published record. |

## `entity`

Access: capability `entity-read`.

Capability `entity-read`: the entities around the player (API 0.1.8). With capability `entity-edit` as well, removing entities and changing what an experience orb is worth, in a world this game runs (single player, or one it opened to LAN); in another's world the edits do nothing and return false. What the game keeps of the other world's entities is what the server told it.

### enum `entity-kind`

What an entity is.

| Case | |
| --- | --- |
| `mob` | A mob (a zombie, a cow, a villager, an armour stand). |
| `item` | An item lying in the world. |
| `player` | A player other than the local one. |
| `other` | Everything else: an experience orb, an arrow, a boat. |

### record `entity`

One entity.

| Field | Type | |
| --- | --- | --- |
| `id` | `u32` | Its id in the world, as of the last tick. |
| `type-id` | `string` | `minecraft:zombie`, `minecraft:item`, `minecraft:experience_orb`, ... |
| `kind` | `entity-kind` |  |
| `name` | `string` | Its name in the game's language (its custom name when it has one); an item's is the item's. |
| `uuid` | `string` | A player's UUID; empty for the others. |
| `x` | `f64` | Where it is, as of the last tick, in blocks. |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `yaw` | `f32` | Where it faces, in degrees. |
| `height` | `f32` | Its height in blocks. |
| `health` | `f32` | Its health when it has one; 0 for an entity without. |
| `max-health` | `f32` |  |
| `xp` | `u32` | What an experience orb is worth; 0 for every other entity. |
| `on-fire` | `bool` | It is burning. |
| `hostile` | `bool` | A mob that attacks the player (the game's monster category). |
| `held-item` | `option<string>` | What it holds in its main hand and its off hand; an item's own item in `held-item`. |
| `off-hand-item` | `option<string>` |  |

### record `face`

A player's face as an image.

| Field | Type | |
| --- | --- | --- |
| `width` | `u32` |  |
| `height` | `u32` |  |
| `rgba` | `list<u8>` | `width * height * 4` bytes, RGBA. |

### record `detail`

The extra state of one entity (API 0.1.20), as of the last tick.

| Field | Type | |
| --- | --- | --- |
| `swell` | `f32` | A creeper's fuse: 0 when it is not about to explode, up to 1 when it is (`Creeper.getSwelling`); primed TNT's fuse the same way (1 at the start, 0 at the blast). |
| `glowing` | `bool` | It has the Glowing effect. |
| `in-water` | `bool` | Its eyes are in water (`Entity.wasEyeInWater`). |
| `squish` | `f32` | How much a cube mob (a slime, a magma cube) is squished, 0 or more; 0 for the others. |
| `dark-ticks` | `u32` | How many ticks a glow squid has left to glow; 0 when it glows fully. The game does not track it, so always 0 for now. |
| `carried-block` | `string` | The block an enderman carries, empty when it carries none. |

### record `type-info`

One entity type of the game's registry (API 0.1.33).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | The registry key, e.g. `minecraft:creeper`. |
| `name` | `string` | The display name from the language files (`entity.<ns>.<path>`), empty when the language has none. |
| `tags` | `list<string>` | The entity tags the type is in, full names without `#` (`minecraft:skeletons`), sorted. |

| Function | Signature | What it does |
| --- | --- | --- |
| `nearby` | `func(type-id: string, radius: f64) -> list<entity>` | The entities of the kind (`minecraft:experience_orb`, ...) within `radius` blocks of the player (at most 512 blocks); every kind when `type-id` is empty. The nearest first, at most 1024. Empty outside a world. |
| `remove` | `func(id: u32) -> bool` | `entity-edit`: removes the entity with that id; false when there is no such entity or the world is not this game's. |
| `set-xp` | `func(id: u32, xp: u32) -> bool` | `entity-edit`: sets what an experience orb is worth; false for another entity or a world this game does not run. |
| `get-face` | `func(uuid: string) -> option<face>` | The face of the player with that UUID (the 8 by 8 face and hat of the skin); none when the game has no skin of theirs. None for now for every UUID: the skins are not given to mods yet. |
| `get-detail` | `func(id: u32) -> option<detail>` | The state of the entity with that id (API 0.1.20; `entity-read`); none for an id `nearby` did not list. |
| `get-types` | `func() -> list<type-info>` | Every entity type of the registry, sorted by id (API 0.1.33; `entity-read`). `BuiltInRegistries.ENTITY_TYPE`, not only the types that were seen near the player. |
| `is-invisible` | `func(id: u32) -> bool` | Whether the entity with that id is invisible (`Entity.isInvisible`: the Invisibility effect or the invisible flag), as of the last tick (API 0.1.33; `entity-read`); false for an id `nearby` did not list. A separate function: `detail` is a published record and stays as it is. |

## `slots`

Access: capability `slots-read`.

Capabilities `slots-read` and `slot-decorate`: the game's own slots, the one under the cursor, and the marks the mod has the game draw on them (API 0.1.10).

### record `picture-mark`

A picture of the mod on a slot, drawn over the item. The images are those of `upload-texture` (the same as `overlay.upload-texture`'s).

| Field | Type | |
| --- | --- | --- |
| `texture` | `u32` | The image's id, 0 to 6. |
| `width` | `f32` | The size it is drawn at, in GUI pixels. |
| `height` | `f32` |  |
| `x` | `f32` | Where its top left corner goes in the slot, in GUI pixels (0, 0 is the slot's top left corner; the slot is 16 by 16). |
| `y` | `f32` |  |
| `color` | `u32` | 0xAARRGGBB multiplying the image's colours; white as it is. |

### record `square-mark`

A square in a chosen place of the slot.

| Field | Type | |
| --- | --- | --- |
| `color` | `u32` | 0xAARRGGBB. |
| `size` | `f32` | The side in GUI pixels. |
| `x` | `f32` |  |
| `y` | `f32` |  |

### record `frame-mark`

| Field | Type | |
| --- | --- | --- |
| `color` | `u32` | 0xAARRGGBB. |

### record `dot-mark`

| Field | Type | |
| --- | --- | --- |
| `color` | `u32` | 0xAARRGGBB. |
| `size` | `f32` | The side of the square in GUI pixels. |

### record `fill-mark`

| Field | Type | |
| --- | --- | --- |
| `color` | `u32` | 0xAARRGGBB. |

### variant `mark-shape`

What to draw on a slot: 16 by 16 GUI pixels where an item is.

| Case | Payload | |
| --- | --- | --- |
| `frame` | `frame-mark` | A frame one pixel thick around the slot. |
| `dot` | `dot-mark` | A square in the slot's top right corner. |
| `fill` | `fill-mark` | The whole slot filled (a highlight). |
| `picture` | `picture-mark` | The mod's own picture on the slot (API 0.1.20). |
| `square` | `square-mark` | A square wherever in the slot (API 0.1.20). |

### record `mark`

A mark on one of the game's slots.

| Field | Type | |
| --- | --- | --- |
| `slot` | `u32` | The slot's id: 0 to 8 the hotbar (on the HUD, in the inventory and in the creative inventory's item tab), 9 to 35 the rest of the inventory, 100 to 108 the crafting grid, 200 the crafting result, 300 and up a container's slots, 500 to 503 the armour (head to feet), 504 the off hand. |
| `shape` | `mark-shape` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-marks` | `func(marks: list<mark>)` | `slot-decorate`: the marks the game draws on its slots, in the HUD's hotbar and in the open screen with slots; a mark of a slot that is not on screen is kept and not drawn. They stay until the mod sets them again (an empty list clears them), and go when the mod stops. At most 256 marks. |
| `set-screen-marks` | `func(marks: list<mark>)` | `slot-decorate` (API 0.1.20): marks drawn only on the slots of the open screen, in addition to those of `set-marks`. |
| `set-hud-marks` | `func(marks: list<mark>)` | `slot-decorate` (API 0.1.20): marks drawn only on the hotbar of the HUD, in addition to those of `set-marks`. |
| `upload-texture` | `func(id: u32, width: u32, height: u32, rgba: list<u8>) -> result<_, string>` | `slot-decorate` (API 0.1.20): an image for `picture` marks; the store is the one of `overlay.upload-texture` (the same ids, at most 7 images of 256 by 256 pixels). |
| `get-hovered-slot` | `func() -> option<u32>` | `slots-read`: the slot under the cursor in the open screen with slots (the inventory, a container); none otherwise. |

## `tooltip`

Access: capability `tooltip`.

Capability `tooltip`: what the mod adds to the tooltip the game draws for an item (API 0.1.11).

### type `element`

`type element = element`

### type `tip-line`

`type tip-line = tip-line`

### record `hovered`

The item whose tooltip the game drew in the last frame (inventory, containers, the creative inventory, the workstation screens), with the tooltip's frame.

| Field | Type | |
| --- | --- | --- |
| `item` | `string` | `minecraft:cooked_beef`, ... |
| `x` | `f32` | The frame in GUI pixels. |
| `y` | `f32` |  |
| `width` | `f32` |  |
| `height` | `f32` |  |

### record `content`

What the mod adds under the tooltip's text: the room it takes and what is drawn in it. The elements' positions are from the top left of that room; `picture` elements use the images of `overlay.upload-texture`.

| Field | Type | |
| --- | --- | --- |
| `width` | `f32` |  |
| `height` | `f32` |  |
| `elements` | `list<element>` |  |

### record `enchantment`

An enchantment of the item the tooltip is for (API 0.1.27).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:sharpness`, ... |
| `level` | `u32` | Its level. |
| `stored` | `bool` | Stored in the item (an enchanted book) rather than on it. |

### record `line`

A line a mod adds into the text of an item's tooltip (API 0.1.27).

| Field | Type | |
| --- | --- | --- |
| `under` | `string` | The translation key of the game's line the new one goes right under: `enchantment.minecraft.sharpness` for the line of that enchantment. Empty: under the last line. A line whose key is not in the tooltip is not drawn. |
| `text` | `string` | What the line says. |
| `color` | `u32` | 0xAARRGGBB. |

### record `container-item`

One item of a container (API 0.1.28).

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:stone`, ... |
| `count` | `u32` |  |
| `damage` | `u32` | The durability used; 0 for an item that does not wear out. |

### record `hovered-stack`

A stack with what is in it (API 0.1.28).

| Field | Type | |
| --- | --- | --- |
| `item` | `string` | `minecraft:red_shulker_box`, ... |
| `count` | `u32` |  |
| `damage` | `u32` |  |
| `container` | `list<option<container-item>>` | The items of the stack's `minecraft:container` component by slot (27 slots or more); an empty slot is none. Empty for a stack without the component. |

### record `hovered-tooltip`

A tooltip of the mod's own for one stack (API 0.1.28).

| Field | Type | |
| --- | --- | --- |
| `lines` | `option<list<tip-line>>` | The lines of the tooltip in place of the game's own (the first is the name); none keeps the game's lines (and the mod's `set-lines`). |
| `width` | `f32` | The room under the lines and what is drawn in it, from the top left of that room. |
| `height` | `f32` |  |
| `elements` | `list<element>` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-hovered` | `func() -> option<hovered>` | The item the game drew a tooltip for in the last frame; none when the cursor was over none. |
| `set-content` | `func(item: string, content: option<content>)` | What to add to the tooltip of the item (all stacks of that item id); none removes it. It stays until the mod changes it, and goes with the mod. Up to 32 items, 128 elements each, and 256 by 256 pixels. |
| `get-hovered-enchantments` | `func() -> list<enchantment>` | The enchantments of the item the game drew a tooltip for in the last frame, in the order of the tooltip (the stored ones first); empty when the cursor was over no item (API 0.1.27). |
| `set-lines` | `func(lines: list<line>)` | The lines this mod adds to the tooltip of the item under the cursor (API 0.1.27); they stay until the mod changes them and go with the mod. Up to 64 lines of 256 characters. Lines of several mods are drawn in the order of the mods' ids. |
| `get-hovered-stack` | `func() -> option<hovered-stack>` | The stack the game drew a tooltip for in the last frame, with what is in it; none when the cursor was over none (API 0.1.28). |
| `set-hovered-tooltip` | `func(tooltip: option<hovered-tooltip>)` | The mod's own tooltip for the stack it last read with `get-hovered-stack`; none gives the game's tooltip back. The game shows it only while it draws a tooltip for that same stack (the same item, count, damage and contents), so a mod sets it in `frame` right after reading the stack; for another stack the game's own tooltip is drawn. It goes with the mod. |

## `resources`

Access: capability `resource-read`.

Capability `resource-read`: the files of the enabled resource packs, as the game merges them (API 0.1.14).

| Function | Signature | What it does |
| --- | --- | --- |
| `namespaces` | `func() -> list<string>` | The namespaces the packs have files for: `minecraft` and the packs' own, sorted. |
| `list-files` | `func(namespace: string, prefix: string) -> list<string>` | The files under `assets/<namespace>/<prefix>`, as paths relative to `assets/<namespace>/` (`optifine/ctm/glass/0.properties`, `textures/block/glass.png`), sorted; an empty prefix lists all. At most 20000. |
| `read-file` | `func(namespace: string, path: string) -> option<list<u8>>` | A file's bytes (up to 32 MiB); none when there is no such file. |
| `file-origin` | `func(namespace: string, path: string) -> option<string>` | The pack the file comes from, as the game merged them: `vanilla` for the game's own file, the pack's id (`file/my_pack`) for a file a pack gave or replaced; none when there is no such file (API 0.1.17). |

## `texture-rules`

Access: capability `texture-rules`.

Capability `texture-rules`: what the faces of blocks show (API 0.1.14). The mod gives pictures and rules; the mesher applies them to each face it draws, so no mod code runs per face.

### record `block-filter`

A block state a rule matches.

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:glass`, ... |
| `properties` | `list<tuple<string, string>>` | Properties the state must have, all of them (`axis`=`y`). |

### enum `connect`

When a neighbour counts as connected.

| Case | |
| --- | --- |
| `block` | The same block (id), whatever its properties. |
| `state` | The same state (id and every property). |
| `tile` | A neighbour that shows the same picture on the same side. |

### enum `method`

How the face's tile is chosen.

| Case | |
| --- | --- |
| `pattern` | By the pattern of connected neighbours (`pattern-tiles`). |
| `random` | By the face's position, weighted (`tiles` and `weights`). |
| `position` | By the block's place along the face (`grid-tiles`). |
| `quarters` | Four quarters of the face, each with its own table (`quarter-tiles`). |

### record `rule`

| Field | Type | |
| --- | --- | --- |
| `blocks` | `list<block-filter>` | States the rule matches. |
| `sprites` | `list<string>` | Pictures the rule matches: `minecraft:block/glass`. A face that shows one of them uses the rule too. |
| `faces` | `u32` | Faces the rule matches, a mask: north 1, east 2, south 4, west 8, up 16, down 32. |
| `connect` | `connect` |  |
| `method` | `method` |  |
| `tiles` | `list<string>` | The pictures the rule may draw: names of `upload-sprite` (the game adds the mod's id), or a picture of the game's own (`minecraft:block/glass`). |
| `pattern-tiles` | `list<u32>` | `pattern`: the tile of each of the 256 patterns, an index in `tiles`; 4294967295 for none. Bits: 1 the neighbour on the left of the face, 2 lower left, 4 below, 8 lower right, 16 right, 32 upper right, 64 above, 128 upper left. Left and right follow the picture's `u`, below follows its `v`. |
| `quarter-tiles` | `list<u32>` | `quarters`: eight tiles for each quarter (upper left, upper right, lower left, lower right), indexed by 1 side neighbour connected (the one on the quarter's horizontal side), 2 vertical side, 4 diagonal. |
| `grid-tiles` | `list<u32>` | `position`: the tile of each cell of a `width` by `height` grid, row by row; the cell is `(v mod height) * width + (u mod width)` for the block's `u` along the face (to the right) and `v` (down). |
| `width` | `u32` |  |
| `height` | `u32` |  |
| `weights` | `list<u32>` | `random`: each tile's weight; a missing one weighs 1. |

| Function | Signature | What it does |
| --- | --- | --- |
| `upload-sprite` | `func(name: string, png: list<u8>) -> result<_, string>` | Adds a picture (PNG) to the block atlas under `name`; the same name replaces the old picture. The game rebuilds the atlas and meshes the world again. At most 512 pictures a mod, 1 MiB each. |
| `register-rule` | `func(rule: rule) -> result<_, string>` | Adds a rule; rules are tried in the order they were added (the mods' in turn), and the first that gives a tile wins. At most 512 a mod. |
| `clear-rules` | `func()` | Removes every rule and picture of this mod. |

## `lighting`

Access: capability `lighting`.

Capability `lighting`: light sources the mod gives the world, like a torch in the player's hand (API 0.1.13). Nothing is relit: the game keeps the sources nearest the camera and the shaders raise the block light of what they reach, one level less per block of distance, before the lightmap.

### record `light`

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` | Where the source is, in the world. |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `level` | `u8` | 0 to 15, as block light. |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-lights` | `func(lights: list<light>)` | The mod's light sources; the game keeps them until the mod sets another list (an empty one removes its light) and drops them when the mod stops. Up to 256 sources; the game draws the 16 nearest the camera that can still light something in sight. Call it from `frame` or `tick`. |
| `get-block-emission` | `func(id: string) -> u8` | The light a block gives off (`lightEmission`), 0 to 15; 0 for an id that is not a block or an unknown one. |

## `debug`

Access: capability `debug`.

Capability `debug`: lines the mod adds to the debug screen (F3) (API 0.1.11).

| Function | Signature | What it does |
| --- | --- | --- |
| `set-lines` | `func(lines: list<string>)` | The mod's lines, in a group of their own; an empty list removes the group. At most 16 lines of 256 characters. |

## `command`

Access: capability `command`.

Capability `command`: vanilla commands run as the player, with the player's permissions, after the hook call returns (8 per call). A mod's own commands are not run this way.

| Function | Signature | What it does |
| --- | --- | --- |
| `run` | `func(line: string)` | `line` without its slash; its feedback goes to the chat as if typed. |

## `ui`

Access: capability `ui`.

Capability `ui`: widgets the game draws with its own menus' widgets, for a screen the mod owns and a settings page in Options > Mods. Layout and drawing are the game's: a mod says what its widgets hold, never where they sit or how they look, and the game clips everything it is handed.

### record `text-widget`

A line of text.

| Field | Type | |
| --- | --- | --- |
| `id` | `u32` | The mod's own name for the widget; `ui-events` calls echo it. |
| `text` | `string` | Up to 256 characters. |
| `color` | `u32` | 0xRRGGBB; 0 is the menus' white. |

### record `item-widget`

An item's icon and a name beside it, as recipe viewers show them.

| Field | Type | |
| --- | --- | --- |
| `id` | `u32` | The mod's own name for the widget. |
| `item` | `string` | `minecraft:diamond`, `<mod>:<path>`; an unknown id draws a "?". |
| `name` | `string` | The name shown beside the icon (empty: the item's own). |

### record `list-widget`

Rows the player picks one of, as an options page lists choices: a pick is `ui-events::select`, `selected` is the row to mark now.

| Field | Type | |
| --- | --- | --- |
| `id` | `u32` | The mod's own name for the widget; `ui-events::select` echoes it. |
| `rows` | `list<string>` | The rows' texts, at most 512, each up to 256 characters. |
| `selected` | `option<u32>` | The row to mark as the current choice, if any. |

### record `button-widget`

A thing on a mod's screen or settings page. A button as the options pages have them (200 × 20): a click is `ui-events::select(id, 0)`. Mods cycle its label for a choice ("Corner: Top Left").

| Field | Type | |
| --- | --- | --- |
| `id` | `u32` | The mod's own name for the widget. |
| `label` | `string` | Up to 256 characters. |

### variant `widget`

| Case | Payload | |
| --- | --- | --- |
| `text` | `text-widget` | Text the player reads. |
| `button` | `button-widget` | A button. |
| `item` | `item-widget` | An item's icon and name. |
| `list` | `list-widget` | Rows the player picks from. |

### record `screen`

A screen the mod owns: a title over widgets stacked down the middle, the Done button the game adds, and Esc to leave it.

| Field | Type | |
| --- | --- | --- |
| `title` | `string` | Its title, up to 256 characters. |
| `widgets` | `list<widget>` | Its widgets, in order, at most 64. |

| Function | Signature | What it does |
| --- | --- | --- |
| `open-screen` | `func(screen: screen)` | Show `screen` over the game; a second call while it is open replaces it. Only the last call of one hook counts. |
| `close-screen` | `func()` | Close the mod's screen when it is open (the mod hears no `closed` for this: it was its own doing). |
| `set-settings` | `func(widgets: list<widget>)` | The mod's page in Options > Mods, reached from a settings button on its row: the widgets it shows, an empty list for no page. The player opening the page is `ui-events::settings-opened`: push fresh widgets then; later calls update the page while it shows. |

## `ui-events`

Access: exported by the mod: the game calls it.

What the game calls on a mod that shows widgets (capability `ui`): what the player did with them.

| Function | Signature | What it does |
| --- | --- | --- |
| `select` | `func(id: u32, row: u32)` | A row of one of the mod's `list` widgets was picked: the widget's `id` and the row, on the screen or the settings page that showed it. |
| `closed` | `func()` | The player left the mod's screen (Esc or Done); not for a screen the mod closed itself or one that went away with the mod. |
| `settings-opened` | `func()` | The player opened the mod's settings page in Options > Mods: call `ui::set-settings` now to fill it with fresh widgets. |

## `controls`

Access: capability `controls`.

Capability `controls` (experimental: the manifest needs `experimental = ["controls"]`): the use button on items the mod claims, and the mod's own keys. Events wait for `poll`, at most 256.

### record `held-item`

The main hand's stack.

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:spyglass`, `<mod>:<path>`. |
| `count` | `u32` |  |
| `slot` | `u32` | The hotbar slot, 0 to 8. |

### record `item-use`

| Field | Type | |
| --- | --- | --- |
| `item` | `held-item` |  |
| `ticks` | `u32` | Game ticks the button was held (0 on `use-start`). |

### variant `event`

| Case | Payload | |
| --- | --- | --- |
| `use-start` | `item-use` | The use button went down with a claimed item in the main hand. The game does nothing else with the click: no placing, no eating, no swing. |
| `use-stop` | `item-use` | It came up again, or the item left the hand, or a screen opened. |
| `key-down` | `string` | A key `register-key` named went down or up while playing (no screen open). The game's own mapping on that key still runs. A screen opening lets every held key up. |
| `key-up` | `string` |  |
| `wheel` | `f32` | Mouse wheel steps while the mod holds the wheel (`claim-wheel`): positive away from the player, as the hotbar reads them. |

### record `mapping-state`

A mapping as the player has bound it.

| Field | Type | |
| --- | --- | --- |
| `label` | `string` | The bound key's name as Controls shows it: `W`, `Space`, `LMB`. |
| `down` | `bool` | Held down now (as of this frame for a mod with a `frame` hook, else this tick). |

| Function | Signature | What it does |
| --- | --- | --- |
| `claim-items` | `func(ids: list<string>)` | The items whose use the mod handles instead of the game, by id (at most 64). Replaces the previous list. |
| `register-key` | `func(name: string, label: string, key: string) -> result<_, string>` | A key of the mod's: `name` comes back in `key-down` and `key-up`, `label` says what it does, `key` is the key as options.txt writes it (`key.keyboard.r`), the default: the player can bind another one on the mod's page in Options > Mods, and that one wins. At most 16; an unknown key is an error. |
| `held` | `func() -> option<held-item>` | The main hand's stack, as of this tick; none when it is empty. |
| `poll` | `func() -> list<event>` | The events since the last call, oldest first. |
| `claim-wheel` | `func(on: bool)` | Takes the mouse wheel while playing (true) or gives it back: while taken, its steps are the mod's `wheel` events and the hotbar doesn't move. The game gives it back when the mod stops. |
| `mapping` | `func(name: string) -> option<mapping-state>` | A movement or click mapping as the player has bound it: `key.forward`, `key.left`, `key.back`, `key.right`, `key.jump`, `key.sneak`, `key.sprint`, `key.attack` or `key.use`. None for another name. |
| `clicks-per-second` | `func() -> tuple<u32, u32>` | Clicks of the attack and use buttons in the last second. |

## `render`

Access: capability `render`.

Capability `render` (experimental: the manifest needs `experimental = ["render"]`): shapes the game draws for the mod with its own renderer, kept until the mod replaces them (no call per frame). A mod never supplies shader code.

### enum `corner`

A corner of the screen, as `hud`'s panels use them.

| Case | |
| --- | --- |
| `top-left` |  |
| `top-right` |  |
| `bottom-left` |  |
| `bottom-right` |  |

### record `rect`

A rectangle in GUI pixels: the top left corner and the size.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |
| `w` | `f32` |  |
| `h` | `f32` |  |

### record `point`

A point in GUI pixels from the screen's top left.

| Field | Type | |
| --- | --- | --- |
| `x` | `f32` |  |
| `y` | `f32` |  |

### record `fill-shape`

| Field | Type | |
| --- | --- | --- |
| `rect` | `rect` |  |
| `color` | `u32` | 0xAARRGGBB. |

### record `quad-shape`

Any four-cornered shape (a wheel's segment, an arrow): the corners in order round it.

| Field | Type | |
| --- | --- | --- |
| `corners` | `tuple<point, point, point, point>` |  |
| `color` | `u32` | 0xAARRGGBB. |

### record `sprite-shape`

| Field | Type | |
| --- | --- | --- |
| `sprite` | `string` | One of the game's GUI sprites, as `gui/sprites/` names them (`hud/heart/full`, `widget/button`), the resource packs' versions included. An unknown one draws nothing. |
| `rect` | `rect` |  |
| `color` | `u32` | A tint, 0xAARRGGBB: 0xFFFFFFFF draws it as it is. |

### record `item-shape`

| Field | Type | |
| --- | --- | --- |
| `item` | `string` | `minecraft:diamond`, `<mod>:<path>`; an unknown one draws nothing. |
| `at` | `point` | The icon's top left. |
| `scale` | `f32` | 1.0 is the hotbar's 16 × 16. |

### record `text-shape`

| Field | Type | |
| --- | --- | --- |
| `text` | `string` | Up to 256 characters, one line. |
| `at` | `point` |  |
| `color` | `u32` | 0xRRGGBB, as `hud`'s lines. |
| `centered` | `bool` | Centred on `at.x` instead of starting there. |
| `shadow` | `bool` |  |

### variant `shape`

| Case | Payload | |
| --- | --- | --- |
| `fill` | `fill-shape` |  |
| `quad` | `quad-shape` |  |
| `sprite` | `sprite-shape` |  |
| `item` | `item-shape` |  |
| `text` | `text-shape` |  |

### record `frame-rate`

| Field | Type | |
| --- | --- | --- |
| `fps` | `u32` | Frames drawn in the last second. |
| `min` | `u32` | The slowest frame's rate, and the 1% low (the 99th percentile frame time's rate), over the last 5 seconds. |
| `low` | `u32` |  |

### record `zoom`

| Field | Type | |
| --- | --- | --- |
| `factor` | `f32` | The field of view is divided by it: 1 is none, at most 10. |
| `smooth` | `bool` | Ease in and out through the game's field-of-view easing, as the spyglass does, instead of at once. |
| `turn` | `f32` | Mouse turning is multiplied by it while zoomed: 1 keeps the player's sensitivity, at least 0.05. |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-hud` | `func(shapes: list<shape>)` | Replaces the mod's HUD shapes: up to 1024, drawn every frame in order over the HUD (after the hotbar, under chat and screens), hidden with the HUD (F1). An empty list removes them. |
| `gui-size` | `func() -> tuple<f32, f32>` | The screen's size in GUI pixels (width, height), as of this tick. |
| `shake` | `func(strength: f32, seconds: f32)` | Shakes the camera: a jolt of `strength` (0 to 1, 1 the hardest) that fades out over `seconds` (at most 10). Jolts add up to 1. The view turns, the player doesn't, and the player's Distortion Effects option scales it (0 turns it off). |
| `set-corner` | `func(corner: corner, width: f32, height: f32, shapes: list<shape>)` | Replaces the mod's corner panel: shapes in GUI pixels from the top left of a `width` × `height` box that the game places in `corner`, stacked with the game's overlays and the other mods' panels (under the status effects at the top right), as `hud`'s panels are. |
| `clear-corner` | `func()` | Removes the mod's corner panel. |
| `text-width` | `func(text: string) -> f32` | The width of one line of text in GUI pixels, in the HUD's font: exact for Latin letters (U+0000 to U+024F), 6 per other character. |
| `frame-stats` | `func() -> frame-rate` | The game's frame counter, recomputed twice a second. |
| `set-zoom` | `func(zoom: zoom)` | Zooms the camera until the next call (`factor` 1 ends it), only while playing: a screen opening or the spyglass's own view (which wins) leave the camera vanilla's. With several mods, the strongest zoom counts. |

## `events`

Access: exported by the mod: the game calls it.

What the game calls. Only the hooks `init` asks for are ever called.

### record `registration`

| Field | Type | |
| --- | --- | --- |
| `commands` | `list<string>` | Chat commands, without the slash. A name vanilla already uses is refused (vanilla's command runs). |
| `tick` | `bool` | Call `tick` every game tick (20 per second while playing). |
| `frame` | `bool` | Call `frame` every rendered frame. Budget: 0.1 ms. |
| `chat` | `bool` | Call `chat` for each chat line the player receives. |

| Function | Signature | What it does |
| --- | --- | --- |
| `init` | `func() -> result<registration, string>` | Once, after loading. An error keeps the mod off and is shown. |
| `tick` | `func(tick: u64)` | Each game tick while in a world, with the game's tick counter. |
| `frame` | `func(seconds: f32)` | Each rendered frame, with the seconds since the last one. |
| `command` | `func(name: string, args: string) -> result<option<string>, string>` | `/name args`: ok(some(text)) is shown as feedback, err(text) in red. |
| `chat` | `func(text: string)` | Each chat line the player receives, as plain text. |

## `exchange`

Access: capability `exchange`.

Capability `exchange` (API 0.1.27): data one mod publishes and others read, so mods can work together (an enchantment mod gives its descriptions to a recipe browser). The data lives while the game runs.

| Function | Signature | What it does |
| --- | --- | --- |
| `publish` | `func(key: string, data: list<u8>) -> result<_, string>` | Publishes bytes under a key of this mod (`[a-z0-9_.-]`, up to 64 characters), replacing the earlier ones. Up to 64 keys of 1 MiB. |
| `withdraw` | `func(key: string)` | Removes a key of this mod. |
| `read` | `func(mod-id: string, key: string) -> option<list<u8>>` | The bytes another mod (or this one) published under the key since the game started; none when there are none. |
| `list-keys` | `func(mod-id: string) -> list<string>` | The keys a mod published, sorted. |
