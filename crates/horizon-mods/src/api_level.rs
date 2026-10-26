//! Mod API levels: which of the WIT contract's functions a Horizon has
//! (`docs/api/stability.md`). Every function the game offers mods has the
//! level it arrived at; a Horizon serves every level up to its [`LEVEL`].
//! A mod needs the level of the newest function it imports, so a mod that
//! calls a function this Horizon doesn't have is refused with "update
//! Horizon" instead of a linking error, and the registry lists each
//! version's level so the launcher can tell before it installs.

/// The newest level this Horizon serves.
pub const LEVEL: u32 = 2;

/// Every function the game offers mods, as `interface.function`, with its
/// level. Level 0 is the contract of Horizon 0.2.67 to 0.2.69. A new
/// function takes the next level and raises [`LEVEL`] (the `api_reference`
/// test fails until it is listed here).
pub const FUNCTIONS: [(&str, u32); 218] = [
    ("log.write", 0),
    ("game.get-player", 0),
    ("game.get-clock", 0),
    ("chat.show", 0),
    ("hud.set-panel", 0),
    ("hud.clear", 0),
    ("storage.get", 0),
    ("storage.set", 0),
    ("storage.delete", 0),
    ("command.run", 0),
    ("ui.open-screen", 0),
    ("ui.close-screen", 0),
    ("ui.set-settings", 0),
    ("input.poll", 2),
    ("render.set-hud", 0),
    ("render.gui-size", 0),
    ("render.shake", 0),
    ("render.set-corner", 0),
    ("render.clear-corner", 0),
    ("render.text-width", 0),
    ("render.frame-stats", 0),
    ("render.set-zoom", 0),
    ("render.set-minimap", 1),
    ("server-players.online", 1),
    ("server-players.tell", 1),
    ("server-players.broadcast", 1),
    ("game.get-input-state", 2),
    ("game.get-food-stats", 2),
    ("game.get-game-mode", 2),
    ("game.get-player-tags", 2),
    ("game.get-cloud-height", 2),
    ("game.get-viewport", 2),
    ("game.get-server-view-distance", 2),
    ("game.add-exhaustion", 2),
    ("game.set-break-speed-modifier", 2),
    ("game.get-seed", 2),
    ("game.get-fov", 2),
    ("game.get-session", 2),
    ("game.get-health", 2),
    ("game.get-effects", 2),
    ("game.get-difficulty", 2),
    ("game.get-main-hand", 2),
    ("game.get-mod-name", 2),
    ("game.get-translation", 2),
    ("game.narrate", 2),
    ("game.get-narration-mode", 2),
    ("chat.get-messages", 2),
    ("chat-decorate.set-picture", 2),
    ("chat-decorate.set-decoration", 2),
    ("players.get-players", 2),
    ("light-events.poll", 2),
    ("input.set-wheel-captured", 2),
    ("audio.play", 2),
    ("audio.register", 2),
    ("audio.register-file", 2),
    ("hud.show-action-bar", 2),
    ("hud.get-bars", 2),
    ("hud.get-boss-bars", 2),
    ("hud.set-boss-bars-hidden", 2),
    ("target.get-block", 2),
    ("target.get-breaking", 2),
    ("target.get-breaking-progress", 2),
    ("target.get-block-name", 2),
    ("target.get-entity", 2),
    ("target.get-entity-armor", 2),
    ("target.get-entity-absorption", 2),
    ("target.get-block-face", 2),
    ("target.get-entity-effects", 2),
    ("target.get-block-ex", 2),
    ("target.get-block-face-ex", 2),
    ("target.get-entity-ex", 2),
    ("target.get-fluid", 2),
    ("target.get-block-properties", 2),
    ("target.get-block-loot-table", 2),
    ("target.get-fluid-ex", 2),
    ("inventory.get-hotbar", 2),
    ("inventory.get-selected", 2),
    ("inventory.get-food", 2),
    ("inventory.item-tags", 2),
    ("inventory.get-tags", 2),
    ("inventory.get-inventory", 2),
    ("inventory.get-crafting", 2),
    ("inventory.give", 2),
    ("inventory.get-food-of", 2),
    ("inventory.get-harmful-food", 2),
    ("inventory.get-offhand", 2),
    ("inventory.get-tools", 2),
    ("inventory.get-slots", 2),
    ("inventory.move-item", 2),
    ("hotbar.select", 2),
    ("keys.bind", 2),
    ("keys.get", 2),
    ("keys.poll", 2),
    ("keys.is-down", 2),
    ("storage.list-keys", 2),
    ("storage.read", 2),
    ("storage.write", 2),
    ("storage.remove", 2),
    ("storage.files", 2),
    ("screen.paint", 2),
    ("screen.get-text-width", 2),
    ("screen.open-panel", 2),
    ("screen.set-panel-area", 2),
    ("screen.get-game-screen", 2),
    ("screen.get-station", 2),
    ("screen.get-menu-origin", 2),
    ("screen.get-item-at", 2),
    ("screen.get-cursor", 2),
    ("screen.get-modifiers", 2),
    ("screen.open", 2),
    ("screen.get-size", 2),
    ("screen.draw", 2),
    ("screen.get-input", 2),
    ("screen.close", 2),
    ("screen.upload-texture", 2),
    ("screen.get-max-texture-side", 2),
    ("screen.set-texture-filter", 2),
    ("world.get-block", 2),
    ("world.random-ticked-blocks", 2),
    ("world.watch-random-ticks", 2),
    ("world.watch-random-tick-blocks", 2),
    ("world.removed-blocks", 2),
    ("world.broken-blocks", 2),
    ("world.break-block", 2),
    ("world.break-block-with-held", 2),
    ("world.damage-held", 2),
    ("world.get-chunk-surface", 2),
    ("world.get-chunk-details", 2),
    ("world.get-biome-names", 2),
    ("world.get-light-level", 2),
    ("world.get-biome-fog", 2),
    ("world.run-command", 2),
    ("world.merge-drops-at", 2),
    ("world.fall-block", 2),
    ("world.set-block", 2),
    ("world.spawn-item", 2),
    ("world.set-weather-passthrough", 2),
    ("world.get-map-colors", 2),
    ("world.get-chunk-columns", 2),
    ("world.get-biome", 2),
    ("world.get-surface", 2),
    ("world.is-solid", 2),
    ("world.get-container", 2),
    ("world.get-block-entity", 2),
    ("items.list", 2),
    ("items.get", 2),
    ("items.get-name-color", 2),
    ("items.get-tooltip", 2),
    ("recipes.list", 2),
    ("recipes.get-for", 2),
    ("recipes.get-uses", 2),
    ("weather.get-weather", 2),
    ("weather.set-default-weather", 2),
    ("weather.set-default-splashing", 2),
    ("settings.get-particles", 2),
    ("particles.define-sprite", 2),
    ("particles.spawn", 2),
    ("particles.spawn-rotated", 2),
    ("particles.spawn-styled", 2),
    ("particles.get-particles", 2),
    ("particles.remove", 2),
    ("particles.landed", 2),
    ("particles.get-game-sprite", 2),
    ("particles.set-suppressed", 2),
    ("particles.register-command-particle", 2),
    ("atmosphere.set-haze", 2),
    ("overlay.paint", 2),
    ("overlay.clear", 2),
    ("overlay.paint-world", 2),
    ("overlay.clear-world", 2),
    ("overlay.get-size", 2),
    ("overlay.get-text-width", 2),
    ("overlay.upload-texture", 2),
    ("overlay.get-hud-state", 2),
    ("overlay.is-tab-list-open", 2),
    ("entity.nearby", 2),
    ("entity.remove", 2),
    ("entity.set-xp", 2),
    ("entity.get-face", 2),
    ("entity.get-detail", 2),
    ("entity.get-types", 2),
    ("entity.is-invisible", 2),
    ("entity.get-extra", 2),
    ("entity.get-loot-table", 2),
    ("entity.get-width", 2),
    ("entity.is-frozen", 2),
    ("slots.set-marks", 2),
    ("slots.set-screen-marks", 2),
    ("slots.set-hud-marks", 2),
    ("slots.upload-texture", 2),
    ("slots.get-hovered-slot", 2),
    ("tooltip.get-hovered", 2),
    ("tooltip.set-content", 2),
    ("tooltip.get-hovered-enchantments", 2),
    ("tooltip.set-lines", 2),
    ("tooltip.get-hovered-stack", 2),
    ("tooltip.set-hovered-tooltip", 2),
    ("resources.namespaces", 2),
    ("resources.list-files", 2),
    ("resources.read-file", 2),
    ("resources.file-origin", 2),
    ("texture-rules.upload-sprite", 2),
    ("texture-rules.register-rule", 2),
    ("texture-rules.clear-rules", 2),
    ("lighting.set-lights", 2),
    ("lighting.get-block-emission", 2),
    ("debug.set-lines", 2),
    ("controls.claim-items", 0),
    ("controls.register-key", 0),
    ("controls.held", 0),
    ("controls.poll", 0),
    ("controls.claim-wheel", 0),
    ("controls.mapping", 0),
    ("controls.clicks-per-second", 0),
    ("exchange.publish", 2),
    ("exchange.withdraw", 2),
    ("exchange.read", 2),
    ("exchange.list-keys", 2),
];

/// What the game's program carries so the launcher can read its level
/// without running it: `horizon-mod-api-level:0001;` (four digits). A
/// program from before the mark serves level 0.
pub const MARK: [u8; 27] = {
    let mut mark = *b"horizon-mod-api-level:0000;";
    let mut level = LEVEL;
    let mut i = 25;
    while level > 0 {
        mark[i] = b'0' + (level % 10) as u8;
        level /= 10;
        i -= 1;
    }
    mark
};

/// The level a program's bytes carry in their [`MARK`], if any.
pub fn read_mark(program: &[u8]) -> Option<u32> {
    let prefix = &MARK[..22];
    program
        .windows(MARK.len())
        .filter(|w| w.starts_with(prefix) && w[26] == b';')
        .find_map(|w| std::str::from_utf8(&w[22..26]).ok()?.parse().ok())
}

/// A function's level; None for one this Horizon doesn't have.
pub fn level_of(function: &str) -> Option<u32> {
    FUNCTIONS
        .iter()
        .find(|(name, _)| *name == function)
        .map(|&(_, level)| level)
}

/// The level a mod needs, from the functions it imports (as [`imports`]
/// names them): its newest function's. A function this Horizon doesn't
/// know comes from a newer one, so it needs more than [`LEVEL`]; that
/// function is returned with it, for the message.
pub fn needed<'a>(imports: impl IntoIterator<Item = &'a str>) -> (u32, Option<&'a str>) {
    let mut needed = (0, None);
    for name in imports {
        match level_of(name) {
            Some(level) if level > needed.0 => needed = (level, Some(name)),
            Some(_) => {}
            None => return (LEVEL + 1, Some(name)),
        }
    }
    needed
}

/// Why this Horizon can't run a mod that needs `level`, or None when it can.
pub fn too_new(level: u32, function: Option<&str>) -> Option<String> {
    (level > LEVEL).then(|| {
        let uses = function.map_or(String::new(), |f| format!(" (it uses {f})"));
        format!("made for a newer Horizon{uses}: update Horizon to run it")
    })
}

/// The functions a mod's component imports from Horizon (`horizon:mods/*`),
/// as `interface.function`, in the component's order.
#[cfg(any(feature = "runtime", feature = "index"))]
pub fn imports(wasm: &[u8]) -> Result<Vec<String>, String> {
    use wasmparser::{
        Parser, Payload, Validator, WasmFeatures, component_types::ComponentEntityType,
    };
    let types = Validator::new_with_features(WasmFeatures::all())
        .validate_all(wasm)
        .map_err(|e| format!("not a component: {e}"))?;
    let types = types.as_ref();
    let mut out = Vec::new();
    for payload in Parser::new(0).parse_all(wasm) {
        let Payload::ComponentImportSection(section) = payload.map_err(|e| e.to_string())? else {
            continue;
        };
        for import in section {
            let name = import.map_err(|e| e.to_string())?.name.name;
            // `horizon:mods/render@0.1.0` → `render`.
            let Some(interface) = name.strip_prefix("horizon:mods/") else {
                continue;
            };
            let interface = interface.split('@').next().unwrap_or(interface);
            let Some(item) = types.component_item_for_import(name) else {
                continue;
            };
            if let ComponentEntityType::Instance(id) = item.ty {
                for (function, export) in &types[id].exports {
                    if matches!(export.ty, ComponentEntityType::Func(_)) {
                        out.push(format!("{interface}.{function}"));
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_listed_once_and_up_to_this_horizons() {
        for (i, (name, level)) in FUNCTIONS.iter().enumerate() {
            assert!(*level <= LEVEL, "{name}");
            assert!(
                FUNCTIONS[..i].iter().all(|(other, _)| other != name),
                "{name} twice"
            );
        }
        assert!(FUNCTIONS.iter().any(|&(_, level)| level == LEVEL));
    }

    #[test]
    fn the_mark_carries_the_level() {
        assert_eq!(read_mark(&MARK), Some(LEVEL));
        let mut program = b"\0\x7fELF horizon-mod-api-level: horizon-mod-api-level:".to_vec();
        program.extend_from_slice(&MARK);
        program.extend_from_slice(b"\0rest");
        assert_eq!(read_mark(&program), Some(LEVEL));
        assert_eq!(read_mark(b"horizon-mod-api-level:12;"), None);
        assert_eq!(read_mark(b"no mark"), None);
    }

    #[test]
    fn a_mod_needs_its_newest_function() {
        assert_eq!(needed(["log.write", "hud.set-panel"]), (0, None));
        assert_eq!(
            needed(["log.write", "render.set-minimap", "hud.clear"]),
            (1, Some("render.set-minimap"))
        );
        assert_eq!(
            needed(["render.set-minimap", "render.set-sky"]),
            (LEVEL + 1, Some("render.set-sky"))
        );
        assert_eq!(too_new(LEVEL, None), None);
        assert_eq!(
            too_new(LEVEL + 1, Some("render.set-sky")).as_deref(),
            Some("made for a newer Horizon (it uses render.set-sky): update Horizon to run it")
        );
    }
}
