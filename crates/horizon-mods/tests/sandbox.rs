//! Loads real mods built with the SDK: the coords-hud example and a guest
//! that misbehaves on command. Needs `rustup target add wasm32-unknown-unknown`.
use horizon_mods::{
    Block, BlockBroken, BlockPos, Clock, Corner, Entity, Food, FoodStats, ItemTags, Mods, Output,
    Package, Player, Snapshot, Stack, Status, WorldAccess, WorldBlock, manifest::componentize,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::Instant,
};

const GUEST_MANIFEST: &str = r#"
id = "test_guest"
name = "Test Guest"
version = "0.1.0"
api = "0.1"
capabilities = ["chat", "game-read", "target", "inventory-read", "hotbar", "world-read", "world-edit", "keys", "storage", "screen", "items-read", "recipes-read", "inventory-edit", "weather-read", "settings-read", "particles", "atmosphere", "overlay", "hud", "entity-read", "entity-edit", "slots-read", "slot-decorate", "tooltip", "debug"]
"#;

/// Builds both mods once and writes their `.hmod` files; returns the folder.
fn mods_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let target = root.join("target/mods-test");
        let status = Command::new(env!("CARGO"))
            .current_dir(&root)
            .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
            .args([
                "-p",
                "coords-hud",
                "-p",
                "horizon-mods-test-guest",
                "--target-dir",
            ])
            .arg(&target)
            .status()
            .expect("cargo runs");
        assert!(
            status.success(),
            "building the test mods needs `rustup target add wasm32-unknown-unknown`"
        );
        let built = target.join("wasm32-unknown-unknown/release");
        let dir = target.join("mods");
        std::fs::create_dir_all(&dir).unwrap();
        let example = root.join("crates/horizon-mod-sdk/examples/coords-hud/horizon-mod.toml");
        let mods = [
            ("coords_hud", std::fs::read_to_string(example).unwrap()),
            ("horizon_mods_test_guest", GUEST_MANIFEST.to_string()),
        ];
        for (name, manifest) in mods {
            let core = std::fs::read(built.join(format!("{name}.wasm"))).unwrap();
            let hmod = Package::write(&manifest, &componentize(&core).unwrap()).unwrap();
            std::fs::write(dir.join(format!("{name}.hmod")), hmod).unwrap();
        }
        dir
    })
}

fn load(name: &str) -> Mods {
    Mods::load(&[mods_dir().join(name)], |_| true, |c| c == "give")
}

fn player() -> Snapshot {
    Snapshot {
        player: Some(Player {
            x: 12.5,
            y: 64.0,
            z: -3.25,
            yaw: 180.0,
            pitch: 0.0,
            health: 20.0,
            food: 20,
            dimension: "minecraft:overworld".into(),
        }),
        clock: Some(Clock {
            game_time: 100,
            day_time: 24000 + 6000,
        }),
        ..Snapshot::default()
    }
}

#[test]
fn coords_hud_draws_and_answers() {
    let mut mods = load("coords_hud.hmod");
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert_eq!(mods.commands().collect::<Vec<_>>(), ["where", "coords"]);
    mods.set_snapshot(player());
    mods.tick(1);
    let panel = mods.panels().next().expect("a panel after a tick");
    assert_eq!(panel.corner, Corner::TopLeft);
    let lines: Vec<&str> = panel.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        lines,
        [
            "XYZ 12.5 / 64.0 / -3.2",
            "Facing North (-Z)",
            "Day 2, 12:00"
        ]
    );

    assert!(mods.command("where"));
    let chat = Output::Chat {
        name: "Coordinates HUD".into(),
        text: "12, 64, -4 in minecraft:overworld".into(),
    };
    assert_eq!(mods.drain(), [chat]);
    assert!(mods.command("coords"));
    mods.tick(2);
    assert_eq!(mods.panels().count(), 0);
    let hidden = Output::Feedback {
        text: "Coordinates panel hidden".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [hidden]);
    assert!(!mods.command("gamemode creative"));
}

#[test]
fn off_mods_cost_nothing() {
    let mods = Mods::load(&[mods_dir().join("coords_hud.hmod")], |_| false, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Off);
    assert!(!mods.is_running());
    assert_eq!(mods.commands().count(), 0);
}

#[test]
fn undeclared_capabilities_are_refused() {
    let dir = mods_dir().join("undeclared");
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = Package::read(&mods_dir().join("horizon_mods_test_guest.hmod"))
        .unwrap()
        .wasm
        .expect("the guest has code");
    let manifest = GUEST_MANIFEST.replace("\"chat\", ", "");
    std::fs::write(dir.join("horizon-mod.toml"), manifest).unwrap();
    std::fs::write(dir.join("mod.wasm"), wasm).unwrap();
    let mods = Mods::load(&[dir], |_| true, |_| false);
    let refused = Status::Failed("uses capability chat, missing from horizon-mod.toml".into());
    assert_eq!(mods.entries()[0].status, refused);
}

fn stack(id: &str, mining_speed: f32, correct_for_drops: bool) -> Option<Stack> {
    Some(Stack {
        id: id.into(),
        count: 1,
        damage: 3,
        max_damage: 250,
        efficiency: 0,
        mining_speed,
        correct_for_drops,
    })
}

#[test]
fn the_hotbar_api_reads_the_target_and_selects_a_slot() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    let stone = Block {
        x: 1,
        y: 60,
        z: -2,
        id: "minecraft:stone".into(),
        hardness: 1.5,
        requires_tool: true,
        tags: vec!["minecraft:mineable/pickaxe".into()],
    };
    let mut hotbar = vec![None; 9];
    hotbar[0] = stack("minecraft:wooden_shovel", 1.0, false);
    hotbar[2] = stack("minecraft:iron_pickaxe", 6.0, true);
    hotbar[5] = stack("minecraft:stone_pickaxe", 4.0, true);
    mods.set_snapshot(Snapshot {
        block: Some(stone),
        hotbar,
        selected: 7,
        ..player()
    });
    assert!(mods.command("tool"));
    let feedback = Output::Feedback {
        text: "minecraft:stone at 1 60 -2: slot 2 minecraft:iron_pickaxe x1 (3/250, efficiency 0, speed 6, drops) of 9, was 7, tags minecraft:mineable/pickaxe".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [Output::Select { slot: 2 }, feedback]);
    // The last slot asked for in a call wins; out of range is ignored.
    assert!(mods.command("select 3 12 5"));
    assert_eq!(mods.drain(), [Output::Select { slot: 5 }]);
    assert!(mods.command("select 9"));
    assert_eq!(mods.drain(), []);
    // Nothing targeted: the mod says so.
    mods.set_snapshot(player());
    assert!(mods.command("tool"));
    let none = Output::Feedback {
        text: "no block".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [none]);
}

#[test]
fn the_food_api_reads_hunger_and_the_hotbar_food() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    let mut hotbar_food = vec![None; 9];
    hotbar_food[4] = Some(Food {
        nutrition: 8,
        saturation: 12.8,
    });
    mods.set_snapshot(Snapshot {
        food: Some(FoodStats {
            food: 14,
            saturation: 2.5,
            exhaustion: 1.25,
        }),
        hotbar_food,
        selected: 4,
        ..player()
    });
    assert!(mods.command("food"));
    let feedback = Output::Feedback {
        text: "food 14 saturation 2.5 exhaustion 1.25, held +8 +12.8, slot 9 None".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [feedback]);
    // A slot that is not food, and the title screen.
    mods.set_snapshot(Snapshot {
        food: Some(FoodStats {
            food: 20,
            saturation: 5.0,
            exhaustion: 0.0,
        }),
        ..player()
    });
    assert!(mods.command("food"));
    let not_food = Output::Feedback {
        text: "food 20 saturation 5 exhaustion 0, held not food, slot 9 None".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [not_food]);
    mods.set_snapshot(Snapshot::default());
    assert!(mods.command("food"));
    let none = Output::Feedback {
        text: "no player".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [none]);
}

#[test]
fn the_item_tags_api_reads_tags_by_item_and_by_slot() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    let mut hotbar = vec![None; 9];
    hotbar[3] = Some(Stack {
        id: "minecraft:iron_axe".into(),
        count: 1,
        damage: 0,
        max_damage: 250,
        efficiency: 0,
        mining_speed: 6.0,
        correct_for_drops: true,
    });
    // `minecraft:tools` nests `minecraft:axes` in vanilla; the game hands the
    // runtime every tag already expanded, so each list is complete.
    let tags = ItemTags::new([
        (
            "minecraft:axes".to_string(),
            vec![
                "minecraft:diamond_axe".to_string(),
                "minecraft:iron_axe".to_string(),
            ],
        ),
        (
            "minecraft:tools".to_string(),
            vec![
                "minecraft:iron_axe".to_string(),
                "minecraft:iron_pickaxe".to_string(),
            ],
        ),
    ]);
    mods.set_snapshot(Snapshot {
        hotbar,
        item_tags: Some(std::sync::Arc::new(tags)),
        ..player()
    });
    assert!(mods.command("tags minecraft:iron_axe 3"));
    assert_eq!(
        mods.drain(),
        [feedback(
            "minecraft:iron_axe: minecraft:axes minecraft:tools, slot 3 minecraft:axes minecraft:tools"
        )]
    );
    // An unknown id and an empty slot are empty lists, not errors.
    assert!(mods.command("tags minecraft:bedrock 4"));
    assert_eq!(mods.drain(), [feedback("minecraft:bedrock: , slot 4 ")]);
    // Nothing shared without a running mod reading the inventory.
    mods.set_snapshot(Snapshot {
        hotbar: vec![
            Some(Stack {
                id: "minecraft:iron_axe".into(),
                count: 1,
                damage: 0,
                max_damage: 250,
                efficiency: 0,
                mining_speed: 6.0,
                correct_for_drops: true,
            });
            9
        ],
        ..player()
    });
    assert!(mods.command("tags minecraft:iron_axe 3"));
    assert_eq!(mods.drain(), [feedback("minecraft:iron_axe: , slot 3 ")]);
}

#[test]
fn the_target_api_names_the_block_its_progress_and_the_mob() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_snapshot(Snapshot {
        block_name: Some("Stone".into()),
        breaking_progress: Some(0.25),
        entity: Some(Entity {
            id: 41,
            type_id: "minecraft:zombie".into(),
            name: "Zombie".into(),
            health: 12.5,
            max_health: 20.0,
        }),
        ..player()
    });
    assert!(mods.command("look"));
    let feedback = Output::Feedback {
        text: "Stone, 0.25, 41 minecraft:zombie Zombie 12.5/20".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [feedback]);
    mods.set_snapshot(player());
    assert!(mods.command("look"));
    let none = Output::Feedback {
        text: "no block, not breaking, no entity".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [none]);
}

#[test]
fn vanilla_commands_win() {
    let mods = Mods::load(
        &[mods_dir().join("coords_hud.hmod")],
        |_| true,
        |c| c == "where",
    );
    assert_eq!(mods.commands().collect::<Vec<_>>(), ["coords"]);
}

#[test]
fn a_misbehaving_mod_is_stopped_alone() {
    for (command, reason) in [
        ("spin", "it took too long (an endless loop?)"),
        ("panic", "it crashed (a panic)"),
        ("grow", "it crashed (a panic)"),
    ] {
        let paths = [
            mods_dir().join("coords_hud.hmod"),
            mods_dir().join("horizon_mods_test_guest.hmod"),
        ];
        let mut mods = Mods::load(&paths, |_| true, |_| false);
        let started = Instant::now();
        assert!(mods.command(command));
        assert!(started.elapsed().as_secs() < 5, "/{command} was cut short");
        let stopped = Output::Stopped {
            name: "Test Guest".into(),
            reason: reason.into(),
        };
        assert_eq!(mods.drain(), [stopped], "/{command}");
        assert_eq!(mods.entries()[1].status, Status::Failed(reason.into()));
        // The other mod runs on, and the stopped one's command is gone.
        assert!(mods.is_running());
        assert!(!mods.command("hello"));
        mods.set_snapshot(player());
        mods.tick(1);
        assert_eq!(mods.panels().count(), 1);
    }
}

#[test]
fn hot_reload_restarts_a_changed_mod() {
    let dir = mods_dir().join("reload");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    mods.tick(1);
    assert!(!mods.reload_changed(|_| false));
    // A rebuild: same bytes, newer file.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(mods.reload_changed(|_| false));
    assert!(mods.command("hello world"));
    let chat = Output::Chat {
        name: "Test Guest".into(),
        text: "hello world after 0 ticks".into(),
    };
    assert_eq!(mods.drain(), [chat]);
}

#[test]
fn storage_persists_across_reload() {
    let dir = mods_dir().join("storage_reload");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    assert!(mods.command("save remembered"));
    let saved = Output::Feedback {
        text: "saved".into(),
        error: false,
    };
    assert_eq!(mods.drain(), [saved]);

    // A rebuild (hot_reload_restarts_a_changed_mod's pattern): a fresh Store,
    // but the same on-disk data directory.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(mods.reload_changed(|_| false));

    assert!(mods.command("load"));
    let chat = Output::Chat {
        name: "Test Guest".into(),
        text: "remembered".into(),
    };
    assert_eq!(mods.drain(), [chat]);
}

#[test]
fn storage_write_budget_limits_one_call() {
    let dir = mods_dir().join("storage_budget");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    // Under the 4 MiB on-disk quota, but over the 1 MiB a single call may
    // write: a loop replacing one key can never bypass the quota by
    // writing a lot in one call, only across many.
    let over_budget = "a".repeat((1 << 20) + 1024);
    assert!(mods.command(&format!("save {over_budget}")));
    let refused = Output::Feedback {
        text: "too much storage traffic in one call".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [refused]);
}

#[test]
fn storage_quota_is_enforced() {
    let dir = mods_dir().join("storage_quota");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    // Each call is under the per-call write budget; distinct keys so the
    // quota (not the fixed "note" key being overwritten) is what's tested.
    let chunk = "a".repeat(900 << 10);
    for i in 0..4 {
        assert!(mods.command(&format!("save_key k{i} {chunk}")));
        assert_eq!(
            mods.drain(),
            [Output::Feedback {
                text: "saved".into(),
                error: false,
            }]
        );
    }
    assert!(mods.command(&format!("save_key k4 {chunk}")));
    let refused = Output::Feedback {
        text: "would use over 4 MiB of storage".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [refused]);
}

#[test]
fn storage_key_count_is_bounded() {
    let dir = mods_dir().join("storage_keys");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("guest.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    for i in 0..64 {
        assert!(mods.command(&format!("save_key k{i} x")));
        assert_eq!(
            mods.drain(),
            [Output::Feedback {
                text: "saved".into(),
                error: false,
            }],
            "key {i}"
        );
    }
    assert!(mods.command("save_key k64 x"));
    let refused = Output::Feedback {
        text: "would hold over 64 keys".into(),
        error: true,
    };
    assert_eq!(mods.drain(), [refused]);
}

#[test]
fn duplicate_mod_ids_are_refused() {
    let dir = mods_dir().join("duplicate_ids");
    std::fs::create_dir_all(&dir).unwrap();
    let first = dir.join("first.hmod");
    let second = dir.join("second.hmod");
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &first).unwrap();
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &second).unwrap();
    // Both declare id "test_guest" (the guest's own manifest): sharing a
    // `data_dir` would let the second read, overwrite or delete the
    // first's storage, so only the first-loaded one may start.
    let mods = Mods::load(&[first, second], |_| true, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Running);
    let refused = Status::Failed("another running mod already uses id test_guest".into());
    assert_eq!(mods.entries()[1].status, refused);
}

/// `cargo test -p horizon-mods --release -- --ignored --nocapture`: the
/// cost of the per-tick and per-frame hooks (docs/mods.md, Performance).
#[test]
#[ignore]
fn hook_overhead() {
    let time = |mods: &mut Mods, frame: bool| {
        let n = 200_000;
        for i in 0..1000 {
            mods.tick(i);
        }
        let started = Instant::now();
        for i in 0..n {
            if frame {
                mods.frame(0.016);
            } else {
                mods.tick(i);
            }
        }
        started.elapsed().as_nanos() as f64 / n as f64
    };
    let mut guest = load("horizon_mods_test_guest.hmod");
    let mut hud = load("coords_hud.hmod");
    hud.set_snapshot(player());
    let mut none = Mods::default();
    println!("no mods, tick:        {:8.1} ns", time(&mut none, false));
    println!("empty tick hook:      {:8.1} ns", time(&mut guest, false));
    println!("empty frame hook:     {:8.1} ns", time(&mut guest, true));
    println!("coords-hud tick hook: {:8.1} ns", time(&mut hud, false));
    let started = Instant::now();
    let _ = load("coords_hud.hmod");
    println!(
        "load coords-hud:      {:8.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn content_only_mods_load_without_an_engine() {
    let manifest = r#"
id = "ruby"
name = "Ruby"
version = "0.1.0"
api = "0.2"
side = "both"
capabilities = ["registries"]
"#;
    let block: &[u8] = br#"{"template": "minecraft:iron_block"}"#;
    let dir = mods_dir().join("content");
    std::fs::create_dir_all(&dir).unwrap();
    let ok = dir.join("ruby.hmod");
    let files: &[(&str, &[u8])] = &[("data/ruby/horizon/block/ruby_block.json", block)];
    std::fs::write(&ok, Package::write_files(manifest, None, files).unwrap()).unwrap();
    let bad = dir.join("bad.hmod");
    let files: &[(&str, &[u8])] = &[("data/minecraft/recipe/stone.json", b"{}")];
    let bad_manifest = manifest.replace("\"ruby\"", "\"bad\"");
    std::fs::write(
        &bad,
        Package::write_files(&bad_manifest, None, files).unwrap(),
    )
    .unwrap();
    let mods = Mods::load(&[ok, bad], |_| true, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert!(matches!(&mods.entries()[1].status, Status::Failed(e) if e.contains("own namespace")));
    // Content has no hooks: the game's per-tick mod work stays skipped.
    assert!(!mods.is_running());
}

/// A few blocks: an oak log at 0 64 0, bedrock at 0 0 0; the hand holds a
/// tool at 10 of 59 damage.
struct FakeWorld;
impl horizon_mods::WorldAccess for FakeWorld {
    fn block(&self, pos: &BlockPos) -> Option<WorldBlock> {
        let (id, properties, tags) = match (pos.x, pos.y, pos.z) {
            (0, 64, 0) => (
                "minecraft:oak_log",
                vec![("axis".into(), "y".into())],
                vec!["minecraft:logs".into()],
            ),
            (0, 0, 0) => ("minecraft:bedrock", Vec::new(), Vec::new()),
            (x, _, z) if x.abs() < 16 && z.abs() < 16 => ("minecraft:air", Vec::new(), Vec::new()),
            _ => return None,
        };
        Some(WorldBlock {
            pos: *pos,
            id: id.into(),
            properties,
            tags,
        })
    }
    fn chunk_columns(&self, cx: i32, cz: i32, y: i32) -> Option<horizon_mods::ChunkSurface> {
        let mut surface = self.chunk_surface(cx, cz)?;
        surface.y.fill(y);
        Some(surface)
    }
    fn biome(&self, pos: &BlockPos) -> Option<horizon_mods::Biome> {
        (pos.x.abs() < 16).then(|| horizon_mods::Biome {
            id: "minecraft:plains".into(),
            temperature: 0.8,
            precipitation: true,
            downfall: 0.4,
            foliage: 0x77AB2F,
            grass: 0x91BD59,
            water: 0x3F76E4,
        })
    }
    fn surface(&self, pos: &BlockPos, from: i32) -> Option<i32> {
        (pos.x.abs() < 16).then_some(from.min(65))
    }
    fn solid(&self, pos: &BlockPos) -> bool {
        self.block(pos).is_some_and(|b| b.id == "minecraft:oak_log")
    }
    fn breakable(&self, pos: &BlockPos) -> bool {
        self.block(pos)
            .is_some_and(|b| b.id != "minecraft:air" && b.id != "minecraft:bedrock")
    }
    fn held_wear(&self) -> Option<(u32, u32)> {
        Some((10, 59))
    }
    fn chunk_surface(&self, cx: i32, cz: i32) -> Option<horizon_mods::ChunkSurface> {
        if cx != 0 || cz != 0 {
            return None;
        }
        Some(horizon_mods::ChunkSurface {
            color: vec![4; 256],
            y: vec![64; 256],
            depth: vec![0; 256],
            biome: vec![1; 256],
        })
    }
    fn biome_names(&self) -> Vec<String> {
        vec!["minecraft:plains".into(), "minecraft:forest".into()]
    }
    fn map_colors(&self) -> Vec<u32> {
        vec![0, 0x8B8B8B]
    }
}

fn feedback(text: &str) -> Output {
    Output::Feedback {
        text: text.into(),
        error: false,
    }
}

#[test]
fn the_world_api_reads_blocks_and_queues_edits() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_snapshot(player());
    mods.set_world(Some(std::sync::Arc::new(FakeWorld)));
    assert!(mods.command("block 0 64 0"));
    let log = r#"minecraft:oak_log [("axis", "y")] tags minecraft:logs"#;
    assert_eq!(mods.drain(), [feedback(log)]);
    assert!(mods.command("block 100 0 100"));
    assert_eq!(mods.drain(), [feedback("unloaded")]);
    // A block breaks once per call, after it; bedrock and air never.
    assert!(mods.command("break 0 64 0"));
    let broken = Output::Break {
        pos: [0, 64, 0],
        drops: true,
        with_held: false,
    };
    assert_eq!(mods.drain(), [broken, feedback("true false")]);
    assert!(mods.command("breakheld 0 64 0"));
    let held = Output::Break {
        pos: [0, 64, 0],
        drops: true,
        with_held: true,
    };
    assert_eq!(mods.drain(), [held, feedback("true")]);
    assert!(mods.command("break 0 0 0"));
    assert!(mods.command("break 1 64 1"));
    assert_eq!(
        mods.drain(),
        [feedback("false false"), feedback("false false")]
    );
    // Wear adds up within a call: 59 - 10 - 1, then - 1 more.
    assert!(mods.command("wear 1"));
    let worn = Output::DamageHeld { amount: 1 };
    assert_eq!(
        mods.drain(),
        [worn.clone(), worn, feedback("Some(48) Some(47)")]
    );
    // The broken blocks come with the snapshot.
    mods.set_snapshot(Snapshot {
        broken: vec![BlockBroken {
            pos: BlockPos { x: 0, y: 65, z: 0 },
            block: FakeWorld.block(&BlockPos { x: 0, y: 64, z: 0 }).unwrap(),
            by_player: true,
            tool: Some("minecraft:iron_axe".into()),
            sneaking: false,
        }],
        ..player()
    });
    assert!(mods.command("broken"));
    assert_eq!(
        mods.drain(),
        [feedback("minecraft:oak_log at 0 65 0 by player true")]
    );
    // Taken back: nothing to read.
    mods.set_world(None);
    assert!(mods.command("block 0 64 0"));
    assert_eq!(mods.drain(), [feedback("unloaded")]);
}

#[test]
fn world_edits_need_world_edit() {
    let dir = mods_dir().join("read-only");
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = Package::read(&mods_dir().join("horizon_mods_test_guest.hmod"))
        .unwrap()
        .wasm
        .expect("the guest has code");
    let manifest = GUEST_MANIFEST.replace(", \"world-edit\"", "");
    std::fs::write(dir.join("horizon-mod.toml"), manifest).unwrap();
    std::fs::write(dir.join("mod.wasm"), wasm).unwrap();
    let mut mods = Mods::load(&[dir], |_| true, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Running);
    mods.set_snapshot(player());
    mods.set_world(Some(std::sync::Arc::new(FakeWorld)));
    assert!(mods.command("break 0 64 0"));
    assert!(mods.command("wear 1"));
    assert!(mods.command("block 0 64 0"));
    let log = r#"minecraft:oak_log [("axis", "y")] tags minecraft:logs"#;
    assert_eq!(
        mods.drain(),
        [
            feedback("false false"),
            feedback("None None"),
            feedback(log)
        ]
    );
}

#[test]
fn the_world_api_reads_the_chunk_surface_and_the_biomes() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_snapshot(player());
    mods.set_world(Some(std::sync::Arc::new(FakeWorld)));
    assert!(mods.command("surface 0 0"));
    assert_eq!(
        mods.drain(),
        [feedback(
            "surface 256 colors, 256 heights, 256 depths, 256 biomes"
        )]
    );
    assert!(mods.command("surface 5 5"));
    assert_eq!(mods.drain(), [feedback("no surface")]);
    assert!(mods.command("biomes"));
    assert_eq!(mods.drain(), [feedback("biomes 2")]);
    assert!(mods.command("colors"));
    assert_eq!(mods.drain(), [feedback("colors 2")]);
    // Without a world there is nothing to read.
    mods.set_world(None);
    assert!(mods.command("surface 0 0"));
    assert_eq!(mods.drain(), [feedback("no surface")]);
    assert!(mods.command("biomes"));
    assert_eq!(mods.drain(), [feedback("biomes 0")]);
}

#[test]
fn the_deprecated_0_1_4_storage_names_use_the_same_folder_and_the_old_files_move() {
    let dir = std::env::temp_dir().join(format!("horizon-mods-storage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // API 0.1.4 kept a mod's files in `<root>/<id>/`.
    let legacy = dir.join("legacy");
    std::fs::create_dir_all(legacy.join("test_guest")).unwrap();
    std::fs::write(legacy.join("test_guest").join("old"), "kept").unwrap();
    let path = dir.join("mods").join("guest.hmod");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::copy(mods_dir().join("horizon_mods_test_guest.hmod"), &path).unwrap();
    let mut mods = Mods::load(std::slice::from_ref(&path), |_| true, |_| false);
    mods.set_storage(Some(legacy.clone()));
    assert!(!legacy.join("test_guest").exists());
    assert!(mods.command("read old"));
    assert_eq!(mods.drain(), [feedback("read kept")]);
    assert!(mods.command("store notes hello"));
    assert_eq!(mods.drain(), [feedback("wrote notes")]);
    // The same file through upstream's names.
    assert!(mods.command("load_key notes"));
    assert_eq!(mods.drain(), [feedback("hello")]);
    assert!(mods.command("files"));
    assert_eq!(mods.drain(), [feedback("files notes,old")]);
    // A name with a path in it is refused.
    assert!(mods.command("store ../escape x"));
    let out = mods.drain();
    assert!(
        matches!(&out[0], Output::Feedback { error: true, text } if text.contains("is not [a-z0-9_.-]")),
        "{out:?}"
    );
    assert!(mods.command("rm notes"));
    assert_eq!(mods.drain(), [feedback("removed true")]);
    assert!(mods.command("rm notes"));
    assert_eq!(mods.drain(), [feedback("removed false")]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_keys_api_binds_and_polls() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_snapshot(player());
    assert!(mods.command("key"));
    assert_eq!(mods.drain(), [feedback("key key.keyboard.m events ")]);
    // The binding is the mod's, and the game may change it.
    let binds = mods.key_binds("test_guest");
    assert_eq!(binds.len(), 1);
    assert_eq!(binds[0].action, "map");
    assert_eq!(binds[0].key, "key.keyboard.m");
    assert!(mods.set_key("test_guest", "map", "key.keyboard.b"));
    assert_eq!(mods.key_binds("test_guest")[0].key, "key.keyboard.b");
    // A key event reaches the mod.
    mods.push_key_event(
        "test_guest",
        horizon_mods::KeyEvent {
            action: "map".into(),
            pressed: true,
            repeat: false,
        },
    );
    assert!(mods.command("key"));
    assert_eq!(
        mods.drain(),
        [feedback("key key.keyboard.m events map true false")]
    );
}

#[test]
fn the_screen_api_draws_and_takes_input() {
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_snapshot(player());
    mods.set_screen_size(320.0, 240.0);
    assert!(mods.command("screen"));
    assert_eq!(mods.drain(), [feedback("screen 1 sized true events ")]);
    // The screen is open, and the game can see it.
    let (id, name, screen) = mods.open_screen().expect("the screen is open");
    assert_eq!(id, "test_guest");
    assert_eq!(name, "Test Guest");
    assert_eq!(screen.shapes.len(), 3);
    assert_eq!(screen.textures.len(), 1);
    assert_eq!(screen.textures[0].width, 2);
    // The player's input reaches the mod.
    mods.push_screen_input(
        "test_guest",
        horizon_mods::ScreenEvent::MouseMove(horizon_mods::MouseMove { x: 3.0, y: 4.0 }),
    );
    assert!(mods.command("screen"));
    let out = mods.drain();
    assert!(
        matches!(&out[0], Output::Feedback { text, .. } if text.contains("events move")),
        "{out:?}"
    );
    // Escape closes it: the mod gets the `closed` event.
    mods.close_screen("test_guest");
    assert!(mods.open_screen().is_none());
    assert!(mods.command("screen"));
    let out = mods.drain();
    assert!(
        matches!(&out[0], Output::Feedback { text, .. } if text.contains("events closed")),
        "{out:?}"
    );
}

#[test]
fn the_browser_api_paints_a_panel_and_reads_items_recipes_and_gives() {
    use horizon_mods::{Area, GameScreen, Item, Modifiers, Point, Recipe, RecipeKind, Ui};
    use std::sync::Arc;
    let mut mods = load("horizon_mods_test_guest.hmod");
    let item = |id: &str| Item {
        id: id.into(),
        name: id.into(),
        max_stack: 64,
        tags: vec![],
    };
    mods.set_snapshot(Snapshot {
        items: Some(Arc::new(vec![
            item("minecraft:oak_planks"),
            item("minecraft:stick"),
        ])),
        recipes: Some(Arc::new(vec![Recipe {
            kind: RecipeKind::CraftingShapeless,
            station: "minecraft:crafting_table".into(),
            inputs: vec![vec!["minecraft:oak_log".into()], vec![]],
            output: Some("minecraft:oak_planks".into()),
            count: 4,
            width: 2,
            height: 1,
            time: 0,
            experience: 0.0,
        }])),
        ..player()
    });
    mods.set_ui(Ui {
        screen: GameScreen::Inventory,
        menu_origin: Some(Point { x: 40.0, y: 20.0 }),
        cursor: Point { x: 21.0, y: 22.0 },
        modifiers: Modifiers {
            shift: true,
            ctrl: false,
            alt: false,
        },
        slots: vec![([16.0, 16.0, 18.0, 18.0], "minecraft:stick".into())],
        text_widths: Some(Arc::new(vec![6.0; 512])),
        can_give: true,
        ..Ui::default()
    });
    assert!(mods.command("browser"));
    let out = mods.drain();
    assert_eq!(
        out,
        [feedback(
            "browser items 2 known true recipes 1 width 12 screen GameScreen::Inventory cursor 21,22 origin Some(40.0) shift true at Some(\"minecraft:stick\") events "
        )]
    );
    // The panel is the mod's, apart from its screen, and the game paints it.
    assert!(mods.open_screen().is_none());
    let (id, _, panel) = mods.overlays().next().expect("a panel is open");
    assert_eq!(id, "test_guest");
    assert_eq!(panel.elements.len(), 5);
    let overlay = panel.overlay.as_ref().expect("an area");
    let Area {
        x,
        y,
        width,
        height,
    } = overlay.area;
    assert_eq!((x, y, width, height), (200.0, 10.0, 100.0, 200.0));
    assert!(overlay.contains(250.0, 100.0) && !overlay.contains(20.0, 100.0));
    assert!(overlay.takes("key.keyboard.r") && !overlay.takes("key.keyboard.e"));
    // A click and a key reach the mod through the panel's input.
    mods.push_overlay_input(
        "test_guest",
        horizon_mods::ScreenEvent::MouseButton(horizon_mods::MousePress {
            button: 1,
            pressed: true,
            x: 210.0,
            y: 50.0,
        }),
    );
    assert!(mods.command("browser"));
    let out = mods.drain();
    assert!(
        matches!(&out[0], Output::Feedback { text, .. } if text.ends_with("events button 1")),
        "{out:?}"
    );
    // `give` needs the capability, a known item, and creative in this game's world.
    assert!(mods.command("giveitem minecraft:stick"));
    assert_eq!(
        mods.drain(),
        [
            Output::Give {
                id: "minecraft:stick".into(),
                count: 3
            },
            feedback("give true"),
        ]
    );
    assert!(mods.command("giveitem nope:nothing"));
    assert_eq!(mods.drain(), [feedback("give false")]);
    mods.set_ui(Ui::default());
    assert!(mods.command("giveitem minecraft:stick"));
    assert_eq!(mods.drain(), [feedback("give false")]);
}

#[test]
fn the_weather_particles_and_haze_apis() {
    use horizon_mods::{Landing, WeatherState};
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_world(Some(std::sync::Arc::new(FakeWorld)));
    mods.set_snapshot(Snapshot {
        weather: Some(WeatherState {
            rain: 0.5,
            thunder: 0.0,
        }),
        particles_setting: 1,
        ..player()
    });
    assert!(mods.command("weatherfx"));
    let out = mods.drain();
    assert_eq!(
        out,
        [feedback(
            "weather Some((0.5, 0.0)) setting 1 biome Some(\"minecraft:plains 7842607\") ground Some(65) solid true sprite 0 spawned true unknown false landed 0"
        )]
    );
    // The game takes the picture and the particle, and the haze is clamped.
    let (sprites, spawned) = mods.take_particles();
    assert_eq!(sprites.len(), 1);
    assert_eq!(sprites[0].name, "test_guest/drop");
    assert_eq!(spawned.len(), 1);
    assert!(mods.take_particles().1.is_empty());
    let hazes = mods.hazes();
    assert_eq!(hazes.len(), 1);
    assert_eq!(hazes[0].amount, 1.0);
    // What landed comes back to the mod that owns the picture only.
    mods.add_landings(&[
        Landing {
            sprite: 0,
            x: 0.5,
            y: 65.0,
            z: 0.5,
            water: false,
        },
        Landing {
            sprite: 9,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            water: true,
        },
    ]);
    assert!(mods.command("weatherfx"));
    let out = mods.drain();
    assert!(
        matches!(&out[0], Output::Feedback { text, .. } if text.ends_with("landed 1")),
        "{out:?}"
    );
}

#[test]
fn the_overlay_api_paints_layers_and_the_world_and_reads_the_hud() {
    use horizon_mods::{Bars, HudState, OverlayLayer, Point, ScreenElement, Ui};
    use std::sync::Arc;
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_world(Some(Arc::new(FakeWorld)));
    mods.set_snapshot(player());
    let icon = |name: &str| horizon_mods::BarIcon {
        x: 1.0,
        y: 2.0,
        width: 9.0,
        height: 9.0,
        name: name.into(),
    };
    mods.set_ui(Ui {
        hud: Some(HudState {
            screen_open: true,
            debug: false,
            hidden: false,
        }),
        bars: Some(Bars {
            food: vec![icon("hud/food_empty"); 10],
            health: vec![icon("hud/heart/container"); 10],
        }),
        seed: Some(42),
        fov: Some(70.0),
        size: Some(horizon_mods::ScreenSize {
            width: 320.0,
            height: 240.0,
        }),
        cursor: Point { x: 0.0, y: 0.0 },
        text_widths: Some(Arc::new(vec![6.0; 512])),
        ..Ui::default()
    });
    assert!(mods.command("overlayfx"));
    let out = mods.drain();
    assert_eq!(
        out,
        [feedback(
            "overlay true true size Some((320.0, 240.0)) hud Some((true, false, false)) bars Some((10, 10)) seed Some(42) fov Some(70.0) columns Some(256) width 12"
        )]
    );
    // The first layer was cleared by the mod, the second stays.
    assert!(mods.overlay_layer(OverlayLayer::UnderBars).is_empty());
    let over = mods.overlay_layer(OverlayLayer::OverBars);
    assert_eq!(over.len(), 1);
    assert_eq!(over[0].0, "test_guest");
    assert_eq!(over[0].1.len(), 2);
    assert!(matches!(over[0].1[0], ScreenElement::Picture(_)));
    assert_eq!(mods.overlay_world().len(), 1);
    let textures = mods.overlay_textures();
    assert_eq!(textures[0].1.len(), 1);
    assert_eq!(textures[0].1[0].width, 2);
}

#[test]
fn the_entity_api_lists_nearby_entities_and_queues_edits() {
    use horizon_mods::{EntityKind, Snapshot, WorldEntity};
    use std::sync::Arc;
    let mut mods = load("horizon_mods_test_guest.hmod");
    let entity = |id: u32, type_id: &str, x: f64, kind: EntityKind| WorldEntity {
        id,
        type_id: type_id.into(),
        kind,
        name: type_id.into(),
        uuid: String::new(),
        x,
        y: 64.0,
        z: -3.25,
        yaw: 0.0,
        height: 1.0,
        health: 20.0,
        max_health: 20.0,
        xp: 0,
        on_fire: false,
        hostile: id == 1,
        held_item: None,
        off_hand_item: None,
    };
    // The player is at (12.5, 64, -3.25).
    mods.set_snapshot(Snapshot {
        entities: Some(Arc::new(vec![
            entity(1, "minecraft:zombie", 13.5, EntityKind::Mob),
            entity(2, "minecraft:experience_orb", 14.5, EntityKind::Other),
            entity(3, "minecraft:cow", 200.0, EntityKind::Mob),
        ])),
        entities_editable: true,
        ..player()
    });
    assert!(mods.command("entityfx"));
    let out = mods.drain();
    assert_eq!(
        out,
        [
            Output::RemoveEntity { id: 1 },
            Output::SetXp { id: 2, xp: 9 },
            feedback(
                "entities 2 orbs 1 first Some((1, \"minecraft:zombie\", true)) removed true again false xp true wrong false face false"
            ),
        ]
    );
    // Another's world: nothing is edited.
    mods.set_snapshot(Snapshot {
        entities: Some(Arc::new(vec![entity(
            1,
            "minecraft:zombie",
            13.5,
            EntityKind::Mob,
        )])),
        entities_editable: false,
        ..player()
    });
    assert!(mods.command("entityfx"));
    let out = mods.drain();
    assert!(
        matches!(&out[0], Output::Feedback { text, .. } if text.contains("removed false")),
        "{out:?}"
    );
}

#[test]
fn the_look_apis_give_the_face_the_armour_and_the_container() {
    use horizon_mods::{Block, ContainerStack, Snapshot};
    let mut mods = load("horizon_mods_test_guest.hmod");
    mods.set_snapshot(Snapshot {
        block: Some(Block {
            x: 3,
            y: 64,
            z: 4,
            id: "minecraft:chest".into(),
            hardness: 2.5,
            requires_tool: false,
            tags: Vec::new(),
        }),
        block_face: Some("up".into()),
        entity_armor: Some(2.0),
        entity_absorption: Some(0.0),
        container: Some((
            [3, 64, 4],
            vec![
                Some(ContainerStack {
                    id: "minecraft:stone".into(),
                    count: 12,
                    damage: 0,
                }),
                None,
            ],
        )),
        ..player()
    });
    assert!(mods.command("lookfx"));
    assert_eq!(
        mods.drain(),
        [feedback(
            "look face Some(\"up\") armor Some(2.0) absorption Some(0.0) contents Some([\"minecraft:stonex12\"]) elsewhere false"
        )]
    );
}

#[test]
fn the_slots_api_reads_the_inventory_and_keeps_marks() {
    use horizon_mods::{Crafting, InventoryStack, PlayerInventory, Session, Snapshot, Ui};
    let stack = |id: &str, count: u32| InventoryStack {
        id: id.into(),
        count,
        damage: 0,
        components: String::new(),
    };
    let mut mods = load("horizon_mods_test_guest.hmod");
    let mut slots: Vec<Option<InventoryStack>> = vec![None; 36];
    slots[0] = Some(stack("minecraft:stone", 7));
    mods.set_snapshot(Snapshot {
        inventory: Some(PlayerInventory {
            slots,
            armour: vec![None; 4],
            offhand: None,
            carried: None,
        }),
        crafting: Some(Crafting {
            grid: vec![None; 4],
            output: None,
        }),
        session: Some(Session {
            id: 3,
            elapsed_ms: 1500,
        }),
        ..player()
    });
    mods.set_ui(Ui {
        hovered_slot: Some(9),
        ..Ui::default()
    });
    assert!(mods.command("slotfx"));
    assert_eq!(
        mods.drain(),
        [feedback(
            "slots Some((36, 4, false, Some((\"minecraft:stone\", 7)))) crafting Some((4, false)) session Some(3) hovered Some(9)"
        )]
    );
    // The marks stay for the game to draw.
    assert_eq!(mods.slot_marks().len(), 2);
    assert_eq!(mods.slot_marks()[0].slot, 0);
}

#[test]
fn the_food_hud_apis_read_the_player_keep_tooltips_and_debug_lines() {
    use horizon_mods::{Food, Health, Snapshot, TooltipHovered, Ui};
    use std::{collections::HashMap, sync::Arc};
    let mut mods = load("horizon_mods_test_guest.hmod");
    let mut foods = HashMap::new();
    foods.insert(
        "minecraft:cooked_beef".to_owned(),
        (
            Food {
                nutrition: 8,
                saturation: 12.8,
            },
            false,
        ),
    );
    foods.insert(
        "minecraft:rotten_flesh".to_owned(),
        (
            Food {
                nutrition: 4,
                saturation: 0.8,
            },
            true,
        ),
    );
    mods.set_snapshot(Snapshot {
        health: Some(Health {
            health: 17.0,
            max_health: 20.0,
            absorption: 2.0,
        }),
        effects: vec!["minecraft:regeneration".into()],
        difficulty: Some("hard".into()),
        foods: Some(Arc::new(foods)),
        ..player()
    });
    mods.set_ui(Ui {
        keys_down: vec!["key.keyboard.left.shift".into()],
        tooltip: Some(TooltipHovered {
            item: "minecraft:bread".into(),
            x: 1.0,
            y: 2.0,
            width: 30.0,
            height: 20.0,
        }),
        ..Ui::default()
    });
    assert!(mods.command("foodfx"));
    assert_eq!(
        mods.drain(),
        [feedback(
            "food health Some((17.0, 20.0, 2.0)) effects [\"minecraft:regeneration\"] difficulty Some(\"hard\") beef Some((8, 12.8)) rotten true offhand false shift true hovered minecraft:bread"
        )]
    );
    // The game draws what the mod added under that item's tooltip.
    let content = mods.tooltip_content("minecraft:cooked_beef");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0].0, "test_guest");
    assert_eq!(content[0].1.elements.len(), 1);
    assert!(mods.tooltip_content("minecraft:bread").is_empty());
    assert_eq!(mods.debug_lines(), ["hunger: 20"]);
}
