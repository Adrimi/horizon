//! YUNG's processors, as the game's types: the structures and pool elements
//! are carried as written (`crate::yung`), but the processors the mods write in
//! Java are converted here.
use serde_json::{Value, json};

/// The game's type for the YUNG's processors that place blocks past their
/// template, which keep their fields (`horizon-world`'s `pillars.rs`).
pub(crate) fn pillar_type(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "betterfortresses:pillar_processor" | "betterjungletemples:pillar_processor" => {
            "horizon:yung_pillar"
        }
        _ => return None,
    })
}

/// The data form of one of YUNG's processors: the processors to put in its
/// place, or `None` when it has none (or is not YUNG's).
pub(crate) fn processor(kind: &str, p: &Value) -> Option<Vec<Value>> {
    let (ns, _) = kind.split_once(':')?;
    Some(match kind {
        // Better Fortresses' stair supports get the blocks its Java code
        // names.
        "betterfortresses:stair_pillar_processor" => {
            vec![json!({"processor_type": "horizon:stair_pillar",
            "blocks": {"minecraft:stone_brick_stairs": "minecraft:red_nether_brick_stairs",
                "minecraft:purpur_stairs": "minecraft:red_nether_bricks"},
            "front": "minecraft:red_nether_brick_stairs",
            "pillar": "minecraft:red_nether_bricks"})]
        }
        _ if ns == "betterdeserttemples"
            && let Some(processor) = desert_temple(kind) =>
        {
            vec![processor]
        }
        "betterdungeons:small_dungeon_chest_processor" => {
            // Its config's defaults: one chest at least, two at most, a
            // fifth of the others kept until then; ChestBlock covers both.
            vec![json!({"processor_type": "horizon:count_cap",
                "blocks": ["minecraft:chest", "minecraft:trapped_chest"],
                "min": 1, "max": 2, "chance": 0.2f32 as f64,
                "replacement": block("minecraft:cave_air")})]
        }
        "betterwitchhuts:leg_processor" => vec![yung_leg(
            "minecraft:brown_stained_glass",
            json!({"id": "minecraft:oak_log", "properties": {"axis": "y"}}),
            false,
        )],
        "betterwitchhuts:fence_leg_processor" => vec![yung_leg(
            "minecraft:crimson_fence",
            block("minecraft:oak_fence"),
            true,
        )],
        _ if ns == "betteroceanmonuments" => ocean_monument(kind)?,
        _ if ns == "betterdungeons" || ns == "betterdeserttemples" => {
            // The spawners first, then YUNG's other processors.
            match yung_spawner(kind, p) {
                Some(spawner) => vec![spawner],
                None => yung_processor(kind, p)?,
            }
        }
        _ if kind.starts_with("betterwitchhuts:")
            || kind.starts_with("betterjungletemples:")
            || kind.starts_with("betterstrongholds:")
            || kind.starts_with("betterfortresses:")
            || kind.starts_with("betterendisland:") =>
        {
            yung_processor(kind, p)?
        }
        _ => return None,
    })
}

/// YUNG's own processors, as the game's types (`horizon:yung_*`,
/// `horizon_world::worldgen::structure::processor::yung`): the mods write
/// them in Java, the game runs the data form. The tables the mods hold in
/// their Java code (the block randomizers, the banner list, the brewing
/// recipes) are written in here, as the mods' own.
fn yung_processor(kind: &str, p: &Value) -> Option<Vec<Value>> {
    /// YUNG's `BlockStateRandomizer`: a default and the entries' shares.
    fn randomizer(default: &str, entries: &[(&str, f64)]) -> Value {
        json!({
            "defaultBlockState": block(default),
            "entries": entries
                .iter()
                .map(|(id, p)| json!({"blockState": block(id), "probability": p}))
                .collect::<Vec<_>>(),
        })
    }
    // The game's `horizon:yung_replace`: the blocks it applies to and the
    // randomizer they become.
    let replace = |blocks: &[&str], output: Value, flags: &[(&str, &str)]| {
        let mut out = json!({"processor_type": "horizon:yung_replace",
            "blocks": blocks, "output": output});
        for (key, value) in flags {
            out[*key] = json!(value);
        }
        out
    };
    // The game's `horizon:yung_leg`: the placeholder, what it becomes and
    // what the column under it is made of.
    let leg = |placeholder: &str, output: &str, pillar: Value, world_when_empty: bool| {
        json!({"processor_type": "horizon:yung_leg",
            "blocks": [placeholder], "output": block(output),
            "world_when_empty": world_when_empty, "pillar_states": pillar})
    };
    Some(match kind {
        // Better Jungle Temples' and Better End Island's block replace: the
        // mod's own randomizer, with the properties it asks for.
        "betterjungletemples:block_replace_processor"
        | "betterendisland:block_replace_processor" => {
            let target = block_name(&p["target_block"])?;
            let mut flags = Vec::new();
            for (key, value) in [
                ("copy_input_properties", "copy_input_properties"),
                ("preserve_waterlog", "copy_waterlogged"),
                ("randomize_facing", "copy_facing"),
                ("randomize_half", "copy_half"),
            ] {
                if p[value].as_bool().unwrap_or(false) {
                    flags.push((key, "true"));
                }
            }
            vec![replace(&[target.as_str()], p["output"].clone(), &flags)]
        }
        // Better Witch Huts' brewing stand: one of five potions, with the
        // ingredient it is brewed from.
        "betterwitchhuts:brewing_stand_processor" => vec![json!({
            "processor_type": "horizon:yung_brew",
            "recipes": [
                {"ingredient": "minecraft:glistering_melon_slice", "potion": "minecraft:healing"},
                {"ingredient": "minecraft:sugar", "potion": "minecraft:swiftness"},
                {"ingredient": "minecraft:pufferfish", "potion": "minecraft:water_breathing"},
                {"ingredient": "minecraft:golden_carrot", "potion": "minecraft:night_vision"},
                {"ingredient": "minecraft:phantom_membrane", "potion": "minecraft:slow_falling"},
            ],
        })],
        // Better Witch Huts' potted mushrooms.
        "betterwitchhuts:potted_mushroom_processor" => vec![replace(
            &["minecraft:potted_red_mushroom"],
            randomizer(
                "minecraft:potted_red_mushroom",
                &[
                    ("minecraft:potted_brown_mushroom", 0.2),
                    ("minecraft:potted_cornflower", 0.1),
                    ("minecraft:potted_cactus", 0.1),
                    ("minecraft:potted_dead_bush", 0.1),
                    ("minecraft:potted_fern", 0.1),
                    ("minecraft:potted_azalea", 0.1),
                ],
            ),
            &[],
        )],
        // Better Witch Huts' witch circle: its walls, its stairs and its
        // floor.
        "betterwitchhuts:witch_circle_processor" => {
            let bricks = randomizer(
                "minecraft:stone_bricks",
                &[
                    ("minecraft:mossy_stone_bricks", 0.6),
                    ("minecraft:cracked_stone_bricks", 0.1),
                ],
            );
            let stone = randomizer(
                "minecraft:cobblestone",
                &[
                    ("minecraft:mossy_cobblestone", 0.6),
                    ("minecraft:coarse_dirt", 0.1),
                ],
            );
            let stairs = randomizer(
                "minecraft:stone_brick_stairs",
                &[("minecraft:mossy_stone_brick_stairs", 0.6)],
            );
            vec![
                replace(&["minecraft:stone_bricks"], bricks.clone(), &[]),
                replace(&["minecraft:mossy_cobblestone"], stone, &[]),
                replace(&["minecraft:stone_brick_stairs"], stairs, &[]),
                replace(&["minecraft:gray_stained_glass"], bricks, &[]),
            ]
        }
        // Better Jungle Temples' torches: most of them are taken away.
        "betterjungletemples:torch_processor" => vec![json!({
            "processor_type": "horizon:yung_torch",
            "blocks": ["minecraft:torch", "minecraft:wall_torch"],
            "chance": 0.9,
        })],
        // Better Jungle Temples' dispensers: an empty one and one with a
        // fire charge.
        "betterjungletemples:empty_dispenser_processor" => vec![json!({
            "processor_type": "horizon:yung_dispenser", "items": []})],
        "betterjungletemples:fireball_dispenser_processor" => vec![json!({
            "processor_type": "horizon:yung_dispenser",
            "items": [{"id": "minecraft:fire_charge", "count": 1}]})],
        // Better Jungle Temples' blast furnaces become utility blocks.
        "betterjungletemples:blast_furnace_processor" => vec![replace(
            &["minecraft:blast_furnace"],
            randomizer(
                "minecraft:dispenser",
                &[("minecraft:dropper", 0.4), ("minecraft:observer", 0.2)],
            ),
            &[],
        )],
        // Better Jungle Temples' cave vines.
        "betterjungletemples:cave_vine_decoration_processor" => vec![replace(
            &["minecraft:green_stained_glass"],
            randomizer(
                "minecraft:air",
                &[
                    ("minecraft:cobblestone_slab", 0.05),
                    ("minecraft:mossy_cobblestone_slab", 0.05),
                    ("minecraft:cave_vines", 0.02),
                    ("minecraft:hanging_roots", 0.02),
                ],
            ),
            &[],
        )],
        // Better Strongholds' ores and rare blocks.
        "betterstrongholds:ore_processor" => vec![replace(
            &["minecraft:nether_gold_ore"],
            randomizer(
                "minecraft:coal_ore",
                &[
                    ("minecraft:coal_ore", 0.2),
                    ("minecraft:iron_ore", 0.2),
                    ("minecraft:gold_ore", 0.2),
                    ("minecraft:lapis_ore", 0.15),
                    ("minecraft:redstone_ore", 0.15),
                    ("minecraft:emerald_ore", 0.05),
                    ("minecraft:diamond_ore", 0.05),
                ],
            ),
            &[],
        )],
        "betterstrongholds:rare_block_processor" => vec![replace(
            &["minecraft:purpur_block"],
            randomizer(
                "minecraft:iron_block",
                &[
                    ("minecraft:iron_block", 0.3),
                    ("minecraft:quartz_block", 0.3),
                    ("minecraft:gold_block", 0.3),
                    ("minecraft:diamond_block", 0.1),
                ],
            ),
            &[],
        )],
        // Better Strongholds' banners: one of the mod's own, with its
        // patterns.
        "betterstrongholds:banner_processor" => vec![json!({
            "processor_type": "horizon:yung_banner",
            "blocks": ["minecraft:gray_wall_banner"],
            "banners": [
                {"block": "minecraft:magenta_wall_banner", "patterns": [
                    {"pattern": "minecraft:stripe_small", "color": "white"},
                    {"pattern": "minecraft:stripe_top", "color": "black"},
                    {"pattern": "minecraft:half_horizontal_mirror", "color": "black"},
                    {"pattern": "minecraft:border", "color": "black"},
                    {"pattern": "minecraft:stripe_middle", "color": "black"},
                    {"pattern": "minecraft:stripe_center", "color": "black"}]},
                {"block": "minecraft:black_wall_banner", "patterns": [
                    {"pattern": "minecraft:stripe_bottom", "color": "gray"},
                    {"pattern": "minecraft:stripe_center", "color": "black"},
                    {"pattern": "minecraft:half_horizontal", "color": "gray"},
                    {"pattern": "minecraft:creeper", "color": "black"},
                    {"pattern": "minecraft:skull", "color": "black"}]},
                {"block": "minecraft:purple_wall_banner", "patterns": [
                    {"pattern": "minecraft:stripe_small", "color": "magenta"},
                    {"pattern": "minecraft:bricks", "color": "purple"},
                    {"pattern": "minecraft:curly_border", "color": "magenta"},
                    {"pattern": "minecraft:border", "color": "black"}]},
            ],
        })],
        // Better Dungeons' walls and rubble: nothing is built where the
        // world is empty.
        "betterdungeons:small_dungeon_cobblestone_processor" => vec![json!({
            "processor_type": "horizon:yung_carve",
            "blocks": ["minecraft:cobblestone"]})],
        "betterdungeons:zombie_rot_processor" => vec![json!({
            "processor_type": "horizon:yung_carve",
            "blocks": ["minecraft:cobblestone", "minecraft:cyan_terracotta",
                "minecraft:cobblestone_stairs"]})],
        // Better Dungeons' legs: the placeholder becomes the leg's block and
        // a column runs down from it.
        "betterdungeons:small_dungeon_leg_processor" => vec![leg(
            "minecraft:yellow_stained_glass",
            "minecraft:mossy_cobblestone",
            randomizer(
                "minecraft:stone_bricks",
                &[
                    ("minecraft:mossy_stone_bricks", 0.5),
                    ("minecraft:cracked_stone_bricks", 0.2),
                ],
            ),
            false,
        )],
        "betterdungeons:skeleton_dungeon_leg_processor" => vec![leg(
            "minecraft:blue_stained_glass",
            "minecraft:cobblestone",
            randomizer(
                "minecraft:cobblestone",
                &[("minecraft:mossy_cobblestone", 0.5)],
            ),
            false,
        )],
        "betterdungeons:zombie_dungeon_leg_processor" => vec![leg(
            "minecraft:magenta_stained_glass",
            "minecraft:smooth_stone",
            randomizer(
                "minecraft:cobblestone",
                &[("minecraft:polished_andesite", 0.8)],
            ),
            true,
        )],
        "betterdungeons:small_nether_dungeon_leg_processor" => vec![leg(
            "minecraft:orange_terracotta",
            "minecraft:nether_bricks",
            randomizer("minecraft:nether_bricks", &[]),
            false,
        )],
        // Better Nether Fortresses' liquids: the mod's own block map.
        "betterfortresses:liquid_block_processor" => {
            let target = block_name(&p["target_block"])?;
            vec![replace(
                &[target.as_str()],
                p["target_block_output"].clone(),
                &[],
            )]
        }
        _ => return None,
    })
}

/// A block's name, in 1.21.1's `{"Name": ...}` shape or 26.3's
/// `{"id": ...}`.
fn block_name(v: &Value) -> Option<String> {
    v["Name"]
        .as_str()
        .or_else(|| v["id"].as_str())
        .map(|s| s.to_owned())
}

/// YUNG's leg processors (Better Ocean Monuments, Better Witch Huts): a
/// placeholder block becomes `state` and a column of it runs down through
/// air and fluids: the game's `horizon:yung_pillar` with fixed states, and
/// the placeholder's properties kept for the fence legs.
fn yung_leg(target: &str, state: Value, keep_properties: bool) -> Value {
    let fixed = json!({"entries": [], "defaultBlockState": state});
    json!({"processor_type": "horizon:yung_pillar", "target_block": block(target),
        "target_block_output": fixed, "pillar_states": fixed,
        "keep_properties": keep_properties})
}

/// Better Ocean Monuments' processors (no fields; what they do is in their
/// Java code): placeholder blocks as vanilla's `rule` or the game's
/// `horizon:pick` (a new `nextFloat` for each chance, as the mod draws),
/// and its sea-level ones as the game's `horizon:flood_air`,
/// `horizon:waterlog` and `horizon:keep_world_block`.
fn ocean_monument(kind: &str) -> Option<Vec<Value>> {
    let rule = |rules: &[(&str, Value)]| {
        let rules: Vec<Value> = rules
            .iter()
            .map(|(from, to)| {
                json!({"input_predicate": {"predicate_type": "minecraft:block_match", "block": from},
                "location_predicate": {"predicate_type": "minecraft:always_true"},
                "output_state": to})
            })
            .collect();
        json!({"processor_type": "minecraft:rule", "rules": rules})
    };
    let state = |id: &str, props: Value| json!({"id": id, "properties": props});
    let pick = |from: &str, picks: Vec<(f32, Value)>, default: Value, keep: bool| {
        let picks: Vec<Value> = picks
            .into_iter()
            .map(|(below, state)| json!({"below": below as f64, "state": state}))
            .collect();
        json!({"processor_type": "horizon:pick", "block": from, "picks": picks,
            "default": {"state": default}, "redraw": true, "keep_properties": keep})
    };
    let slabs = |from: &str, slab: &str, otherwise: &str| {
        let wet = |t: &str| state(slab, json!({"type": t, "waterlogged": "true"}));
        pick(
            from,
            vec![(0.4, wet("top")), (0.8, wet("bottom"))],
            block(otherwise),
            false,
        )
    };
    // Exposed below 0.1, else weathered below a new 0.3, else oxidized,
    // with the placeholder's properties for the stairs and slabs.
    let copper = |kind: &str| {
        let id = |age: &str| block(&format!("minecraft:{age}{kind}"));
        pick(
            &format!("minecraft:oxidized_{kind}"),
            vec![(0.1, id("exposed_")), (0.3, id("weathered_"))],
            id("oxidized_"),
            true,
        )
    };
    Some(match kind.strip_prefix("betteroceanmonuments:")? {
        "random_prismarine_slab_decoration_processor" => vec![slabs(
            "minecraft:lime_concrete",
            "minecraft:prismarine_slab",
            "minecraft:air",
        )],
        "random_dark_prismarine_slab_decoration_processor" => vec![slabs(
            "minecraft:blue_concrete",
            "minecraft:dark_prismarine_slab",
            "minecraft:water",
        )],
        "sand_gravel_processor" => {
            vec![rule(&[(
                "minecraft:yellow_stained_glass",
                block("minecraft:gravel"),
            )])]
        }
        "random_oxidization_processor" => [
            "copper",
            "cut_copper",
            "cut_copper_stairs",
            "cut_copper_slab",
        ]
        .into_iter()
        .map(copper)
        .collect(),
        "structure_void_processor" => vec![json!({"processor_type": "horizon:keep_world_block",
            "blocks": ["minecraft:purpur_slab", "minecraft:brown_wool"]})],
        "air_processor" => vec![json!({"processor_type": "horizon:flood_air"})],
        "waterlog_processor" => vec![json!({"processor_type": "horizon:waterlog"})],
        "seagrass_processor" => vec![rule(&[
            (
                "minecraft:red_sandstone_slab",
                state("minecraft:tall_seagrass", json!({"half": "lower"})),
            ),
            (
                "minecraft:end_stone_brick_slab",
                state("minecraft:tall_seagrass", json!({"half": "upper"})),
            ),
            ("minecraft:brick_slab", block("minecraft:seagrass")),
        ])],
        "random_sponge_processor" => vec![pick(
            "minecraft:orange_stained_glass",
            vec![(0.75, block("minecraft:wet_sponge"))],
            block("minecraft:water"),
            false,
        )],
        "leg_processor" => vec![yung_leg(
            "minecraft:blue_stained_glass",
            block("minecraft:prismarine_bricks"),
            false,
        )],
        _ => return None,
    })
}

/// A block state as 26.3 writes it.
fn block(id: &str) -> Value {
    json!({"id": id})
}

/// Better Desert Temples' placeholder blocks (glass, concrete and wool in
/// the templates) as the mod's processors turn them: the fixed ones as
/// vanilla's `rule`, the random ones as the game's `horizon:pick`, with the
/// mod's bounds on one `nextFloat` (YUNG's BlockStateRandomizer adds its
/// chances up in floats). The ones that read or change the world around
/// (pillars, Diorite's waterlogging) stay dropped.
fn desert_temple(kind: &str) -> Option<Value> {
    let swap = |from: &str, to: &str| {
        json!({"processor_type": "minecraft:rule", "rules": [{
            "input_predicate": {"predicate_type": "minecraft:block_match", "block": from},
            "location_predicate": {"predicate_type": "minecraft:always_true"},
            "output_state": block(to)}]})
    };
    let pick = |from: &str, picks: Vec<Value>, default: Value| json!({"processor_type": "horizon:pick", "block": from, "picks": picks, "default": default});
    // YUNG's BlockStateRandomizer: an entry wins below the sum of the
    // chances so far, summed as floats.
    let randomizer = |entries: &[(f32, &str)]| {
        let mut bottom = 0f32;
        entries
            .iter()
            .map(|(chance, id)| {
                bottom += chance;
                json!({"below": bottom as f64, "state": block(id)})
            })
            .collect::<Vec<_>>()
    };
    let at = |below: f32, id: &str| json!({"below": below as f64, "state": block(id)});
    // Sandstone stairs, upside down, facing Plane.HORIZONTAL.getRandomDirection.
    let stairs = || {
        let states: Vec<Value> = ["north", "east", "south", "west"]
            .iter()
            .map(|facing| {
                json!({"id": "minecraft:sandstone_stairs", "properties": {
                    "facing": facing, "half": "top", "shape": "straight", "waterlogged": "false"}})
            })
            .collect();
        json!({"states": states})
    };
    Some(match kind.strip_prefix("betterdeserttemples:")? {
        "blue_concrete_processor" => swap("minecraft:blue_concrete", "minecraft:blue_terracotta"),
        "yellow_concrete_processor" => {
            swap("minecraft:yellow_concrete", "minecraft:yellow_terracotta")
        }
        "red_wool_processor" => swap("minecraft:red_wool", "minecraft:tnt"),
        "white_stained_glass_processor" => pick(
            "minecraft:white_stained_glass",
            randomizer(&[(0.4, "minecraft:cobweb")]),
            json!({"state": block("minecraft:air")}),
        ),
        "polished_diorite_processor" => pick(
            "minecraft:polished_diorite",
            randomizer(&[(0.6, "minecraft:cut_sandstone")]),
            json!({"state": block("minecraft:sandstone")}),
        ),
        "orange_stained_glass_processor" => {
            let mut sus = at(0.01, "minecraft:suspicious_sand");
            sus["nbt"] = json!({"LootTable": "minecraft:archaeology/desert_pyramid"});
            let mut stairs = stairs();
            stairs["below"] = (0.45f32 as f64).into();
            pick(
                "minecraft:orange_stained_glass",
                vec![
                    sus,
                    at(0.1, "minecraft:sand"),
                    at(0.2, "minecraft:sandstone"),
                    stairs,
                ],
                json!({"state": block("minecraft:cut_sandstone")}),
            )
        }
        "red_stained_glass_processor" => pick(
            "minecraft:red_stained_glass",
            vec![at(0.3, "minecraft:sand"), at(0.75, "minecraft:sandstone")],
            stairs(),
        ),
        "pot_processor" => {
            // Four sherds, one float each; the pot back to its default state.
            let values: Vec<Value> = [
                (0.05f32, "archer"),
                (0.1, "miner"),
                (0.15, "prize"),
                (0.2, "skull"),
            ]
            .iter()
            .map(|(below, sherd)| {
                json!({"below": *below as f64, "value": format!("minecraft:{sherd}_pottery_sherd")})
            })
            .collect();
            pick(
                "minecraft:decorated_pot",
                Vec::new(),
                json!({"state": block("minecraft:decorated_pot"), "lists": [{
                    "key": "sherds", "count": 4, "values": values, "default": "minecraft:brick"}]}),
            )
        }
        _ => return None,
    })
}

/// An item as 26.3 saves it.
fn item(id: &str) -> Value {
    json!({"id": format!("minecraft:{id}"), "count": 1})
}

/// YUNG's processors that make a spawner of a fixed mob (YUNG's API's
/// `MobSpawnerData`), as vanilla's `rule`: every `input` block becomes a
/// spawner with that NBT.
fn yung_spawner(kind: &str, p: &Value) -> Option<Value> {
    const SPAWNER: &str = "minecraft:spawner";
    // (input block, mob, fields other than the defaults, whether the mob is
    // also the only potential, its equipment)
    let (input, mob, fields, potential, equipment): (_, _, &[(&str, i64)], _, _) = match kind {
        "betterdungeons:mob_spawner_processor" => {
            (SPAWNER, p["spawner_mob"].as_str()?, &[], true, None)
        }
        "betterdungeons:zombie_mob_spawner_processor" => (
            SPAWNER,
            "minecraft:zombie",
            &[("MaxNearbyEntities", 8)],
            true,
            None,
        ),
        "betterdungeons:skeleton_mob_spawner_processor" => (
            SPAWNER,
            "minecraft:skeleton",
            &[
                ("RequiredPlayerRange", 18),
                ("MaxNearbyEntities", 8),
                ("MaxSpawnDelay", 650),
            ],
            true,
            None,
        ),
        "betterdungeons:small_nether_dungeon_mob_spawner_processor" => {
            let mob = p["spawner_mob"].as_str()?;
            // Wither skeletons get netherite armor that never drops and a
            // stone sword (with the mod's default config, they and blazes
            // keep their loot).
            let armor = (mob == "minecraft:wither_skeleton").then(|| {
                json!({
                    "equipment": {
                        "feet": item("netherite_boots"),
                        "legs": item("netherite_leggings"),
                        "chest": item("netherite_chestplate"),
                        "head": item("netherite_helmet"),
                        "mainhand": item("stone_sword"),
                    },
                    "drop_chances": {"feet": 0.0, "legs": 0.0, "chest": 0.0, "head": 0.0},
                })
            });
            (SPAWNER, mob, &[], true, armor)
        }
        "betterdungeons:zombie_tombstone_spawner_processor" => (
            "minecraft:black_stained_glass",
            "minecraft:skeleton",
            &[],
            false,
            Some(json!({"equipment": {"mainhand": item("iron_sword")}})),
        ),
        "betterdeserttemples:gravel_processor" => (
            "minecraft:gravel",
            "minecraft:husk",
            &[("MaxNearbyEntities", 8), ("RequiredPlayerRange", 24)],
            false,
            None,
        ),
        // An empty hand list in 1.21.1: the skeletons spawn without a bow,
        // since a spawn entry with more than its id is not finalized.
        "betterdeserttemples:bone_block_processor" => (
            "minecraft:bone_block",
            "minecraft:skeleton",
            &[("RequiredPlayerRange", 32)],
            false,
            Some(json!({"equipment": {}})),
        ),
        "betterdeserttemples:infested_cracked_stone_bricks_processor" => (
            "minecraft:infested_cracked_stone_bricks",
            "minecraft:silverfish",
            &[("RequiredPlayerRange", 24)],
            false,
            None,
        ),
        // 1.21.1 ignores the items' old `Count` and `tag` (the shield's
        // pattern, the boots' dye): plain items.
        "betterdeserttemples:yellow_wool_processor" => (
            "minecraft:yellow_wool",
            "minecraft:husk",
            &[("MaxNearbyEntities", 8), ("RequiredPlayerRange", 24)],
            false,
            Some(json!({"equipment": {
                "mainhand": item("golden_sword"),
                "offhand": item("shield"),
                "feet": item("leather_boots"),
                "legs": item("leather_leggings"),
                "chest": item("golden_chestplate"),
                "head": item("chainmail_helmet"),
            }})),
        ),
        _ => return None,
    };
    let mut entity = json!({"id": mob});
    if let Some(Value::Object(extra)) = equipment {
        entity.as_object_mut()?.extend(extra);
    }
    let potentials = match potential {
        true => json!([{"data": {"entity": entity.clone()}, "weight": 1}]),
        false => json!([]),
    };
    let mut nbt = json!({
        "Delay": 20, "MinSpawnDelay": 200, "MaxSpawnDelay": 800, "SpawnCount": 4,
        "MaxNearbyEntities": 6, "RequiredPlayerRange": 16, "SpawnRange": 4,
        "SpawnData": {"entity": entity},
        "SpawnPotentials": potentials,
    });
    for (key, value) in fields {
        nbt[*key] = (*value).into();
    }
    Some(json!({
        "processor_type": "minecraft:rule",
        "rules": [{
            "input_predicate": {"predicate_type": "minecraft:block_match", "block": input},
            "location_predicate": {"predicate_type": "minecraft:always_true"},
            "output_state": {"Name": SPAWNER},
            "block_entity_modifier": {"type": "minecraft:append_static", "data": nbt},
        }],
    }))
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_structure_mods::data;
    use crate::processors::{ProcessorData, carry};
    use std::collections::BTreeMap;

    #[test]
    fn yungs_spawners_become_rules() {
        let mut pool = json!({"elements": [{"element": {"processors": {"processors": [
            {"processor_type": "betterdungeons:skeleton_mob_spawner_processor"},
            {"processor_type": "betterdeserttemples:bone_block_processor"},
            {"processor_type": "betterdungeons:small_nether_dungeon_mob_spawner_processor",
             "spawner_mob": "minecraft:wither_skeleton"},
            {"processor_type": "betterdungeons:candle_processor"}]}}}]});
        let mut carried = BTreeMap::new();
        assert!(carry("template_pool", &mut pool, &data(), &mut carried));
        let p = &pool["elements"][0]["element"]["processors"]["processors"];
        let rule = |i: usize| &p[i]["rules"][0];
        assert_eq!(rule(0)["input_predicate"]["block"], "minecraft:spawner");
        assert_eq!(
            rule(0)["output_state"],
            json!({"Name": "minecraft:spawner"})
        );
        let nbt = &rule(0)["block_entity_modifier"]["data"];
        assert_eq!(
            (
                nbt["RequiredPlayerRange"].clone(),
                nbt["MaxNearbyEntities"].clone()
            ),
            (json!(18), json!(8))
        );
        assert_eq!(nbt["MaxSpawnDelay"], 650);
        assert_eq!(
            nbt["SpawnPotentials"],
            json!([{"data": {"entity": {"id": "minecraft:skeleton"}}, "weight": 1}])
        );
        // Bone blocks become bowless skeleton spawners with no potentials.
        let bone = &rule(1)["block_entity_modifier"]["data"];
        assert_eq!(rule(1)["input_predicate"]["block"], "minecraft:bone_block");
        assert_eq!(
            bone["SpawnData"],
            json!({"entity": {"id": "minecraft:skeleton", "equipment": {}}})
        );
        assert_eq!(bone["SpawnPotentials"], json!([]));
        let wither = &rule(2)["block_entity_modifier"]["data"]["SpawnData"]["entity"];
        assert_eq!(
            wither["equipment"]["head"]["id"],
            "minecraft:netherite_helmet"
        );
        assert_eq!(wither["drop_chances"]["feet"], 0.0);
        assert_eq!(p[3]["processor_type"], "betterdungeons:candle_processor");
        assert_eq!(carried.values().sum::<usize>(), 3);
    }

    #[test]
    fn desert_temple_placeholders_become_rules_and_picks() {
        let mut list = json!({"processors": [
            {"processor_type": "betterdeserttemples:red_wool_processor"},
            {"processor_type": "betterdeserttemples:white_stained_glass_processor"},
            {"processor_type": "betterdeserttemples:orange_stained_glass_processor"},
            {"processor_type": "betterdeserttemples:pot_processor"},
            {"processor_type": "betterdeserttemples:quartz_pillar_processor"}]});
        let mut carried = BTreeMap::new();
        assert!(carry("processor_list", &mut list, &data(), &mut carried));
        let p = list["processors"].as_array().unwrap();
        assert_eq!(
            p[0]["rules"][0]["output_state"],
            json!({"id": "minecraft:tnt"})
        );
        assert_eq!(p[1]["processor_type"], "horizon:pick");
        assert_eq!(p[1]["picks"][0]["below"], 0.4f32 as f64);
        assert_eq!(p[1]["default"]["state"]["id"], "minecraft:air");
        let glass = &p[2]["picks"];
        assert_eq!(glass[0]["state"]["id"], "minecraft:suspicious_sand");
        assert_eq!(
            glass[0]["nbt"]["LootTable"],
            "minecraft:archaeology/desert_pyramid"
        );
        assert_eq!(glass[3]["states"].as_array().unwrap().len(), 4);
        assert_eq!(glass[3]["below"], 0.45f32 as f64);
        assert_eq!(p[3]["default"]["lists"][0]["count"], 4);
        // The pillar reads the world below: still Java, still dropped later.
        assert_eq!(
            p[4]["processor_type"],
            "betterdeserttemples:quartz_pillar_processor"
        );
        assert_eq!(carried.values().sum::<usize>(), 4);
    }

    #[test]
    fn small_dungeon_chests_are_capped() {
        let mut list = json!({"processors": [
            {"processor_type": "betterdungeons:small_dungeon_chest_processor"}]});
        let mut carried = BTreeMap::new();
        assert!(carry("processor_list", &mut list, &data(), &mut carried));
        let cap = &list["processors"][0];
        assert_eq!(cap["processor_type"], "horizon:count_cap");
        assert_eq!(
            (cap["min"].as_u64(), cap["max"].as_u64()),
            (Some(1), Some(2))
        );
        assert_eq!(cap["replacement"], json!({"id": "minecraft:cave_air"}));
    }

    #[test]
    fn ocean_monuments_become_the_games() {
        let names = [
            "random_prismarine_slab_decoration_processor",
            "random_dark_prismarine_slab_decoration_processor",
            "sand_gravel_processor",
            "random_oxidization_processor",
            "structure_void_processor",
            "air_processor",
            "waterlog_processor",
            "seagrass_processor",
            "random_sponge_processor",
            "leg_processor",
        ];
        let processors: Vec<Value> = names
            .iter()
            .map(|n| json!({"processor_type": format!("betteroceanmonuments:{n}")}))
            .chain([json!({"processor_type": "betterwitchhuts:fence_leg_processor"})])
            .collect();
        let mut list = json!({"processors": processors});
        let mut carried = BTreeMap::new();
        assert!(carry(
            "processor_list",
            &mut list,
            &ProcessorData::default(),
            &mut carried
        ));
        let p = list["processors"].as_array().unwrap();
        let types: Vec<&str> = p
            .iter()
            .map(|p| p["processor_type"].as_str().unwrap())
            .collect();
        let mut want = vec!["horizon:pick", "horizon:pick", "minecraft:rule"];
        want.extend(["horizon:pick"; 4]);
        want.extend([
            "horizon:keep_world_block",
            "horizon:flood_air",
            "horizon:waterlog",
            "minecraft:rule",
            "horizon:pick",
            "horizon:yung_pillar",
            "horizon:yung_pillar",
        ]);
        assert_eq!(types, want);
        assert_eq!(carried.values().sum::<usize>(), 11);
        // Copper stairs keep their properties, each chance on a new float.
        let stairs = &p[5];
        assert_eq!(stairs["block"], "minecraft:oxidized_cut_copper_stairs");
        assert_eq!(
            stairs["picks"][1]["state"]["id"],
            "minecraft:weathered_cut_copper_stairs"
        );
        assert_eq!(
            (stairs["redraw"].clone(), stairs["keep_properties"].clone()),
            (json!(true), json!(true))
        );
        assert_eq!(p[0]["picks"][0]["state"]["properties"]["type"], "top");
        assert_eq!(p[1]["default"]["state"]["id"], "minecraft:water");
        assert_eq!(
            p[12]["target_block_output"]["defaultBlockState"]["id"],
            "minecraft:prismarine_bricks"
        );
        assert_eq!(p[13]["keep_properties"], true);
    }
}
