//! YUNG's API's own processors, carried as data from the mods that write
//! them in Java (`horizon:yung_*`):
//! - `horizon:yung_replace`: a placeholder block becomes a draw from YUNG's
//!   block randomizer (Better Jungle Temples' block replace, Better Dungeons'
//!   walls and legs, Better Witch Huts' mushrooms);
//! - `horizon:yung_brew`: a brewing stand gets a potion and its ingredient
//!   (Better Witch Huts);
//! - `horizon:yung_dispenser`: a dispenser gets items (Better Jungle Temples);
//! - `horizon:yung_banner`: a wall banner gets a pattern and a colour
//!   (Better Strongholds, Better Dungeons);
//! - `horizon:yung_torch`: torches are removed with a chance (Better Jungle
//!   Temples).
//!
//! The randomizer is YUNG's `BlockStateRandomizer`: one `nextFloat` and the
//! entries' shares of `[0, 1)`, the default when the draw misses them all.
use super::{PlaceSettings, ProcessResult, ProcessorRandom, json_to_nbt};
use crate::{
    nbt::{Compound, Tag},
    registry::BlockStateId,
    worldgen::{
        DecodeError,
        placement::PlacementContext,
        structure::{BlockInfo, StructureResources},
    },
};
use serde_json::Value;
use std::collections::BTreeSet;

/// YUNG's `BlockStateRandomizer`: the entries' shares of `[0, 1)` and the
/// default, drawn with one `nextFloat`.
#[derive(Clone, Debug)]
pub(super) struct Randomizer {
    entries: Vec<(BlockStateId, f32)>,
    default: BlockStateId,
}
impl Randomizer {
    pub(super) fn decode(
        v: &Value,
        state: &impl Fn(&Value) -> Result<BlockStateId, DecodeError>,
    ) -> Result<Self, DecodeError> {
        let entries = v["entries"]
            .as_array()
            .ok_or(DecodeError::Field("randomizer entries"))?
            .iter()
            .map(|e| {
                let p = e["probability"]
                    .as_f64()
                    .ok_or(DecodeError::Field("randomizer probability"))?;
                Ok((state(&e["blockState"])?, p as f32))
            })
            .collect::<Result<_, DecodeError>>()?;
        Ok(Self {
            entries,
            default: state(&v["defaultBlockState"])?,
        })
    }
    /// `get(random)`: the first entry whose share holds the draw, else the
    /// default.
    pub(super) fn get(&self, random: &mut dyn ProcessorRandom) -> BlockStateId {
        let target = random.float();
        let mut bottom = 0f32;
        for &(state, p) in &self.entries {
            if bottom <= target && target < bottom + p {
                return state;
            }
            bottom += p;
        }
        self.default
    }
}

/// The blocks a YUNG processor applies to, by name.
fn blocks(value: &Value, key: &'static str) -> Result<BTreeSet<String>, DecodeError> {
    value[key]
        .as_array()
        .ok_or(DecodeError::Field(key))?
        .iter()
        .map(|b| {
            b.as_str()
                .map(|b| crate::worldgen::tags::identifier(b).map(|i| i.to_string()))
                .ok_or(DecodeError::Field(key))
                .and_then(|id| id)
        })
        .collect()
}

/// `horizon:yung_replace`: a block of `blocks` becomes a draw from `output`.
/// The input's properties are copied to the output when the processor says
/// so (`Block.withPropertiesOf`), and the ones it names (`facing`, `half`,
/// `waterlogged`) are copied on their own.
#[derive(Clone, Debug)]
pub struct Replace {
    blocks: BTreeSet<String>,
    output: Randomizer,
    copy_input_properties: bool,
    copy_facing: bool,
    copy_half: bool,
    copy_waterlogged: bool,
    /// The block entity the output gets (a banner's patterns, a spawner's
    /// data); the block's own stays when missing.
    nbt: Option<Compound>,
}
impl Replace {
    pub(super) fn decode(
        v: &Value,
        state: &impl Fn(&Value) -> Result<BlockStateId, DecodeError>,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            blocks: blocks(v, "blocks")?,
            output: Randomizer::decode(&v["output"], state)?,
            copy_input_properties: v["copy_input_properties"].as_bool().unwrap_or(false),
            copy_facing: v["copy_facing"].as_bool().unwrap_or(false),
            copy_half: v["copy_half"].as_bool().unwrap_or(false),
            copy_waterlogged: v["copy_waterlogged"].as_bool().unwrap_or(false),
            nbt: match v.get("nbt").map(json_to_nbt) {
                Some(Tag::Compound(c)) => Some(c),
                Some(_) => return Err(DecodeError::Field("yung_replace nbt")),
                None => None,
            },
        })
    }
    /// The output with the input's properties, as the processor asks.
    fn output_state(
        &self,
        registry: &crate::registry::BlockRegistry,
        input: BlockStateId,
        output: BlockStateId,
    ) -> BlockStateId {
        let mut state = output;
        if self.copy_input_properties
            && let Some(kept) = registry
                .state(output)
                .and_then(|s| super::with_properties_of(registry, &s.name, input))
        {
            state = kept;
        }
        for (flag, key) in [
            (self.copy_facing, "facing"),
            (self.copy_half, "half"),
            (self.copy_waterlogged, "waterlogged"),
        ] {
            if !flag {
                continue;
            }
            let Some(value) = registry
                .state(input)
                .and_then(|s| s.properties.get(key).cloned())
            else {
                continue;
            };
            let Some(current) = registry.state(state).cloned() else {
                continue;
            };
            if !current.properties.contains_key(key) {
                continue;
            }
            let mut changed = current;
            changed.properties.insert(key.to_owned(), value);
            if let Some(id) = registry.id(&changed) {
                state = id;
            }
        }
        state
    }
    pub(super) fn process(
        &self,
        world: &mut impl PlacementContext,
        resources: &StructureResources,
        info: BlockInfo,
        settings: &mut PlaceSettings<'_>,
    ) -> ProcessResult {
        let registry = resources.registry;
        let name = registry
            .state(info.state)
            .map(|s| s.name.as_str())
            .unwrap_or("");
        if !self.blocks.contains(name) {
            return Ok(Some(info));
        }
        let pos = info.pos;
        let state = settings.with_random(pos, |random| {
            let drawn = self.output.get(random);
            self.output_state(registry, info.state, drawn)
        });
        let nbt = self.nbt.clone().or(info.nbt);
        let _ = world;
        Ok(Some(BlockInfo { pos, state, nbt }))
    }
}

/// `horizon:yung_brew`: a brewing stand gets a potion and the ingredient it
/// is brewed from, in its `Items` list.
#[derive(Clone, Debug)]
pub struct Brew {
    /// (ingredient, potion) pairs; one is drawn.
    recipes: Vec<(String, String)>,
}
impl Brew {
    pub(super) fn decode(v: &Value) -> Result<Self, DecodeError> {
        let recipes = v["recipes"]
            .as_array()
            .ok_or(DecodeError::Field("brew recipes"))?
            .iter()
            .map(|r| {
                let text = |k: &'static str| {
                    r[k].as_str()
                        .map(|s| crate::worldgen::tags::identifier(s).map(|i| i.to_string()))
                        .ok_or(DecodeError::Field(k))
                        .and_then(|id| id)
                };
                Ok((text("ingredient")?, text("potion")?))
            })
            .collect::<Result<_, DecodeError>>()?;
        Ok(Self { recipes })
    }
    pub(super) fn process(
        &self,
        world: &mut impl PlacementContext,
        resources: &StructureResources,
        info: BlockInfo,
        settings: &mut PlaceSettings<'_>,
    ) -> ProcessResult {
        let registry = resources.registry;
        let name = registry
            .state(info.state)
            .map(|s| s.name.as_str())
            .unwrap_or("");
        if name != "minecraft:brewing_stand" || self.recipes.is_empty() {
            return Ok(Some(info));
        }
        let pos = info.pos;
        let (ingredient, potion) = settings.with_random(pos, |random| {
            let i = random.int(self.recipes.len() as i32) as usize;
            self.recipes[i].clone()
        });
        let slot = settings.with_random(pos, |random| random.int(3));
        let items = vec![
            item_stack(&ingredient, slot as i8, 1, None),
            item_stack("minecraft:potion", 0, 1, Some(potion)),
        ];
        let mut nbt = info.nbt.clone().unwrap_or_default();
        nbt.insert("Items".into(), Tag::List(items));
        let _ = world;
        Ok(Some(BlockInfo {
            pos,
            state: info.state,
            nbt: Some(nbt),
        }))
    }
}

/// An item stack in 26.3's shape: the id, the count, the slot and the
/// components (a potion's contents).
fn item_stack(id: &str, slot: i8, count: i32, potion: Option<String>) -> Tag {
    let mut stack = Compound::new();
    stack.insert("Slot".into(), Tag::Byte(slot));
    stack.insert("id".into(), Tag::String(id.into()));
    stack.insert("count".into(), Tag::Int(count));
    if let Some(potion) = potion {
        let mut contents = Compound::new();
        contents.insert("potion".into(), Tag::String(potion.as_str().into()));
        let mut components = Compound::new();
        components.insert("minecraft:potion_contents".into(), Tag::Compound(contents));
        stack.insert("components".into(), Tag::Compound(components));
    }
    Tag::Compound(stack)
}

/// `horizon:yung_dispenser`: a dispenser gets the items it holds.
#[derive(Clone, Debug)]
pub struct Dispenser {
    items: Vec<(String, i32)>,
}
impl Dispenser {
    pub(super) fn decode(v: &Value) -> Result<Self, DecodeError> {
        let items = v["items"]
            .as_array()
            .ok_or(DecodeError::Field("dispenser items"))?
            .iter()
            .map(|i| {
                let id = i["id"]
                    .as_str()
                    .map(|s| crate::worldgen::tags::identifier(s).map(|i| i.to_string()))
                    .ok_or(DecodeError::Field("dispenser item"))?
                    .map_err(|_| DecodeError::Field("dispenser item"))?;
                let count = i["count"].as_i64().unwrap_or(1) as i32;
                Ok((id, count))
            })
            .collect::<Result<_, DecodeError>>()?;
        Ok(Self { items })
    }
    pub(super) fn process(
        &self,
        world: &mut impl PlacementContext,
        resources: &StructureResources,
        info: BlockInfo,
        settings: &mut PlaceSettings<'_>,
    ) -> ProcessResult {
        let registry = resources.registry;
        let name = registry
            .state(info.state)
            .map(|s| s.name.as_str())
            .unwrap_or("");
        if name != "minecraft:dispenser" {
            return Ok(Some(info));
        }
        let pos = info.pos;
        let mut nbt = info.nbt.clone().unwrap_or_default();
        let items: Vec<Tag> = self
            .items
            .iter()
            .enumerate()
            .map(|(slot, (id, count))| {
                let mut stack = Compound::new();
                stack.insert("Slot".into(), Tag::Int(slot as i32));
                stack.insert("id".into(), Tag::String(id.as_str().into()));
                stack.insert("count".into(), Tag::Int(*count));
                Tag::Compound(stack)
            })
            .collect();
        nbt.insert("Items".into(), Tag::List(items));
        let _ = world;
        let _ = settings;
        Ok(Some(BlockInfo {
            pos,
            state: info.state,
            nbt: Some(nbt),
        }))
    }
}

/// `horizon:yung_banner`: a wall banner gets its patterns and their colours.
#[derive(Clone, Debug)]
pub struct Banner {
    blocks: BTreeSet<String>,
    /// (banner block, patterns: (pattern, colour)).
    banners: Vec<(String, Vec<(String, String)>)>,
}
impl Banner {
    pub(super) fn decode(v: &Value) -> Result<Self, DecodeError> {
        let text = |value: &Value, key: &'static str| {
            value[key]
                .as_str()
                .map(|s| crate::worldgen::tags::identifier(s).map(|i| i.to_string()))
                .ok_or(DecodeError::Field(key))
                .and_then(|id| id)
        };
        // The colour is a dye colour's name, not an identifier.
        let colour = |value: &Value| {
            value["color"]
                .as_str()
                .map(|s| s.to_owned())
                .ok_or(DecodeError::Field("banner color"))
        };
        let banners = v["banners"]
            .as_array()
            .ok_or(DecodeError::Field("banners"))?
            .iter()
            .map(|b| {
                let patterns = b["patterns"]
                    .as_array()
                    .ok_or(DecodeError::Field("banner patterns"))?
                    .iter()
                    .map(|p| Ok((text(p, "pattern")?, colour(p)?)))
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                Ok((text(b, "block")?, patterns))
            })
            .collect::<Result<_, DecodeError>>()?;
        Ok(Self {
            blocks: blocks(v, "blocks")?,
            banners,
        })
    }
    pub(super) fn process(
        &self,
        world: &mut impl PlacementContext,
        resources: &StructureResources,
        info: BlockInfo,
        settings: &mut PlaceSettings<'_>,
    ) -> ProcessResult {
        let registry = resources.registry;
        let name = registry
            .state(info.state)
            .map(|s| s.name.as_str())
            .unwrap_or("");
        if !self.blocks.contains(name) || self.banners.is_empty() {
            return Ok(Some(info));
        }
        let pos = info.pos;
        let index = settings.with_random(pos, |random| random.int(self.banners.len() as i32));
        let (block, patterns) = &self.banners[index as usize];
        let Some(state) = registry.default_state(block) else {
            return Ok(Some(info));
        };
        // The facing of the banner the template had stays.
        let state = registry
            .state(state)
            .and_then(|s| super::with_properties_of(registry, &s.name, info.state))
            .unwrap_or(state);
        let mut nbt = info.nbt.clone().unwrap_or_default();
        nbt.insert("id".into(), Tag::String("minecraft:banner".into()));
        let list: Vec<Tag> = patterns
            .iter()
            .map(|(pattern, color)| {
                let mut entry = Compound::new();
                entry.insert("pattern".into(), Tag::String(pattern.as_str().into()));
                entry.insert("color".into(), Tag::String(color.as_str().into()));
                Tag::Compound(entry)
            })
            .collect();
        nbt.insert("Patterns".into(), Tag::List(list));
        let _ = world;
        Ok(Some(BlockInfo {
            pos,
            state,
            nbt: Some(nbt),
        }))
    }
}

/// `horizon:yung_carve`: the block is taken away where the world is empty
/// (Better Dungeons' walls and rubble: nothing is built in a hole).
#[derive(Clone, Debug)]
pub struct Carve {
    blocks: BTreeSet<String>,
    replacement: BlockStateId,
}
impl Carve {
    pub(super) fn decode(
        v: &Value,
        registry: &crate::registry::BlockRegistry,
    ) -> Result<Self, DecodeError> {
        let replacement = registry
            .default_state("minecraft:cave_air")
            .ok_or_else(|| DecodeError::Registry("unknown block minecraft:cave_air".into()))?;
        Ok(Self {
            blocks: blocks(v, "blocks")?,
            replacement,
        })
    }
    pub(super) fn process(
        &self,
        world: &mut impl PlacementContext,
        resources: &StructureResources,
        info: BlockInfo,
        settings: &mut PlaceSettings<'_>,
    ) -> ProcessResult {
        let registry = resources.registry;
        let name = registry
            .state(info.state)
            .map(|s| s.name.as_str())
            .unwrap_or("");
        if !self.blocks.contains(name) {
            return Ok(Some(info));
        }
        // The world there decides: where it is empty, the block is not built.
        let there = world.block_state(info.pos)?;
        if !empty(registry, there) {
            return Ok(Some(info));
        }
        let _ = settings;
        Ok(Some(BlockInfo {
            pos: info.pos,
            state: self.replacement,
            nbt: None,
        }))
    }
}

/// `horizon:yung_leg`: the leg under a dungeon room. The placeholder becomes
/// a block and a column of blocks runs down from it, until the world is
/// empty there (the plain air ends the leg; a fluid does not).
#[derive(Clone, Debug)]
pub struct Leg {
    blocks: BTreeSet<String>,
    /// What the placeholder becomes.
    output: BlockStateId,
    /// The world's own block is kept where the world is empty or holds a
    /// fluid (the zombie leg's floor).
    world_when_empty: bool,
    pillar: Randomizer,
}
impl Leg {
    pub(super) fn decode(
        v: &Value,
        state: &impl Fn(&Value) -> Result<BlockStateId, DecodeError>,
    ) -> Result<Self, DecodeError> {
        let output = state(&v["output"])?;
        Ok(Self {
            blocks: blocks(v, "blocks")?,
            output,
            world_when_empty: v["world_when_empty"].as_bool().unwrap_or(false),
            pillar: Randomizer::decode(&v["pillar_states"], state)?,
        })
    }
    pub(super) fn process(
        &self,
        world: &mut impl PlacementContext,
        resources: &StructureResources,
        info: BlockInfo,
        settings: &mut PlaceSettings<'_>,
    ) -> ProcessResult {
        let registry = resources.registry;
        let name = registry
            .state(info.state)
            .map(|s| s.name.as_str())
            .unwrap_or("");
        if !self.blocks.contains(name) {
            return Ok(Some(info));
        }
        let there = world.block_state(info.pos)?;
        let state = match self.world_when_empty {
            true if empty(registry, there) || fluid(registry, there) => there,
            _ => self.output,
        };
        let pos = info.pos;
        let (min, max) = (world.min_y()?, world.max_y()?);
        let mut at = [pos[0], pos[1] - 1, pos[2]];
        settings.with_random(pos, |random| {
            while at[1] > min && at[1] < max {
                let block = world.block_state(at)?;
                // Plain air ends the column; a fluid does not.
                if empty(registry, block) && !fluid(registry, block) {
                    break;
                }
                world.set_chunk_block(at, self.pillar.get(random))?;
                at[1] -= 1;
            }
            Ok::<(), crate::worldgen::placement::PlacementQueryError>(())
        })?;
        Ok(Some(BlockInfo {
            pos,
            state,
            nbt: None,
        }))
    }
}

/// Whether the state is air (plain or cave air).
fn empty(registry: &crate::registry::BlockRegistry, state: BlockStateId) -> bool {
    registry
        .state(state)
        .map(|s| matches!(s.name.as_str(), "minecraft:air" | "minecraft:cave_air"))
        .unwrap_or(false)
}

/// Whether the state holds a fluid: a fluid block or a waterlogged one.
fn fluid(registry: &crate::registry::BlockRegistry, state: BlockStateId) -> bool {
    let Some(block) = registry.state(state) else {
        return false;
    };
    matches!(block.name.as_str(), "minecraft:water" | "minecraft:lava")
        || block
            .properties
            .get("waterlogged")
            .is_some_and(|v| v == "true")
}

/// `horizon:yung_torch`: torches of `blocks` are removed with a chance.
#[derive(Clone, Debug)]
pub struct Torch {
    blocks: BTreeSet<String>,
    chance: f32,
    air: BlockStateId,
}
impl Torch {
    pub(super) fn decode(v: &Value, resources: &StructureResources) -> Result<Self, DecodeError> {
        let air = resources
            .registry
            .default_state("minecraft:air")
            .ok_or_else(|| DecodeError::Registry("unknown block minecraft:air".into()))?;
        Ok(Self {
            blocks: blocks(v, "blocks")?,
            chance: v["chance"].as_f64().unwrap_or(0.) as f32,
            air,
        })
    }
    pub(super) fn process(
        &self,
        world: &mut impl PlacementContext,
        resources: &StructureResources,
        info: BlockInfo,
        settings: &mut PlaceSettings<'_>,
    ) -> ProcessResult {
        let registry = resources.registry;
        let name = registry
            .state(info.state)
            .map(|s| s.name.as_str())
            .unwrap_or("");
        if !self.blocks.contains(name) {
            return Ok(Some(info));
        }
        let pos = info.pos;
        let remove = settings.with_random(pos, |random| random.float() < self.chance);
        let _ = world;
        Ok(Some(match remove {
            true => BlockInfo {
                pos,
                state: self.air,
                nbt: None,
            },
            false => info,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbt::JavaString;

    /// A random that gives the values the test names, in order.
    struct Fixed {
        values: Vec<f32>,
        at: usize,
    }
    impl Fixed {
        fn new(values: Vec<f32>) -> Self {
            Self { values, at: 0 }
        }
    }
    impl ProcessorRandom for Fixed {
        fn float(&mut self) -> f32 {
            let value = self.values.get(self.at).copied().unwrap_or(0.);
            self.at += 1;
            value
        }
        fn int(&mut self, bound: i32) -> i32 {
            let value = self.values.get(self.at).copied().unwrap_or(0.);
            self.at += 1;
            ((value * bound as f32) as i32).min(bound - 1)
        }
        fn long(&mut self) -> i64 {
            0
        }
    }

    #[test]
    fn the_randomizer_draws_by_share() {
        let randomizer = Randomizer::decode(
            &serde_json::json!({
                "defaultBlockState": {"id": "minecraft:stone"},
                "entries": [
                    {"blockState": {"id": "minecraft:coal_ore"}, "probability": 0.2},
                    {"blockState": {"id": "minecraft:iron_ore"}, "probability": 0.3}]}),
            &|v: &Value| {
                let id = v["id"].as_str().unwrap_or_default();
                Ok(BlockStateId(match id {
                    "minecraft:coal_ore" => 1,
                    "minecraft:iron_ore" => 2,
                    _ => 3,
                }))
            },
        )
        .unwrap();
        // One nextFloat and the entries' shares of [0, 1).
        for (roll, want) in [
            (0.0, 1),
            (0.19, 1),
            (0.2, 2),
            (0.49, 2),
            (0.5, 3),
            (0.99, 3),
        ] {
            let mut random = Fixed::new(vec![roll]);
            assert_eq!(randomizer.get(&mut random), BlockStateId(want), "{roll}");
        }
    }

    #[test]
    fn the_replace_processor_reads_its_blocks_and_flags() {
        let replace = Replace::decode(
            &serde_json::json!({
                "blocks": ["minecraft:gray_stained_glass"],
                "output": {"defaultBlockState": {"id": "minecraft:stone"},
                    "entries": [{"blockState": {"id": "minecraft:oak_log"}, "probability": 1.0}]},
                "copy_facing": true}),
            &|_v: &Value| Ok(BlockStateId(1)),
        )
        .unwrap();
        assert!(replace.blocks.contains("minecraft:gray_stained_glass"));
        assert!(replace.copy_facing);
        assert!(!replace.copy_input_properties);
    }

    #[test]
    fn the_brew_processor_holds_a_potion_and_its_ingredient() {
        let brew = Brew::decode(&serde_json::json!({"recipes": [
            {"ingredient": "minecraft:sugar", "potion": "minecraft:swiftness"}]}))
        .unwrap();
        assert_eq!(brew.recipes.len(), 1);
        assert_eq!(brew.recipes[0].0, "minecraft:sugar");
        assert_eq!(brew.recipes[0].1, "minecraft:swiftness");
    }

    #[test]
    fn the_dispenser_processor_holds_its_items() {
        let dispenser = Dispenser::decode(&serde_json::json!({"items": [
            {"id": "minecraft:fire_charge", "count": 1}]}))
        .unwrap();
        assert_eq!(dispenser.items, [("minecraft:fire_charge".to_owned(), 1)]);
    }

    #[test]
    fn the_banner_processor_reads_its_banners() {
        let banner = Banner::decode(&serde_json::json!({
            "blocks": ["minecraft:gray_wall_banner"],
            "banners": [{"block": "minecraft:magenta_wall_banner",
                "patterns": [{"pattern": "minecraft:stripe_small", "color": "white"},
                    {"pattern": "minecraft:border", "color": "black"}]}]}))
        .unwrap();
        assert!(banner.blocks.contains("minecraft:gray_wall_banner"));
        assert_eq!(banner.banners.len(), 1);
        assert_eq!(banner.banners[0].0, "minecraft:magenta_wall_banner");
        assert_eq!(banner.banners[0].1.len(), 2);
        assert_eq!(banner.banners[0].1[0].1, "white");
        assert_eq!(banner.banners[0].1[1].1, "black");
    }

    /// A registry with air, cave air, cobblestone and stone.
    fn registry() -> crate::registry::BlockRegistry {
        let facts = serde_json::json!({"defaults": {
                "minecraft:air": 0, "minecraft:cave_air": 1,
                "minecraft:cobblestone": 2, "minecraft:stone": 3},
            "states": [
                ["minecraft:air", false, false, "", "minecraft:air"],
                ["minecraft:cave_air", false, false, "", "minecraft:cave_air"],
                ["minecraft:cobblestone", false, false, "", "minecraft:cobblestone"],
                ["minecraft:stone", false, false, "", "minecraft:stone"]]});
        crate::registry::BlockRegistry::from_facts_json(facts.to_string().as_bytes()).unwrap()
    }

    #[test]
    fn the_carve_processor_reads_its_blocks() {
        let registry = registry();
        let carve = Carve::decode(
            &serde_json::json!({"blocks": ["minecraft:cobblestone"]}),
            &registry,
        )
        .unwrap();
        assert!(carve.blocks.contains("minecraft:cobblestone"));
        assert_eq!(carve.replacement, BlockStateId(1));
    }

    #[test]
    fn the_leg_processor_reads_its_blocks_and_column() {
        let leg = Leg::decode(
            &serde_json::json!({
                "blocks": ["minecraft:yellow_stained_glass"],
                "output": {"id": "minecraft:mossy_cobblestone"},
                "pillar_states": {"defaultBlockState": {"id": "minecraft:stone_bricks"},
                    "entries": []}}),
            &|_v: &Value| Ok(BlockStateId(1)),
        )
        .unwrap();
        assert!(leg.blocks.contains("minecraft:yellow_stained_glass"));
        assert!(!leg.world_when_empty);
        assert_eq!(leg.output, BlockStateId(1));
    }

    #[test]
    fn the_torch_processor_removes_torches_with_its_chance() {
        let torch = Torch {
            blocks: ["minecraft:torch".to_owned()].into_iter().collect(),
            chance: 0.9,
            air: BlockStateId(0),
        };
        assert!(torch.blocks.contains("minecraft:torch"));
        assert_eq!(torch.chance, 0.9);
    }

    #[test]
    fn the_item_stack_is_in_the_shape_of_26_3() {
        let stack = item_stack("minecraft:potion", 1, 1, Some("minecraft:swiftness".into()));
        let Tag::Compound(compound) = &stack else {
            panic!("not a compound");
        };
        assert_eq!(
            compound.get(&JavaString::from("id")),
            Some(&Tag::String("minecraft:potion".into()))
        );
        assert_eq!(compound.get(&JavaString::from("count")), Some(&Tag::Int(1)));
        assert!(compound.get(&JavaString::from("components")).is_some());
    }
}
