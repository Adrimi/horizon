//! A mod for the runtime's tests: `/spin` loops forever, `/panic` panics,
//! `/grow` asks for a gigabyte, `/hello` chats, `/tool` names the hotbar's
//! fastest slot for the target and selects it, `/select a b ...` selects
//! each slot in turn, `/food` reads hunger and the selected slot's food;
//! `/tags <id> <slot>` reads an item's tags and a hotbar slot's;
//! `tick` and `frame` do nothing, to measure the cost of
//! a call.
use horizon_mod_sdk::{
    Mod, Registration, atmosphere, chat, debug, entity, game, hotbar, hud, inventory, items, keys,
    lighting, chat_decorate, overlay, resources, texture_rules, particles, recipes, screen, settings, slots, storage, target, tooltip, weather, world,
};

struct Guest {
    ticks: u64,
    /// The screen the mod opened, 0 when it has none.
    screen: u32,
    /// The panel the mod opened, 0 when it has none.
    panel: u32,
    /// The random ticks heard so far, and the last block's description.
    heard: u32,
    last: String,
    /// What the last chat hook read from `chat.get-messages`.
    said: String,
}

impl Mod for Guest {
    fn init() -> Result<(Self, Registration), String> {
        let hooks = [
            "spin",
            "panic",
            "grow",
            "hello",
            "tool",
            "select",
            "food",
            "tags",
            "block",
            "break",
            "breakheld",
            "wear",
            "broken",
            "look",
            "surface",
            "biomes",
            "colors",
            "store",
            "read",
            "rm",
            "files",
            "key",
            "screen",
            "browser",
            "giveitem",
            "weatherfx",
            "overlayfx",
            "entityfx",
            "lookfx",
            "slotfx",
            "foodfx",
            "ticks",
            "lightfx",
            "texfx",
            "said",
            "toolfx",
            "resfx",
            "save",
            "load",
            "save_key",
            "load_key",
        ]
        .into_iter()
        .fold(Registration::new(), Registration::command)
        .on_tick()
        .on_chat()
        .on_frame();
        Ok((
            Guest {
                ticks: 0,
                screen: 0,
                panel: 0,
                heard: 0,
                last: String::new(),
                said: String::new(),
            },
            hooks,
        ))
    }
    fn chat(&mut self, text: &str) {
        // A picture before the messages that say `pic`, from their sender.
        let messages = chat::get_messages();
        self.said = messages
            .iter()
            .map(|m| format!("{}:{}:{}", m.id, m.text, m.sender.as_ref().map_or("-", |s| &s.name)))
            .collect::<Vec<_>>()
            .join("|");
        if text.contains("pic")
            && let Some(m) = messages.last()
            && m.sender.is_some()
        {
            chat_decorate::set_picture(
                m.id,
                Some(&chat_decorate::Picture {
                    source: chat_decorate::Source::SenderSkin,
                    regions: vec![chat_decorate::Region {
                        x: 8,
                        y: 8,
                        width: 8,
                        height: 8,
                    }],
                    width: 8,
                    height: 8,
                }),
            );
        }
    }
    fn tick(&mut self, _tick: u64) {
        self.ticks += 1;
        for t in world::random_ticked_blocks() {
            self.heard += 1;
            self.last = format!("{} at {} {} {}", t.block.id, t.pos.x, t.pos.y, t.pos.z);
        }
    }
    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        match name {
            "spin" => {
                let mut n = 0u64;
                while std::hint::black_box(n) != u64::MAX {
                    n = n.wrapping_add(2);
                }
                Ok(None)
            }
            "panic" => panic!("asked to"),
            "grow" => {
                let big = std::hint::black_box(vec![1u8; 1 << 30]);
                Ok(Some(format!("{}", big.len())))
            }
            "tool" => {
                let block = target::get_breaking()
                    .or_else(target::get_block)
                    .ok_or("no block")?;
                let hotbar = inventory::get_hotbar();
                let (slot, stack) = hotbar
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| Some((i, s.as_ref()?)))
                    .max_by(|a, b| a.1.mining_speed.total_cmp(&b.1.mining_speed))
                    .ok_or("empty hotbar")?;
                hotbar::select(slot as u32);
                Ok(Some(format!(
                    "{} at {} {} {}: slot {slot} {} x{} ({}/{}, efficiency {}, speed {}, {}) of {}, was {}, tags {}",
                    block.id,
                    block.x,
                    block.y,
                    block.z,
                    stack.id,
                    stack.count,
                    stack.damage,
                    stack.max_damage,
                    stack.efficiency,
                    stack.mining_speed,
                    if stack.correct_for_drops {
                        "drops"
                    } else {
                        "no drops"
                    },
                    hotbar.len(),
                    inventory::get_selected(),
                    block.tags.join(" "),
                )))
            }
            "look" => {
                let block = target::get_block_name().unwrap_or_else(|| "no block".into());
                let progress = target::get_breaking_progress()
                    .map_or("not breaking".into(), |p| format!("{p}"));
                let entity = target::get_entity().map_or("no entity".into(), |e| {
                    format!(
                        "{} {} {} {}/{}",
                        e.id, e.type_id, e.name, e.health, e.max_health
                    )
                });
                Ok(Some(format!("{block}, {progress}, {entity}")))
            }
            "food" => {
                let stats = game::get_food_stats().ok_or("no player")?;
                let held = match inventory::get_food(inventory::get_selected()) {
                    Some(f) => format!("+{} +{}", f.nutrition, f.saturation),
                    None => "not food".into(),
                };
                Ok(Some(format!(
                    "food {} saturation {} exhaustion {}, held {held}, slot 9 {:?}",
                    stats.food,
                    stats.saturation,
                    stats.exhaustion,
                    inventory::get_food(9).map(|f| f.nutrition),
                )))
            }
            "tags" => {
                let (id, slot) = args.split_once(' ').unwrap_or((args, "0"));
                let slot: u8 = slot.trim().parse().map_err(|_| "not a slot")?;
                let id = id.trim();
                Ok(Some(format!(
                    "{id}: {}, slot {slot} {}",
                    inventory::item_tags(id).join(" "),
                    inventory::get_tags(slot).join(" "),
                )))
            }
            "block" | "break" | "breakheld" => {
                let n: Vec<i32> = args
                    .split_whitespace()
                    .map(|a| a.parse().map_err(|_| "not a number"))
                    .collect::<Result<_, _>>()?;
                let [x, y, z] = n[..] else {
                    return Err("x y z".into());
                };
                let pos = world::BlockPos { x, y, z };
                Ok(Some(match name {
                    "block" => match world::get_block(pos) {
                        Some(b) => format!("{} {:?} tags {}", b.id, b.properties, b.tags.join(" ")),
                        None => "unloaded".into(),
                    },
                    "break" => format!(
                        "{} {}",
                        world::break_block(pos, true),
                        world::break_block(pos, true)
                    ),
                    _ => world::break_block_with_held(pos).to_string(),
                }))
            }
            "wear" => {
                let n = args.trim().parse().map_err(|_| "not a number")?;
                Ok(Some(format!(
                    "{:?} {:?}",
                    world::damage_held(n),
                    world::damage_held(n)
                )))
            }
            "resfx" => {
                let spaces = resources::namespaces();
                let files = resources::list_files("minecraft", args);
                let first = files.first().cloned().unwrap_or_default();
                let size = resources::read_file("minecraft", &first).map(|b| b.len());
                let escape = resources::read_file("minecraft", "../x").is_some();
                Ok(Some(format!(
                    "{} namespaces, {} files, first {first} {size:?}, escape {escape}",
                    spaces.join(","),
                    files.len()
                )))
            }
            "said" => Ok(Some(self.said.clone())),
            "texfx" => {
                // Stone shows a red and a blue tile by turns along its faces;
                // `off` removes them.
                if args == "off" {
                    texture_rules::clear_rules();
                    return Ok(Some("cleared".into()));
                }
                texture_rules::upload_sprite("red", include_bytes!("red.png"))?;
                texture_rules::upload_sprite("blue", include_bytes!("blue.png"))?;
                texture_rules::register_rule(&texture_rules::Rule {
                    blocks: vec![texture_rules::BlockFilter {
                        id: "minecraft:stone".into(),
                        properties: Vec::new(),
                    }],
                    sprites: Vec::new(),
                    faces: 63,
                    connect: texture_rules::Connect::Block,
                    method: texture_rules::Method::Position,
                    tiles: vec!["red".into(), "blue".into()],
                    pattern_tiles: Vec::new(),
                    quarter_tiles: Vec::new(),
                    grid_tiles: vec![0, 1],
                    width: 2,
                    height: 1,
                    weights: Vec::new(),
                })?;
                Ok(Some("rules set".into()))
            }
            "lightfx" => {
                let torch = lighting::get_block_emission("minecraft:torch");
                let stone = lighting::get_block_emission("minecraft:stone");
                if args == "off" {
                    lighting::set_lights(&[]);
                } else {
                    lighting::set_lights(&[
                        lighting::Light {
                            x: 1.5,
                            y: 65.0,
                            z: -2.5,
                            level: 14,
                        },
                        lighting::Light {
                            x: 0.0,
                            y: 70.0,
                            z: 0.0,
                            level: 40,
                        },
                    ]);
                }
                Ok(Some(format!("torch {torch} stone {stone}")))
            }
            "ticks" => {
                if !args.is_empty() {
                    world::watch_random_ticks(
                        &args.split_whitespace().map(str::to_owned).collect::<Vec<_>>(),
                    );
                    self.heard = 0;
                }
                Ok(Some(format!("{} heard, last {}", self.heard, self.last)))
            }
            "broken" => Ok(Some(
                world::broken_blocks()
                    .iter()
                    .map(|b| {
                        format!(
                            "{} at {} {} {} by player {}",
                            b.block.id, b.pos.x, b.pos.y, b.pos.z, b.by_player
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            )),
            "select" => {
                for slot in args.split_whitespace() {
                    hotbar::select(slot.parse().map_err(|_| "not a slot")?);
                }
                Ok(None)
            }
            "surface" => {
                let n: Vec<i32> = args
                    .split_whitespace()
                    .map(|a| a.parse().map_err(|_| "not a number"))
                    .collect::<Result<_, _>>()?;
                let [cx, cz] = n[..] else {
                    return Err("cx cz".into());
                };
                Ok(Some(match world::get_chunk_surface(cx, cz) {
                    Some(s) => format!(
                        "surface {} colors, {} heights, {} depths, {} biomes",
                        s.color.len(),
                        s.y.len(),
                        s.depth.len(),
                        s.biome.len()
                    ),
                    None => "no surface".into(),
                }))
            }
            "biomes" => Ok(Some(format!("biomes {}", world::get_biome_names().len()))),
            "colors" => Ok(Some(format!("colors {}", world::get_map_colors().len()))),
            "store" => {
                let (name, text) = args.split_once(' ').unwrap_or((args, ""));
                storage::write(name, text.as_bytes())?;
                Ok(Some(format!("wrote {name}")))
            }
            "read" => Ok(Some(match storage::read(args.trim()) {
                Some(bytes) => format!("read {}", String::from_utf8_lossy(&bytes)),
                None => "nothing".into(),
            })),
            "rm" => Ok(Some(format!("removed {}", storage::remove(args.trim())))),
            "files" => Ok(Some(format!("files {}", storage::files().join(",")))),
            "key" => {
                keys::bind("map", "key.keyboard.m")?;
                let bound = keys::get("map").unwrap_or_default();
                let events = keys::poll();
                let polled: Vec<String> = events
                    .iter()
                    .map(|e| format!("{} {} {}", e.action, e.pressed, e.repeat))
                    .collect();
                Ok(Some(format!("key {bound} events {}", polled.join(";"))))
            }
            "screen" => {
                // The screen is opened once; the command then draws and
                // reports what the player did.
                if self.screen == 0 {
                    self.screen = screen::open("Test screen")?;
                }
                let id = self.screen;
                let size = screen::get_size(id);
                screen::upload_texture(
                    0,
                    2,
                    2,
                    &[
                        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                    ],
                )?;
                screen::draw(
                    id,
                    &[
                        screen::Shape::Rect(screen::RectShape {
                            x: 20.0,
                            y: 30.0,
                            width: 40.0,
                            height: 20.0,
                            color: 0x00FF00,
                        }),
                        screen::Shape::Text(screen::TextShape {
                            x: 20.0,
                            y: 60.0,
                            text: "hi".into(),
                            color: 0xFFFFFF,
                            shadow: true,
                        }),
                        screen::Shape::Image(screen::ImageShape {
                            x: 20.0,
                            y: 80.0,
                            width: 64.0,
                            height: 64.0,
                            texture: 0,
                        }),
                    ],
                );
                let events = screen::get_input(id);
                let kinds: Vec<&str> = events
                    .iter()
                    .map(|e| match e {
                        screen::Event::Key(_) => "key",
                        screen::Event::MouseMove(_) => "move",
                        screen::Event::MouseButton(_) => "button",
                        screen::Event::Wheel(_) => "wheel",
                        screen::Event::Closed => "closed",
                    })
                    .collect();
                Ok(Some(format!(
                    "screen {id} sized {} events {}",
                    size.is_some(),
                    kinds.join(",")
                )))
            }
            "weatherfx" => {
                // Weather, settings, a biome, the ground, a picture, a
                // particle, what landed, and a haze.
                let state = weather::get_weather();
                let pos = world::BlockPos { x: 1, y: 64, z: 1 };
                let biome = world::get_biome(pos).map(|b| format!("{} {}", b.id, b.foliage));
                let ground = world::get_surface(pos, 70);
                let solid = world::is_solid(world::BlockPos { x: 0, y: 64, z: 0 });
                let sprite = particles::define_sprite(&particles::Sprite {
                    name: "drop".into(),
                    width: 2,
                    height: 2,
                    pixels: vec![255; 16],
                })?;
                let spawned = particles::spawn(particles::Options {
                    sprite,
                    x: 0.5,
                    y: 70.0,
                    z: 0.5,
                    vx: 0.0,
                    vy: -0.4,
                    vz: 0.0,
                    color: particles::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                    size: 0.1,
                    lifetime: 40,
                    gravity: 0.0,
                    wind_x: 0.0,
                    wind_z: 0.0,
                    collide: true,
                    size_curve: particles::SizeCurve::Constant,
                    fade: particles::Fade::Late,
                    flat: false,
                    linger: 0,
                    sway: 0.0,
                    sway_period: 1,
                    spin: 0.0,
                    tumble: 0.0,
                });
                let nobody = particles::spawn(particles::Options {
                    sprite: sprite + 1000,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    vx: 0.0,
                    vy: 0.0,
                    vz: 0.0,
                    color: particles::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                    size: 0.1,
                    lifetime: 40,
                    gravity: 0.0,
                    wind_x: 0.0,
                    wind_z: 0.0,
                    collide: false,
                    size_curve: particles::SizeCurve::Constant,
                    fade: particles::Fade::None,
                    flat: false,
                    linger: 0,
                    sway: 0.0,
                    sway_period: 1,
                    spin: 0.0,
                    tumble: 0.0,
                });
                let landed = particles::landed().len();
                atmosphere::set_haze(Some(atmosphere::Haze {
                    amount: 2.0,
                    red: 0.6,
                    green: 0.7,
                    blue: 0.8,
                    density: 0.2,
                }));
                Ok(Some(format!(
                    "weather {:?} setting {} biome {biome:?} ground {ground:?} solid {solid} sprite {sprite} spawned {spawned} unknown {nobody} landed {landed}",
                    state.map(|w| (w.rain, w.thunder)),
                    settings::get_particles(),
                )))
            }
            "overlayfx" => {
                // Two layers, the world, an image, the HUD's state, the bars,
                // the seed, the fov and a cave scan.
                overlay::paint(
                    overlay::Layer::UnderBars,
                    &[screen::Element::Fill(screen::FillElement {
                        x: 1.0,
                        y: 2.0,
                        width: 3.0,
                        height: 4.0,
                        top: 0xFF00_0000,
                        bottom: 0xFF00_0000,
                    })],
                );
                overlay::paint(
                    overlay::Layer::OverBars,
                    &[
                        screen::Element::Picture(screen::PictureElement {
                            x: 0.0,
                            y: 0.0,
                            width: 8.0,
                            height: 8.0,
                            texture: 0,
                            u: 0.0,
                            v: 0.0,
                            u_width: 2.0,
                            v_height: 2.0,
                            tint: 0xFFFF_FFFF,
                        }),
                        screen::Element::Fill(screen::FillElement {
                            x: 0.0,
                            y: 0.0,
                            width: 1.0,
                            height: 1.0,
                            top: 0,
                            bottom: 0,
                        }),
                    ],
                );
                overlay::paint_world(&[overlay::WorldElement::Label(overlay::WorldLabel {
                    x: 1.0,
                    y: 65.0,
                    z: 1.0,
                    text: "home".into(),
                    color: 0xFFFF_FFFF,
                    shadow: true,
                    scale: 1.0,
                    offset_y: -4.0,
                })]);
                let uploaded = overlay::upload_texture(0, 2, 2, &[255; 16]).is_ok();
                let refused = overlay::upload_texture(7, 2, 2, &[255; 16]).is_err();
                let size = overlay::get_size().map(|s| (s.width, s.height));
                let state = overlay::get_hud_state().map(|h| (h.screen_open, h.debug, h.hidden));
                let bars = hud::get_bars().map(|b| (b.food.len(), b.health.len()));
                let columns = world::get_chunk_columns(0, 0, 60).map(|c| c.y.len());
                overlay::clear(overlay::Layer::UnderBars);
                Ok(Some(format!(
                    "overlay {uploaded} {refused} size {size:?} hud {state:?} bars {bars:?} seed {:?} fov {:?} columns {columns:?} width {}",
                    game::get_seed(),
                    game::get_fov(),
                    overlay::get_text_width("ab"),
                )))
            }
            "entityfx" => {
                let all = entity::nearby("", 10.0);
                let orbs = entity::nearby("minecraft:experience_orb", 10.0);
                let first = all.first().map(|e| (e.id, e.name.clone(), e.hostile));
                let removed = entity::remove(1);
                let again = entity::remove(1);
                let xp = entity::set_xp(2, 9);
                let wrong = entity::set_xp(1, 9);
                Ok(Some(format!(
                    "entities {} orbs {} first {first:?} removed {removed} again {again} xp {xp} wrong {wrong} face {}",
                    all.len(),
                    orbs.len(),
                    entity::get_face("x").is_some(),
                )))
            }
            "lookfx" => {
                let at = target::get_block().map(|b| world::BlockPos {
                    x: b.x,
                    y: b.y,
                    z: b.z,
                });
                let contents = at.and_then(world::get_container).map(|slots| {
                    slots
                        .iter()
                        .flatten()
                        .map(|s| format!("{}x{}", s.id, s.count))
                        .collect::<Vec<_>>()
                });
                let elsewhere = world::get_container(world::BlockPos { x: 9, y: 9, z: 9 });
                Ok(Some(format!(
                    "look face {:?} armor {:?} absorption {:?} contents {contents:?} elsewhere {}",
                    target::get_block_face(),
                    target::get_entity_armor(),
                    target::get_entity_absorption(),
                    elsewhere.is_some(),
                )))
            }
            "slotfx" => {
                let inv = inventory::get_inventory();
                let summary = inv.as_ref().map(|i| {
                    (
                        i.slots.len(),
                        i.armour.len(),
                        i.offhand.is_some(),
                        i.slots[0].as_ref().map(|s| (s.id.clone(), s.count)),
                    )
                });
                let crafting =
                    inventory::get_crafting().map(|c| (c.grid.len(), c.output.is_some()));
                let session = game::get_session().map(|s| s.id);
                slots::set_marks(&[
                    slots::Mark {
                        slot: 0,
                        shape: slots::MarkShape::Frame(slots::FrameMark { color: 0xFFFF_D058 }),
                    },
                    slots::Mark {
                        slot: 0,
                        shape: slots::MarkShape::Dot(slots::DotMark {
                            color: 0xFFFF_F0A8,
                            size: 3.0,
                        }),
                    },
                ]);
                Ok(Some(format!(
                    "slots {summary:?} crafting {crafting:?} session {session:?} hovered {:?}",
                    slots::get_hovered_slot()
                )))
            }
            "foodfx" => {
                let health = game::get_health().map(|h| (h.health, h.max_health, h.absorption));
                let food = inventory::get_food_of("minecraft:cooked_beef")
                    .map(|f| (f.nutrition, f.saturation));
                tooltip::set_content(
                    "minecraft:cooked_beef",
                    Some(&tooltip::Content {
                        width: 60.0,
                        height: 10.0,
                        elements: vec![screen::Element::Label(screen::LabelElement {
                            x: 0.0,
                            y: 0.0,
                            text: "8 hunger".into(),
                            color: 0xFFFF_FFFF,
                            shadow: true,
                            scale: 1.0,
                        })],
                    }),
                );
                debug::set_lines(&["hunger: 20".to_string()]);
                Ok(Some(format!(
                    "food health {health:?} effects {:?} difficulty {:?} beef {food:?} rotten {} offhand {} shift {} hovered {}",
                    game::get_effects(),
                    game::get_difficulty(),
                    inventory::get_harmful_food("minecraft:rotten_flesh"),
                    inventory::get_offhand().is_some(),
                    keys::is_down("key.keyboard.left.shift"),
                    tooltip::get_hovered().map_or("none".into(), |h| h.item),
                )))
            }
            "toolfx" => {
                // Tools, slots, input, moving items, the action bar.
                let tools: Vec<String> = inventory::get_tools()
                    .iter()
                    .map(|t| {
                        t.as_ref().map_or("-".to_owned(), |t| {
                            format!("{}:{}:{}", t.id, t.damage_per_block, t.enchantments.len())
                        })
                    })
                    .collect();
                let slots = inventory::get_slots();
                let input = game::get_input_state().map(|i| (i.sneaking, i.attacking));
                let moved = match args.split_once(' ') {
                    Some((a, b)) => Some(inventory::move_item(
                        a.parse().map_err(|_| "slot")?,
                        b.parse().map_err(|_| "slot")?,
                    )),
                    None => None,
                };
                hud::show_action_bar("Tool swap mode: on", 0x55FF55);
                Ok(Some(format!(
                    "tools {} slots {} input {input:?} moved {moved:?}",
                    tools.join(","),
                    slots.len()
                )))
            }
            "giveitem" => {
                let ok = inventory::give(args, 3);
                Ok(Some(format!("give {ok}")))
            }
            "browser" => {
                // Items, recipes, a panel over the game's screen, and the
                // elements painted into it.
                if self.panel == 0 {
                    self.panel = screen::open_panel(
                        "Browser",
                        screen::Area {
                            x: 200.0,
                            y: 10.0,
                            width: 100.0,
                            height: 200.0,
                        },
                        &["key.keyboard.r".into(), "key.keyboard.u".into()],
                    )?;
                }
                let id = self.panel;
                let items = items::list();
                let recipes = recipes::list();
                let first = items.first().map(|i| i.id.clone()).unwrap_or_default();
                let known = items::get(&first).is_some() && items::get("nope:nothing").is_none();
                screen::paint(
                    id,
                    &[
                        screen::Element::Fill(screen::FillElement {
                            x: 200.0,
                            y: 10.0,
                            width: 100.0,
                            height: 200.0,
                            top: 0xBE10_1010,
                            bottom: 0xBE00_0000,
                        }),
                        screen::Element::Label(screen::LabelElement {
                            x: 204.0,
                            y: 14.0,
                            text: "Search".into(),
                            color: 0xFFFF_FFFF,
                            shadow: true,
                            scale: 1.0,
                        }),
                        screen::Element::Sprite(screen::SpriteElement {
                            x: 204.0,
                            y: 30.0,
                            width: 9.0,
                            height: 9.0,
                            name: "gui/sprites/hud/heart/full".into(),
                            tint: 0xFFFF_FFFF,
                        }),
                        screen::Element::Item(screen::ItemElement {
                            x: 204.0,
                            y: 44.0,
                            id: first.clone(),
                            count: 5,
                        }),
                        screen::Element::Tooltip(screen::TooltipElement {
                            x: 204.0,
                            y: 70.0,
                            id: first,
                            count: 1,
                            extra: vec![screen::TipLine {
                                text: "R: recipes".into(),
                                color: 0xFF80_8080,
                            }],
                        }),
                    ],
                );
                let kinds: Vec<String> = screen::get_input(id)
                    .iter()
                    .map(|e| match e {
                        screen::Event::Key(k) => format!("key {} {}", k.key, k.pressed),
                        screen::Event::MouseMove(_) => "move".into(),
                        screen::Event::MouseButton(b) => format!("button {}", b.button),
                        screen::Event::Wheel(_) => "wheel".into(),
                        screen::Event::Closed => "closed".into(),
                    })
                    .collect();
                let point = screen::get_cursor();
                let origin = screen::get_menu_origin().map(|p| p.x);
                let shift = screen::get_modifiers().shift;
                Ok(Some(format!(
                    "browser items {} known {known} recipes {} width {} screen {:?} cursor {},{} origin {origin:?} shift {shift} at {:?} events {}",
                    items.len(),
                    recipes.len(),
                    screen::get_text_width("ab"),
                    screen::get_game_screen(),
                    point.x,
                    point.y,
                    screen::get_item_at(20.0, 20.0),
                    kinds.join(","),
                )))
            }
            "save" => {
                storage::set("note", args.as_bytes())?;
                Ok(Some("saved".into()))
            }
            "load" => {
                let text =
                    storage::get("note").map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
                chat::show(text.as_deref().unwrap_or("(empty)"));
                Ok(None)
            }
            "load_key" => Ok(Some(
                storage::get(args.trim()).map_or("(none)".into(), |b| {
                    String::from_utf8_lossy(&b).into_owned()
                }),
            )),
            "save_key" => {
                let (key, text) = args
                    .split_once(' ')
                    .ok_or("Usage: /save_key <key> <text>")?;
                storage::set(key, text.as_bytes())?;
                Ok(Some("saved".into()))
            }
            _ => {
                chat::show(&format!("hello {args} after {} ticks", self.ticks));
                Ok(None)
            }
        }
    }
}

horizon_mod_sdk::export_mod!(Guest);
