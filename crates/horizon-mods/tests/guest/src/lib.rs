//! A mod for the runtime's tests: `/spin` loops forever, `/panic` panics,
//! `/grow` asks for a gigabyte, `/hello` chats, `/tool` names the hotbar's
//! fastest slot for the target and selects it, `/select a b ...` selects
//! each slot in turn, `/food` reads hunger and the selected slot's food;
//! `/tags <id> <slot>` reads an item's tags and a hotbar slot's;
//! `tick` and `frame` do nothing, to measure the cost of
//! a call.
use horizon_mod_sdk::{
    Mod, Registration, atmosphere, audio, chat, chat_decorate, debug, entity, exchange, game,
    hotbar, hud, input, inventory, items, keys, light_events, lighting, overlay, particles,
    players, recipes, resources, screen, settings, slots, storage, target, texture_rules, tooltip,
    weather, world,
};

struct Guest {
    ticks: u64,
    /// The screen the mod opened, 0 when it has none.
    screen: u32,
    /// The panel the mod opened, 0 when it has none.
    panel: u32,
    /// The random ticks heard so far, and the last block's description.
    heard: u32,
    /// The removed blocks the tick hook saw, and the last one.
    removed_seen: String,
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
            "bigtexture",
            "mapdetails",
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
            "playersfx",
            "detailfx",
            "typesfx",
            "parityfx",
            "rainfx",
            "restfx",
            "sndfx",
            "crossfx",
            "tipfx",
            "stackfx",
            "rmdef",
            "stylefx",
            "rmfx",
            "timberfx",
            "veinfx",
            "pfx",
            "gsprite",
            "removedfx",
            "minimapfx",
            "leaffx",
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
                removed_seen: String::new(),
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
            .map(|m| {
                format!(
                    "{}:{}:{}",
                    m.id,
                    m.text,
                    m.sender.as_ref().map_or("-", |s| &s.name)
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        if text.contains("deco")
            && let Some(m) = messages.last()
        {
            let region = |x| chat_decorate::Region {
                x,
                y: 8,
                width: 8,
                height: 8,
            };
            chat_decorate::set_decoration(
                m.id,
                Some(&chat_decorate::Decoration {
                    picture: chat_decorate::Picture {
                        source: chat_decorate::Source::SenderSkin,
                        regions: vec![region(8), region(40)],
                        width: 8,
                        height: 8,
                    },
                    gap: Some(1),
                    place: chat_decorate::Place::Text(8),
                    styles: vec![chat_decorate::Style {
                        offset_x: 1,
                        offset_y: 0,
                        width: 0,
                        height: 0,
                        color: 0x40_4040,
                    }],
                }),
            );
            return;
        }
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
        for r in world::removed_blocks() {
            self.removed_seen
                .push_str(&format!("{}:{};", r.block.id, r.by_world));
        }
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
                let origin = resources::file_origin("minecraft", &first);
                Ok(Some(format!(
                    "{} namespaces, {} files, first {first} {size:?}, escape {escape}, origin {origin:?}",
                    spaces.join(","),
                    files.len()
                )))
            }
            "said" => Ok(Some(self.said.clone())),
            "leaffx" => {
                // Silencing, watching blocks by id, removed blocks, a game
                // sprite and a command particle.
                particles::set_suppressed(&["minecraft:tinted_leaves".into()], 0.0);
                world::watch_random_tick_blocks(&["minecraft:pink_wool".into()]);
                let removed = world::removed_blocks().len();
                let sprite = particles::get_game_sprite("minecraft:block/oak_leaves");
                let size = sprite.as_ref().map(|s| (s.width, s.height, s.pixels.len()));
                let picture = sprite
                    .map(|s| {
                        particles::define_sprite(&particles::Sprite {
                            name: "game_leaf".into(),
                            width: s.width,
                            height: s.height,
                            pixels: s.pixels,
                        })
                    })
                    .transpose()?;
                let registered = picture.map(|sprite| {
                    particles::register_command_particle(
                        "leafy",
                        particles::Options {
                            sprite,
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
                            size: 0.2,
                            lifetime: 100,
                            gravity: 0.02,
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
                        },
                    )
                });
                Ok(Some(format!(
                    "removed {removed} sprite {size:?} registered {registered:?}"
                )))
            }
            "gsprite" => Ok(Some(format!(
                "{:?}",
                particles::get_game_sprite(args).map(|s| {
                    let a: Vec<u8> = s.pixels.chunks(4).map(|p| p[3]).collect();
                    let opaque = a.iter().filter(|v| **v == 255).count();
                    let clear = a.iter().filter(|v| **v == 0).count();
                    let first: Vec<u8> = s.pixels.iter().take(16).copied().collect();
                    (s.width, s.height, s.pixels.len(), opaque, clear, first)
                })
            ))),
            "pfx" => {
                // N pictures of S by S pixels, each its own colour, in a row at x y z.
                let nums: Vec<f64> = args
                    .split_whitespace()
                    .filter_map(|a| a.parse().ok())
                    .collect();
                let (x, y, z) = (nums[0], nums[1], nums[2]);
                let count = nums.get(3).copied().unwrap_or(2.0) as u32;
                let side = nums.get(4).copied().unwrap_or(16.0) as u32;
                let mut ids = Vec::new();
                for i in 0..count {
                    let rgb = [
                        (i * 97 % 256) as u8,
                        (i * 53 % 256) as u8,
                        (255 - i * 40 % 256) as u8,
                    ];
                    let mut pixels = Vec::new();
                    for _ in 0..side * side {
                        pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                    }
                    ids.push(particles::define_sprite(&particles::Sprite {
                        name: format!("c{i}"),
                        width: side,
                        height: side,
                        pixels,
                    })?);
                }
                for (i, id) in ids.iter().enumerate() {
                    particles::spawn(particles::Options {
                        sprite: *id,
                        x: x + (i % 8) as f64 * 0.5,
                        y: y + (i / 8) as f64 * 0.5,
                        z,
                        vx: 0.0,
                        vy: 0.0,
                        vz: 0.0,
                        color: particles::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        },
                        size: 0.2,
                        lifetime: 1000,
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
                }
                Ok(Some(format!("sprites {ids:?}")))
            }
            "veinfx" => {
                let seen: Vec<String> = input::poll()
                    .iter()
                    .map(|e| format!("{}/{}/{}", e.wheel, e.button, e.pressed))
                    .collect();
                input::set_wheel_captured(true);
                game::add_exhaustion(1.5);
                game::set_break_speed_modifier(0.5);
                let merged = world::merge_drops_at(world::BlockPos { x: 1, y: 64, z: 1 });
                let (x, y, z): (f64, f64, f64) = {
                    let nums: Vec<f64> = args
                        .split_whitespace()
                        .filter_map(|a| a.parse().ok())
                        .collect();
                    if nums.len() == 3 {
                        (nums[0], nums[1], nums[2])
                    } else {
                        (1.0, 64.0, 1.0)
                    }
                };
                overlay::paint_world(&[
                    overlay::WorldElement::Box(overlay::WorldBox {
                        x,
                        y,
                        z,
                        color: 0xFFFF_FFFF,
                        width: 2.0,
                        through_blocks: false,
                    }),
                    overlay::WorldElement::Box(overlay::WorldBox {
                        x: x + 2.0,
                        y,
                        z,
                        color: 0xFFFF_0000,
                        width: 2.0,
                        through_blocks: true,
                    }),
                ]);
                Ok(Some(format!("events {} merged {merged}", seen.join(","))))
            }
            "timberfx" => {
                let nums: Vec<i32> = args
                    .split_whitespace()
                    .filter_map(|a| a.parse().ok())
                    .collect();
                let pos = if nums.len() == 3 {
                    world::BlockPos {
                        x: nums[0],
                        y: nums[1],
                        z: nums[2],
                    }
                } else {
                    world::BlockPos { x: 0, y: 64, z: 0 }
                };
                let mode = game::get_game_mode();
                let tags = game::get_player_tags();
                let fell = world::fall_block(pos, 0.1, 0.0, 0.0, true, args.contains("place"));
                let put = world::set_block(
                    world::BlockPos {
                        x: pos.x + 3,
                        y: pos.y,
                        z: pos.z,
                    },
                    "minecraft:oak_log",
                    &[("axis".to_string(), "x".to_string())],
                );
                let item = world::spawn_item(pos, "minecraft:stick", 3);
                let bad_item = world::spawn_item(pos, "minecraft:stick", 0);
                Ok(Some(format!(
                    "mode {mode:?} tags {tags:?} fell {fell} put {put} item {item} {bad_item}"
                )))
            }
            "restfx" => {
                weather::set_default_splashing(false);
                world::set_weather_passthrough(&[
                    "minecraft:oak_fence".into(),
                    "#minecraft:fences".into(),
                ]);
                let n = particles::get_particles().len();
                let clouds = game::get_cloud_height();
                let bad_name = audio::register("Bad Name", &[1, 2]).is_err();
                let bad_file = audio::register("ok", &[1, 2, 3, 4]).is_err();
                let fake = audio::register("fake", b"OggS-not-really").is_ok();
                let wav = audio::register("tone", b"RIFF\x00\x00\x00\x00WAVEfmt ").is_ok();
                Ok(Some(format!(
                    "particles {n} clouds {clouds:?} {bad_name} {bad_file} {fake} {wav}"
                )))
            }
            "stackfx" => {
                let Some(stack) = tooltip::get_hovered_stack() else {
                    tooltip::set_hovered_tooltip(None);
                    return Ok(Some("none".into()));
                };
                let first = stack.container.first().cloned().flatten();
                tooltip::set_hovered_tooltip(Some(&tooltip::HoveredTooltip {
                    lines: Some(vec![screen::TipLine {
                        text: "Contains".into(),
                        color: 0xFFFF_FFFF,
                    }]),
                    width: 200.0,
                    height: 70.0,
                    elements: (0..100)
                        .map(|i| {
                            screen::Element::Fill(screen::FillElement {
                                x: i as f32,
                                y: 0.0,
                                width: 1.0,
                                height: 1.0,
                                top: 0xFF00_0000,
                                bottom: 0xFF00_0000,
                            })
                        })
                        .collect(),
                }));
                Ok(Some(format!(
                    "{} {} {} {:?}",
                    stack.item,
                    stack.count,
                    stack.container.len(),
                    first.map(|f| (f.id, f.count))
                )))
            }
            "tipfx" => {
                let found = tooltip::get_hovered_enchantments();
                let station = screen::get_station();
                tooltip::set_lines(&[
                    tooltip::Line {
                        under: "enchantment.minecraft.sharpness".into(),
                        text: " Sharp".into(),
                        color: 0xFF55_5555,
                    },
                    tooltip::Line {
                        under: String::new(),
                        text: "Hold shift".into(),
                        color: 0xFFAA_AAAA,
                    },
                ]);
                let published = exchange::publish("descriptions", b"abc").is_ok();
                let bad = exchange::publish("Bad Key", b"x").is_err();
                let read = exchange::read("test_guest", "descriptions");
                let keys = exchange::list_keys("test_guest");
                let none = exchange::read("nobody", "descriptions").is_none();
                Ok(Some(format!(
                    "{} {} {:?} {station:?} {published} {bad} {read:?} {keys:?} {none}",
                    found.len(),
                    found.first().map_or(0, |e| e.level),
                    found.first().map(|e| e.stored)
                )))
            }
            "stylefx" => {
                let id = particles::define_sprite(&particles::Sprite {
                    name: "drop".into(),
                    width: 2,
                    height: 2,
                    pixels: vec![255; 16],
                })?;
                let ok = particles::spawn_styled(
                    particles::Options {
                        sprite: id,
                        x: 0.0,
                        y: 70.0,
                        z: 0.0,
                        vx: 0.0,
                        vy: 0.0,
                        vz: 0.0,
                        size: 0.1,
                        lifetime: 40,
                        gravity: 0.04,
                        wind_x: 0.0,
                        wind_z: 0.0,
                        collide: true,
                        color: particles::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.5,
                        },
                        size_curve: particles::SizeCurve::Constant,
                        fade: particles::Fade::None,
                        flat: false,
                        linger: 0,
                        sway: 0.0,
                        sway_period: 20,
                        spin: 0.0,
                        tumble: 0.0,
                    },
                    particles::Rotation::Camera,
                    particles::Style {
                        bounciness: 2.0,
                        render_type: particles::RenderType::Opaque,
                        distance_scaling: f32::NAN,
                    },
                );
                Ok(Some(format!("{ok}")))
            }
            "rmdef" => {
                let id = particles::define_sprite(&particles::Sprite {
                    name: "drop".into(),
                    width: 2,
                    height: 2,
                    pixels: vec![255; 16],
                })?;
                Ok(Some(format!("{id}")))
            }
            "rmfx" => {
                let nums: Vec<f64> = args
                    .split_whitespace()
                    .filter_map(|a| a.parse().ok())
                    .collect();
                let n = particles::remove(nums[0] as u32, nums[1], nums[2], nums[3]);
                let other = particles::remove(999, nums[1], nums[2], nums[3]);
                let file = audio::register_file("fake").is_ok();
                let missing = audio::register_file("nofile").is_err();
                let bad = audio::register_file("Bad Name").is_err();
                Ok(Some(format!("removed {n} {other} {file} {missing} {bad}")))
            }
            "crossfx" => {
                let nums: Vec<f64> = args
                    .split_whitespace()
                    .filter_map(|a| a.parse().ok())
                    .collect();
                let img =
                    particles::get_game_sprite("minecraft:block/dead_bush").ok_or("no sprite")?;
                let id = particles::define_sprite(&particles::Sprite {
                    name: "bush".into(),
                    width: img.width,
                    height: img.height,
                    pixels: img.pixels,
                })?;
                let ok = particles::spawn_rotated(
                    particles::Options {
                        sprite: id,
                        x: nums[0],
                        y: nums[1],
                        z: nums[2],
                        vx: 0.0,
                        vy: 0.0,
                        vz: 0.0,
                        color: particles::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        },
                        size: 0.5,
                        lifetime: 1000,
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
                        spin: 0.05,
                        tumble: 0.0,
                    },
                    particles::Rotation::Crossed,
                );
                Ok(Some(format!("bush {id} {ok}")))
            }
            "sndfx" => {
                let Some(bytes) = resources::read_file("minecraft", "sounds/step/stone6.ogg")
                else {
                    return Ok(Some("no file".into()));
                };
                let registered = audio::register("hit", &bytes);
                let played = audio::play("test_guest:hit", None, 1.0, 1.0);
                Ok(Some(format!(
                    "size {} registered {registered:?} played {played}",
                    bytes.len()
                )))
            }
            "rainfx" => {
                let sprite = particles::define_sprite(&particles::Sprite {
                    name: "drop".into(),
                    width: 2,
                    height: 2,
                    pixels: vec![255; 16],
                })?;
                let rotated = particles::spawn_rotated(
                    particles::Options {
                        sprite,
                        x: 0.5,
                        y: 70.0,
                        z: 0.5,
                        vx: 0.3,
                        vy: -0.9,
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
                    },
                    particles::Rotation::RelativeVelocity,
                );
                weather::set_default_weather(false);
                let fog = world::get_biome_fog(world::BlockPos { x: 1, y: 64, z: 1 });
                let sound = audio::play("minecraft:weather.rain", None, 0.2, 1.0);
                let bad = audio::play("Bad Name", None, 1.0, 1.0);
                Ok(Some(format!(
                    "rotated {rotated} fog {fog:?} sound {sound} bad {bad}"
                )))
            }
            "removedfx" => Ok(Some(self.removed_seen.clone())),
            "minimapfx" => {
                let light = world::get_light_level(world::BlockPos { x: 1, y: 64, z: 1 })
                    .map(|l| (l.block, l.sky));
                let command = world::run_command("/tp 10 64 -20");
                if command {
                    overlay::paint_world(&[overlay::WorldElement::Picture(
                        overlay::WorldPicture {
                            x: 1.0,
                            y: 65.0,
                            z: 1.0,
                            width: 8.0,
                            height: 12.0,
                            texture: 0,
                            u: 0.0,
                            v: 0.0,
                            u_width: 8.0,
                            v_height: 12.0,
                            tint: 0xFFFF_FFFF,
                            offset_y: 0.0,
                        },
                    )]);
                }
                Ok(Some(format!("light {light:?} command {command}")))
            }
            "detailfx" => {
                let mut lines: Vec<String> = entity::nearby("", 50.0)
                    .iter()
                    .map(|e| match entity::get_detail(e.id) {
                        Some(d) => format!(
                            "{}:{:.2}:{}:{}:{:.1}:{}:{}",
                            e.type_id,
                            d.swell,
                            d.glowing,
                            d.in_water,
                            d.squish,
                            d.dark_ticks,
                            d.carried_block
                        ),
                        None => format!("{}:none", e.type_id),
                    })
                    .collect();
                lines.push(format!("gone:{}", entity::get_detail(4_000_000).is_none()));
                lines.extend(light_events::poll().iter().map(|f| {
                    format!(
                        "{}@{},{},{}->{}:{}",
                        f.kind, f.x, f.y, f.z, f.end_y, f.ticks
                    )
                }));
                Ok(Some(lines.join("|")))
            }
            "parityfx" => {
                let effects = target::get_entity_effects()
                    .map(|e| {
                        e.iter()
                            .map(|e| {
                                format!(
                                    "{}:{}:{}:{}:{}",
                                    e.id, e.amplifier, e.duration_ticks, e.ambient, e.visible
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_else(|| "none".into());
                let ex = |reach: f32, eye: bool| {
                    format!(
                        "{}/{}/{}",
                        target::get_block_ex(reach, eye).map_or("-".into(), |b| b.id),
                        target::get_block_face_ex(reach, eye).unwrap_or_else(|| "-".into()),
                        target::get_entity_ex(reach, eye).map_or("-".into(), |e| e.type_id)
                    )
                };
                let fluid = |m: &str| {
                    target::get_fluid(m).map_or("-".into(), |f| format!("{}#{}", f.id, f.level))
                };
                let bars: Vec<String> = hud::get_boss_bars()
                    .iter()
                    .map(|b| format!("{}@{}", b.name, b.y_bottom))
                    .collect();
                Ok(Some(format!(
                    "{effects}|{} {} {}|{} {} {}|{}|{}|{}|{}|{}|{}",
                    ex(5.0, true),
                    ex(1.0, true),
                    ex(10.0, false),
                    fluid("any"),
                    fluid("fallback"),
                    fluid("none"),
                    game::get_main_hand(),
                    overlay::is_tab_list_open(),
                    bars.join(","),
                    "",
                    "",
                    ""
                )))
            }
            "typesfx" => {
                let types = entity::get_types();
                let mut lines: Vec<String> = types
                    .iter()
                    .map(|t| format!("{}={}[{}]", t.id, t.name, t.tags.join(",")))
                    .collect();
                lines.push(format!(
                    "invisible:{}:{}",
                    entity::is_invisible(1),
                    entity::is_invisible(2)
                ));
                Ok(Some(lines.join("|")))
            }
            "playersfx" => Ok(Some(
                players::get_players()
                    .iter()
                    .map(|p| format!("{}/{}/{}", p.name, p.display_name, p.uuid))
                    .collect::<Vec<_>>()
                    .join(","),
            )),
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
                        &args
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<Vec<_>>(),
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
            "mapdetails" => {
                let d = world::get_chunk_details(0, 0).ok_or("no details")?;
                let none = world::get_chunk_details(5, 5).is_none();
                let v = game::get_viewport().ok_or("no viewport")?;
                Ok(Some(format!(
                    "details {} {} {} {} {} viewport {} {} {} view {}",
                    d.block[0],
                    d.ambiguous[0],
                    d.light[0],
                    d.block.len(),
                    if none { "none" } else { "some" },
                    v.gui_scale,
                    v.width,
                    v.height,
                    game::get_server_view_distance()
                )))
            }
            "bigtexture" => {
                // Large screen images and their filter (API 0.1.31).
                if self.screen == 0 {
                    self.screen = screen::open("Test screen")?;
                }
                let big = screen::upload_texture(1, 512, 512, &vec![255; 512 * 512 * 4]).is_ok();
                let too_big =
                    screen::upload_texture(2, 1024, 1024, &vec![255; 1024 * 1024 * 4]).is_err();
                screen::set_texture_filter(1, screen::TextureFilter::Linear);
                Ok(Some(format!(
                    "big {big} refused {too_big} max {}",
                    screen::get_max_texture_side()
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
                let uploaded = slots::upload_texture(0, 2, 2, &[255; 16]).is_ok();
                let too_big = slots::upload_texture(9, 2, 2, &[255; 16]).is_err();
                slots::set_screen_marks(&[slots::Mark {
                    slot: 1,
                    shape: slots::MarkShape::Picture(slots::PictureMark {
                        texture: 0,
                        width: 8.0,
                        height: 8.0,
                        x: 0.0,
                        y: 0.0,
                        color: 0xFFFF_FFFF,
                    }),
                }]);
                slots::set_hud_marks(&[slots::Mark {
                    slot: 2,
                    shape: slots::MarkShape::Square(slots::SquareMark {
                        color: 0xFF00_00FF,
                        size: 3.0,
                        x: 13.0,
                        y: 13.0,
                    }),
                }]);
                let color = items::get_name_color("minecraft:stone");
                Ok(Some(format!(
                    "uploaded {uploaded} {too_big} color {color:?} slots {summary:?} crafting {crafting:?} session {session:?} hovered {:?}",
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
