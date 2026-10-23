//! The `world` interface of Horizon mods (docs/mods.md, API 0.1.2): the
//! chunks around the player lent to mods for one hook call, the blocks
//! broken since the last tick, and the edits mods ask for (breaking blocks,
//! wearing the held item), made after the call in a world this game runs.
use super::mod_scan::{self, Palette};
use super::{Game, Resources};
use horizon_client::chunk::Chunk;
use horizon_mods::{BlockBroken, BlockPos, Capability, ChunkSurface, WorldAccess, WorldBlock};
use horizon_world::registry::{BlockRegistry, BlockStateId};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Chunks lent around the player's: 17 by 17 columns, Arc clones only.
const RADIUS: i32 = 8;
const AIR: [&str; 3] = ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"];

/// What `world` reads during a hook call.
pub(super) struct ModWorld {
    chunks: HashMap<(i32, i32), Arc<Chunk>>,
    res: Arc<Resources>,
    registry: &'static BlockRegistry,
    /// The world is this game's own: edits are allowed.
    local: bool,
    /// The main hand's `(damage, max_damage)`, outside creative.
    held: Option<(u32, u32)>,
    /// Each block's tags, found once per call.
    tags: Mutex<HashMap<String, Vec<String>>>,
    /// The map colours of the blocks, built once (the scan needs them).
    palette: Arc<Palette>,
    /// The air state's id (the scan skips it).
    air: u32,
    /// The world's biome names, in the order of their indexes.
    biomes: Arc<Vec<String>>,
}
impl ModWorld {
    fn state(&self, pos: &BlockPos) -> Option<BlockStateId> {
        let chunk = self.chunks.get(&(pos.x >> 4, pos.z >> 4))?;
        Some(chunk.block(pos.x & 15, pos.y, pos.z & 15))
    }
}
impl WorldAccess for ModWorld {
    fn block(&self, pos: &BlockPos) -> Option<WorldBlock> {
        let state = self.state(pos)?;
        let mut tags = self.tags.lock().unwrap_or_else(|e| e.into_inner());
        Some(world_block(
            &self.res,
            self.registry,
            *pos,
            state,
            &mut tags,
        ))
    }
    fn breakable(&self, pos: &BlockPos) -> bool {
        self.local
            && self.state(pos).is_some_and(|state| {
                let facts = self.res.facts().get(state);
                facts.hardness >= 0.0 && !AIR.contains(&&*facts.name)
            })
    }
    fn held_wear(&self) -> Option<(u32, u32)> {
        self.held.filter(|_| self.local)
    }
    fn chunk_surface(&self, cx: i32, cz: i32) -> Option<ChunkSurface> {
        let chunk = self.chunks.get(&(cx, cz))?;
        let columns = mod_scan::scan_chunk(chunk, &self.palette, self.air);
        // The biome of each column, at the height the scan found.
        let biome = (0..16)
            .flat_map(|z| (0..16).map(move |x| (x, z)))
            .map(|(x, z)| {
                let i = (z * 16 + x) as usize;
                chunk.biome(x, columns.height[i] as i32, z)
            })
            .collect();
        Some(ChunkSurface {
            color: columns.color.to_vec(),
            y: columns.height.iter().map(|h| i32::from(*h)).collect(),
            depth: columns.depth.to_vec(),
            biome,
        })
    }
    fn biome(&self, pos: &BlockPos) -> Option<horizon_mods::Biome> {
        let chunk = self.chunks.get(&(pos.x >> 4, pos.z >> 4))?;
        let info = self
            .res
            .biomes()
            .get(chunk.biome(pos.x & 15, pos.y, pos.z & 15));
        Some(horizon_mods::Biome {
            id: info.name.clone(),
            temperature: info.temperature,
            precipitation: info.precipitation,
            downfall: info.downfall,
            foliage: info.foliage,
            grass: info.grass,
            water: info.water,
        })
    }
    fn surface(&self, pos: &BlockPos, start_y: i32) -> Option<i32> {
        let chunk = self.chunks.get(&(pos.x >> 4, pos.z >> 4))?;
        let facts = self.res.facts();
        let (x, z) = (pos.x & 15, pos.z & 15);
        let top = start_y.min(horizon_client::chunk::MIN_Y + horizon_client::chunk::HEIGHT - 1);
        let mut y = top;
        while y >= horizon_client::chunk::MIN_Y {
            let f = facts.get(chunk.block(x, y, z));
            if f.full_collision || f.fluid != horizon_client::blocks::Fluid::None {
                return Some(y + 1);
            }
            y -= 1;
        }
        Some(horizon_client::chunk::MIN_Y)
    }
    fn solid(&self, pos: &BlockPos) -> bool {
        self.state(pos)
            .is_some_and(|s| self.res.facts().get(s).full_collision)
    }
    fn biome_names(&self) -> Vec<String> {
        self.biomes.as_ref().clone()
    }
    fn map_colors(&self) -> Vec<u32> {
        horizon_client::blocks::MAP_COLORS.to_vec()
    }
}

/// A block as `world` gives it: its id, properties and every block tag it
/// is in (the tags of each id found once per call through `cache`).
fn world_block(
    res: &Resources,
    registry: &BlockRegistry,
    pos: BlockPos,
    state: BlockStateId,
    cache: &mut HashMap<String, Vec<String>>,
) -> WorldBlock {
    let id = res.facts().get(state).name.to_string();
    let properties = registry
        .state(state)
        .map(|s| {
            s.properties
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();
    let tags = cache
        .entry(id.clone())
        .or_insert_with(|| {
            let mut tags: Vec<String> = res
                .items()
                .block_tags
                .iter()
                .filter(|(_, blocks)| blocks.contains(id.as_str()))
                .map(|(tag, _)| tag.clone())
                .collect();
            tags.sort();
            tags
        })
        .clone();
    WorldBlock {
        pos,
        id,
        properties,
        tags,
    }
}

impl Game<'_> {
    /// Lends the chunks around the player to the mods that read the world,
    /// for the hook calls until [`Game::take_mod_world`].
    pub(super) fn lend_mod_world(&mut self) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        if !self.mods.mods.uses(Capability::WorldRead) {
            return;
        }
        let (cx, cz) = ((p.position.x as i32) >> 4, (p.position.z as i32) >> 4);
        let mut chunks = HashMap::new();
        for x in cx - RADIUS..=cx + RADIUS {
            for z in cz - RADIUS..=cz + RADIUS {
                if let Some(chunk) = self.world.chunks.get(&(x, z)) {
                    chunks.insert((x, z), chunk.clone());
                }
            }
        }
        let held = if self.creative {
            None
        } else {
            self.held().and_then(|s| {
                let max = self.res.items().get(&s.item)?.max_damage;
                (max > 0).then_some((s.damage, max))
            })
        };
        // The map colours and the biome names, built once (API 0.1.4).
        let facts = self.res.facts();
        let palette = self
            .mods
            .palette
            .get_or_init(|| Arc::new(Palette::new(facts)))
            .clone();
        let biomes = self
            .mods
            .biomes
            .get_or_init(|| {
                Arc::new(
                    self.res
                        .biomes()
                        .list
                        .iter()
                        .map(|b| b.name.clone())
                        .collect(),
                )
            })
            .clone();
        let world = ModWorld {
            chunks,
            res: self.res.clone(),
            registry: self.pack.assets.registry,
            local: self.remote.is_none(),
            held,
            tags: Mutex::default(),
            palette,
            air: self.world.air.0,
            biomes,
        };
        self.mods.mods.set_world(Some(Arc::new(world)));
    }
    /// Takes the lent chunks back, so the game's next writes to them don't
    /// copy them.
    pub(super) fn take_mod_world(&mut self) {
        self.mods.mods.set_world(None);
    }
    /// Notes a block about to break or just broken (`state` as it was) for
    /// the next tick's `world.broken-blocks`.
    pub(super) fn mod_block_broken(&mut self, pos: [i32; 3], state: BlockStateId, by_player: bool) {
        if !self.mods.mods.uses(Capability::WorldRead) {
            return;
        }
        let [x, y, z] = pos;
        let at = BlockPos { x, y, z };
        let block = world_block(
            &self.res,
            self.pack.assets.registry,
            at,
            state,
            &mut HashMap::new(),
        );
        let tool = by_player
            .then(|| self.held().map(|s| s.item.to_string()))
            .flatten();
        let sneaking = by_player
            && self
                .player
                .as_ref()
                .is_some_and(|p| p.pose == horizon_client::pose::Pose::Crouching);
        self.mods.broken.push(BlockBroken {
            pos: at,
            block,
            by_player,
            tool,
            sneaking,
        });
    }
    /// `world.break-block` (`with_held` false) and `break-block-with-held`:
    /// `Level.destroyBlock`, its drops rolled without a tool or with the
    /// held item (which is not worn), in a world this game runs.
    pub(super) fn mod_break(&mut self, pos: [i32; 3], drops: bool, with_held: bool) {
        if self.remote.is_some() {
            return;
        }
        let [x, y, z] = pos;
        let state = self.world.block(x, y, z);
        let facts = self.res.facts().get(state).clone();
        if facts.hardness < 0.0 || AIR.contains(&&*facts.name) {
            return;
        }
        self.mod_block_broken(pos, state, false);
        let at = horizon_core::position::BlockPos::new(x, y, z);
        if with_held {
            let held = self.held().cloned();
            let harvest = !facts.requires_tool
                || self
                    .res
                    .items()
                    .correct_for_drops(held.as_ref(), &facts.name);
            self.with_level(|b, level, ticks| b.destroy_block(level, ticks, at, false));
            if harvest {
                for d in self.block_loot_at(pos, state, held.as_ref()) {
                    if &*d.item != "minecraft:air" {
                        self.pop_resource(pos, d);
                    }
                }
            }
        } else {
            self.with_level(|b, level, ticks| b.destroy_block(level, ticks, at, drops));
        }
        if let Err(e) = self.process_level_events() {
            log::warn!("Horizon: a mod's block break at {x} {y} {z}: {e}");
        }
    }
}
