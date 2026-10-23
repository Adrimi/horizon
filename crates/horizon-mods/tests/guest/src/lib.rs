//! A mod for the runtime's tests: `/spin` loops forever, `/panic` panics,
//! `/grow` asks for a gigabyte, `/hello` chats, `/tool` names the hotbar's
//! fastest slot for the target and selects it, `/select a b ...` selects
//! each slot in turn, `/food` reads hunger and the selected slot's food;
//! `/tags <id> <slot>` reads an item's tags and a hotbar slot's;
//! `tick` and `frame` do nothing, to measure the cost of
//! a call.
use horizon_mod_sdk::{
    Mod, Registration, chat, game, hotbar, inventory, items, keys, recipes, screen, storage,
    target, world,
};

struct Guest {
    ticks: u64,
    /// The screen the mod opened, 0 when it has none.
    screen: u32,
    /// The panel the mod opened, 0 when it has none.
    panel: u32,
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
            "save",
            "load",
            "save_key",
            "load_key",
        ]
        .into_iter()
        .fold(Registration::new(), Registration::command)
        .on_tick()
        .on_frame();
        Ok((
            Guest {
                ticks: 0,
                screen: 0,
                panel: 0,
            },
            hooks,
        ))
    }
    fn tick(&mut self, _tick: u64) {
        self.ticks += 1;
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
                            name: "hud/heart/full".into(),
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
