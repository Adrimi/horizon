# Horizon extras

Optional conveniences built into the client: fullbright, zoom, a minimap, connected
textures, an FPS counter, a keystroke overlay, auto-reconnect, Mouse Tweaks-style
inventory controls and inventory sorting. **Every one is off by default**: a fresh
install looks and behaves exactly like vanilla 26.3, and turning an extra off
again restores vanilla's behaviour completely.

## Where they live

| Part | Code |
|---|---|
| Settings, parsing and saving | `crates/horizon-client/src/extras.rs` |
| Zoom curve, FPS and click counters | `extras.rs` (pure logic, unit tested) |
| HUD drawing (FPS, keystrokes, minimap frame) | `crates/horizon-client/src/extras_hud.rs` |
| Minimap colours, tiles and image | `crates/horizon-client/src/minimap.rs` |
| Dynamic lights: source levels and packing | `crates/horizon-client/src/dynamic_lights.rs` (unit tested); sources in `game/dynamic_lights.rs`, shaders in `terrain.wgsl` and `entity.wgsl` |
| Connected textures: rules, the 47-tile table and tile choice | `crates/horizon-client/src/ctm.rs` (unit tested); mesher hook in `mesher.rs` |
| Falling leaves: block ticks, spawn odds and colours | `crates/horizon-client/src/animate_tick.rs` (unit tested) |
| Leaf particle: texture, fall, landing and lingering | `crates/horizon-client/src/particles.rs` (unit tested) |
| Particle rain: the weather of a column, the density and the storm's haze | `crates/horizon-client/src/rain.rs` (unit tested) |
| Weather particles: textures, fall, splash and ripples | `crates/horizon-client/src/particles.rs` (unit tested) |
| Reconnect policy and backoff | `crates/horizon-client/src/reconnect.rs` |
| Game glue (input hooks, per-frame update) | `crates/horizon-client/src/game/extras.rs` |
| Mouse Tweaks-style controls, sort buttons | `crates/horizon-client/src/game/tweaks.rs` |
| Sort order and its clicks | `crates/horizon-client/src/inv_sort.rs` (pure, unit tested) |
| Item highlighter: what is new and the mark | `crates/horizon-client/src/item_highlighter.rs` (unit tested); the watch and the drawing in `game/item_highlighter.rs` |
| The settings page | `options_horizon.rs`: the **Extras** section of Horizon's Options |

`game.rs` only gets one-line hooks (key, wheel, mouse, frame, HUD), so the
extras stay out of the way of other input and options work.

## Options storage

`horizon-extras.txt` beside `options.txt`, in the same `key:value` format as
`horizon-graphics.txt`. `options.txt` is untouched, so a vanilla client can share
the folder. Unknown or broken lines fall back to the defaults.

The zoom key is a Horizon setting stored in that file (`zoomKey:key.keyboard.z`)
and bound on the Extras page, not in vanilla's Key Binds screen: Horizon has no
Horizon category in Controls, and adding one would put a non-vanilla mapping into
`options.txt`.

## The extras

### Fullbright (`fullbright`, default off)
- Displays blocks and entities with a fully bright lightmap, including in dark caves.
- Turning it off restores the normal lightmap using the current brightness,
  time of day and effects. The brightness slider keeps its normal range.

### Dynamic lights (`dynamicLights`: off, fast or fancy; default off)
- A held torch or lantern, a glowing block item in hand (glowstone, sea lantern,
  lava bucket...), a glowing item dropped on the ground, a mob that glows by kind
  (blaze, magma cube, glow squid, allay, vex) and a burning mob or player light
  the blocks and entities around them while they move. A held item uses its
  block's `lightEmission`, else a short table of glowing items (`lava_bucket` 15,
  `glow_berries` 14, `blaze_rod` 10, ...). A burning mob is a full 15.
- Nothing is relit in the world: each frame the client keeps the 16 nearest
  sources within the render distance and packs them camera relative. `terrain.wgsl`
  and `entity.wgsl` raise each vertex's block light to the brightest
  `level - distance` (one level per block, as block light spreads) before they
  sample the existing lightmap, so sky light, fog, biome tint and everything else
  stay vanilla's and no section is remeshed when a light moves.
- **Fast** gathers the sources again every 4th frame (they stay put in between);
  **Fancy** every frame. Both pack and upload the list every frame. Off leaves
  the count 0, so the shaders' loop does nothing and the picture is vanilla's.
- Works in the offscreen path (background test mode) as any other frame.
- Limitation: light is interpolated from the terrain and entity vertices (one
  block apart; more on distant terrain tiles), not per pixel. The list holds 16
  sources, nearest first, and every terrain and entity vertex runs the loop for
  each of them.

### Zoom (`zoom`, default off)
- Hold the zoom key (default **Z**, unused by every vanilla 26.3 key mapping; C
  is `key.saveToolbarActivator`) to zoom. The key does nothing while zoom is off.
- It zooms only while playing: in chat, signs, books, text fields and every
  other screen the key is typed or handled as usual. The key is never taken
  from vanilla's mappings: one bound to the same key still acts (a key bound
  twice triggers both, as vanilla's do), and the Extras page marks the zoom key
  "Also bound" and names the mapping.
- Starts at the default level (4x, settable 2x to 10x); the scroll wheel changes
  it while the key is held (and does not move the hotbar then).
- **Smooth** (default on) goes through vanilla's own FOV easing: the zoom divides
  the field-of-view modifier target `Camera.tickFov` eases toward, the path the
  spyglass uses (`AbstractClientPlayer.getFieldOfViewModifier` returns 0.1 while
  scoping). The modifier keeps vanilla's 0.1 floor, so 10x is the deepest zoom.
  Off applies the zoom to the final FOV with no easing.
- While scoping with a spyglass the zoom does nothing: the spyglass view and its
  overlay stay exactly vanilla's.
- **Lower sensitivity** (default on) divides mouse turning by the zoom factor.
- With zoom off (or the key up) the FOV target and mouse are vanilla's, bit for
  bit; the FOV harness covers this.

### FPS counter (`fps`, default off)
- A small "N fps" line in a corner (default top left), optionally followed by
  the minimum and the 1% low over the last 5 seconds.
- Separate from the Advanced graphics "Overlay" (the frame-time graph).

### Keystrokes (`keystrokes`, default off)
- W A S D, Space, LMB and RMB keys that light up while held, with clicks per
  second under the mouse buttons (clicks in the last second). The letters follow
  the bound keys.

### Minimap (`minimap`, default off)
- A corner map (top right by default), square or round, small/medium/large,
  zoom 0.5 to 4 blocks per pixel, rotating with the player or north up, with the
  player arrow (vanilla's map decoration sprite).
- Colours are vanilla's `MapColor` per block, as map items use, with the map
  item's height shading and water-depth dithering.
- Underground, in the Nether and under any roof, it maps the floor around the
  player's height instead of the top block, so caves read sensibly.
- Cost: one cached 16x16 tile per loaded chunk (packed map colour and height).
  Tiles are filled nearest first and refreshed when blocks change, within a time
  budget of 0.3 ms a frame; composing the 128x128 image is a few thousand
  lookups, done only when the view moved or a tile changed, and uploaded into a
  dedicated atlas slot. With the minimap off nothing runs.

### Connected textures (`connectedTextures`, default off)
- Reads OptiFine-format CTM from the enabled resource packs:
  `assets/<namespace>/optifine/ctm/**/*.properties`, with `tiles` ranges and
  names, `matchBlocks` (block and property filters) or `matchTiles`, `faces`,
  `connect=block|tile|state`, `width`/`height` and `weights`. A file without a
  match takes it from its name (`block_glass` names a block, `glass` a tile).
- Methods: `ctm` (the real 47-tile template, with the inner corners of sides
  that meet but whose diagonal does not), `ctm_compact` (5 tiles, drawn as four
  quarters of a face), `horizontal`, `vertical`, `top`, `random` and `repeat`.
- A face connects only across its own plane: left, down, right, up and the four
  diagonals of that face, so a wall of glass joins seamlessly while its outer
  border keeps the frame, exactly as OptiFine's and Continuity's tables do.
- Tiles are loaded from beside the properties file, `~/` or
  `assets/<namespace>/textures/`, and packed into the block atlas with the
  pack's other sprites; changing the pack or the option remeshes the world.
- With no CTM pack, or with the option off, sections mesh exactly as vanilla's
  (the mesher returns before any lookups), so the default stays byte-identical.
- Not implemented: `innerSeams`, `orient`, `ctm.N`, `randomLoops`, `symmetry`
  and the biome/height/size/health conditions; unsupported keys are ignored or
  the file is skipped with a log line, never fatal.

### Falling leaves (`fallingLeaves`, default off; `fallingLeavesAmount` 2, 4, 8 or 16)
- Leaf blocks with air under them now and then drop a leaf particle, in the
  same random block ticks vanilla's `animateTick` samples around the player
  (16 blocks, 32 when the client's particle distance allows): oak, birch,
  jungle, acacia, dark oak and mangrove, and spruce, azalea and flowering
  azalea, which vanilla's leaves never drop.
- The leaf follows vanilla's tinted-leaf motion: it sinks slowly, sways and
  turns on its way down. Where it lands it lies still for 3 seconds and then
  vanishes, instead of disappearing on the tick it touches the ground.
- Colour: the block's foliage colour at its position (the biome tint Oak,
  Birch etc. use), or the fixed evergreen colour for spruce and the azaleas'
  own average colours. Cherry and pale oak are left to vanilla's own petals
  and pale oak leaves, so nothing is drawn twice.
- `fallingLeavesAmount` multiplies vanilla's chance (1 leaf in 100 block ticks
  per oak-family leaf; the evergreens get their first leaf from Horizon). The
  game's Particles setting scales it: **Decreased** a third, **Minimal** none.
  With the extra off, or on Minimal, no new particle is spawned at all.
- The leaf uses its own 16x16 texture, generated in code and packed into the HUD
  atlas (`particles::horizon_leaf_texture`), so it is not a copy of a vanilla
  particle texture; it is white and the particle's tint gives it the colour.

### Particle rain (`particleRain`, default off; `particleRainAmount` 1, 2, 4 or 8)
- The weather becomes particles on top of vanilla's rain columns: drops of
  rain fall and splash where they land, snow comes down as slow flakes that
  drift with the wind, and in the deserts and the badlands, where vanilla has
  no rain at all, a storm blows grains of sand along the ground.
- A drop that lands on the ground leaves a ring of splashed water and throws
  a few droplets up; a drop that lands on water leaves a ring spreading over
  the surface. A heavy rain or a thunderstorm puts a light haze in the
  distance, and a sandstorm a sand-coloured one.
- Only where it really falls: the drops are spawned in the same columns
  vanilla draws its rain in (the height map of the terrain decides what the
  sky reaches), in the biomes where it rains, and snow only where it is cold
  enough to snow.
- The density follows the rain level (a thunderstorm rains harder), the
  game's Particles setting (**Decreased** a third, **Minimal** none) and
  `particleRainAmount`, which multiplies Horizon's own density.
- The rain, the snow, the sand and the rings use their own textures,
  generated in code and packed into the HUD atlas
  (`particles::horizon_rain_texture` and friends), not copies of a mod's.
- With the extra off, the weather is vanilla's: the same columns, the same
  splashes, no extra particles.

### Auto-reconnect (`autoReconnect`, default off)
- On the "Connection lost" screen after a multiplayer connection drops, a
  countdown "Reconnecting in N s" with a Cancel button, then a new connection
  to the same address.
- Backoff 5, 10, 20, 40, 60 s; at most 5 attempts, then it stops. The count
  only resets after staying connected for a minute, so a server that drops
  players at once never causes an endless loop.
- Never after a kick, a ban, a whitelist or version refusal, a duplicate login,
  an online-mode refusal (no account signed in), or a session refusal from the
  account's login (invalid session, banned from playing online, multiplayer
  disabled, the server failing to verify the username). Unreachable session
  servers, on either side, are retried with the same backoff.
- Online-mode servers: the retry joins with the account the game signed in
  with, held in memory for the whole run (`auth::current()`); nothing is
  written to disk.

### Mouse Tweaks controls (`mouseTweaks`, default off; `mouseTweaksWheel`, default on)
In the player inventory and chest-like containers (chests, barrels, ender
chests, shulker boxes, hoppers, dispensers), in singleplayer and on servers:
- Right-drag holding a stack: one item goes into each slot entered, at once
  (slots holding another item, or full, are skipped).
- Left-drag holding a stack: each slot entered that holds the same item is
  gathered into the hand, when it all fits. This takes over only while
  vanilla's drag covers one slot, so a left-drag across empty slots still
  splits the stack as vanilla does.
- Shift + left-drag: every slot entered is quick-moved (with a stack in hand,
  only the slots of its item).
- The wheel over a slot, with an empty hand (`mouseTweaksWheel`): down moves
  one item to the other side (container and inventory, or hotbar and main
  inventory: to a stack of it with room, else the first empty slot, as
  quick-move picks), up brings one of the same item back.

Every action is ordinary slot clicks (`ContainerClick` on a server), so a
vanilla server sees nothing unusual. With the extra off, the hooks return at
once and the screens click exactly as vanilla's (`tools/test_menu_click_parity.py`
and the container screen tests are unchanged).

### Sorting (`sort`, default off; `sortKey`, unbound)
- A small button right of the container's title (chests, barrels, ender
  chests, shulker boxes) and one right of the "Inventory" line (the main
  inventory; the hotbar is left alone). The key, when bound on the Extras
  page, sorts the part under the cursor (the container when over no slot).
- Partial stacks of one item merge, then the stacks go in creative tab order
  (the search tab's), then by name; full stacks before the partial one,
  empty slots last. It needs an empty hand.
- Done with ordinary left clicks planned by `inv_sort::plan`, so a vanilla
  server accepts it; a unit test replays the planned clicks through the menu
  engine on random chests.

### Item and recipe browser (`recipeBrowser`, default off)
- Beside the inventory, the crafting table and container screens (chests,
  barrels...; not the stations with screens of their own): a searchable grid of
  every item in the creative search tab's order, in pages, with a page number,
  arrows and the wheel. `@namespace` in the search keeps a namespace (a prefix,
  as JEI's); a right click on the search box clears it.
- R over an item (in the panel, an inventory slot or a recipe) shows the
  recipes that make it: crafting (shaped and shapeless), smelting, blasting,
  smoking, campfire cooking, stonecutting, smithing (transforms and trims). U
  shows the recipes that use it. The frame draws the station, the ingredients
  with arrows and the result; Left/Right (or its arrows) step through the
  recipes, a click on an ingredient follows it, Escape/Backspace goes back.
- A plain click in the panel shows an item's recipes, a right click its uses.
  In creative mode a click gives the stack to the inventory instead (a right
  click one item); the stack goes straight to the inventory, not to the cursor,
  because the click's end would drop what the cursor holds.
- The panel has a dark background, as JEI's, and the recipes' frame is dark
  too, with the station's icon, the ingredients and the result.
- The recipes are the local data's (vanilla's and content mods'), built once
  when first needed from `Recipes` and `StationRecipes`; a 26.3 server sends a
  client only the recipe book's displays, not the recipes. Special (code)
  crafting recipes are not shown. Code: `item_browser.rs` (the lookup, the
  query and the pages) and `game/item_browser.rs` (panel, frame, keys, clicks).

### Item highlighter (`itemHighlighter`, default off)
- A slot that holds a newly gained item is marked: a soft golden frame around
  the slot and a dot in its top right corner, in the hotbar and in the
  inventory screen (the player's slots, the armour and the off hand). The frame
  pulses slowly.
- The mark is gone when the player looks at the slot (the cursor over it in the
  inventory screen) or picks it (the hotbar's selection changes to that slot).
- **What counts as new:** the number of that item the player has grew. The
  stack on the cursor counts too, so a pickup is new, and a stack the player
  had already grows into a bigger one, but a stack the player moves between
  slots (also through the cursor), splits in two or swaps with another one is
  not: the number of the item did not change. Items the player crafted are not
  new either.
- The items a world (or a server) comes with are not new: the highlighter
  waits two seconds after the world is loaded and takes the inventory as it is
  then. The state is not kept between sessions.
- Limitation: the count is by item id and damage, so two potions of a kind are
  one item for the count; a crafted item is remembered for two seconds, so a
  pickup of the same item in that time is not marked either.
- Code: `item_highlighter.rs` (the comparison and the marks, unit tested) and
  `game/item_highlighter.rs` (the watch, the hotbar and the inventory screen).

## Shared rules

- All extras are hidden with F1 and drawn in the HUD layer: under screens, over
  the world, like vanilla's HUD. Extras in the top left corner give way to the
  debug screen (F3), which shows the FPS anyway.
- Corners stack: extras placed in the same corner stack downward (top corners)
  or upward (bottom corners) in the order minimap, FPS, keystrokes. The top
  right corner starts below the status effect icons.

## Perf budget

Off: a flag check per frame per extra (no allocations, no world access).
On: FPS and keystrokes are a few quads; the minimap stays under 0.3 ms of CPU
per frame (tile budget) plus a 64 KiB texture upload when its image changes.
