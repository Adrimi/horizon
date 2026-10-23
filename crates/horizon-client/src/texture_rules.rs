//! The texture rules mods give the mesher (`texture-rules`, API 0.1.14): a
//! mod hands over pictures and rules, and the mesher picks a tile for each
//! face it draws from the blocks around it, so no mod code runs per face.
//! The rules and pictures live in [`DEFINITIONS`] so they outlive a reload of
//! the resource packs, which builds the atlas again.
use crate::sprites::Sprites;
use horizon_core::geometry::Direction;
use horizon_mods::{RuleConnect, RuleMethod, TextureSet};
use horizon_world::registry::{BlockRegistry, BlockStateId};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// The world directions a face's texture runs along: right (the texture's
/// `u` grows) and up (its `v` shrinks), as vanilla's cube faces map their
/// UVs.
fn axes(face: Direction) -> ([i32; 3], [i32; 3]) {
    match face {
        Direction::Up => ([1, 0, 0], [0, 0, -1]),
        Direction::Down => ([1, 0, 0], [0, 0, 1]),
        Direction::North => ([-1, 0, 0], [0, 1, 0]),
        Direction::South => ([1, 0, 0], [0, 1, 0]),
        Direction::West => ([0, 0, 1], [0, 1, 0]),
        Direction::East => ([0, 0, -1], [0, 1, 0]),
    }
}

/// What a face draws instead of its sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// The face keeps its own sprite.
    Keep,
    /// The face shows this layer of the atlas.
    Layer(u16),
    /// The face is four quarters, each showing its own layer (top-left,
    /// top-right, bottom-left, bottom-right).
    Quarters([u16; 4]),
}

/// What the running mods gave: set by the game, read when the atlas is built.
pub static DEFINITIONS: RwLock<Option<Arc<TextureSet>>> = RwLock::new(None);

pub fn set_definitions(set: TextureSet) {
    let mut guard = DEFINITIONS.write().unwrap_or_else(|e| e.into_inner());
    *guard = (!set.rules.is_empty() || !set.sprites.is_empty()).then(|| Arc::new(set));
}
fn definitions() -> Option<Arc<TextureSet>> {
    DEFINITIONS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

const NONE: u32 = u32::MAX;

/// A rule's face mask bit for `face` (north 1, east 2, south 4, west 8, up
/// 16, down 32).
fn face_bit(face: Direction) -> u32 {
    match face {
        Direction::North => 1,
        Direction::East => 2,
        Direction::South => 4,
        Direction::West => 8,
        Direction::Up => 16,
        Direction::Down => 32,
    }
}

struct Rule {
    method: RuleMethod,
    connect: RuleConnect,
    faces: u32,
    /// Each tile's layer; none for a picture the atlas does not have.
    tiles: Vec<Option<u16>>,
    pattern: Vec<u32>,
    quarters: Vec<u32>,
    grid: Vec<u32>,
    width: u32,
    height: u32,
    weights: Vec<u32>,
}

/// The mods' rules, indexed by block state and by picture.
#[derive(Default)]
pub struct ModRules {
    rules: Vec<Rule>,
    by_state: HashMap<u32, Vec<u16>>,
    by_layer: HashMap<u32, Vec<u16>>,
    /// Each state's block, for `connect = block`.
    block: Vec<u32>,
    /// Each state's faces as (facing, layer), for `connect = tile`.
    faces: Vec<Vec<(u8, u16)>>,
}

fn hash(pos: [i32; 3], face: Direction) -> u32 {
    let mut h = 0x811c_9dc5u32;
    for v in [pos[0], pos[1], pos[2], face as i32] {
        h ^= v as u32;
        h = h.wrapping_mul(0x0100_0193);
        h ^= h >> 15;
    }
    h
}

impl ModRules {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Adds the mods' pictures to `sprites` and indexes their rules;
    /// `state_faces` gives the (facing, layer) of every face a state draws.
    pub fn load(
        registry: &BlockRegistry,
        sprites: &mut Sprites,
        state_faces: impl Fn(BlockStateId) -> Vec<(u8, u32)>,
    ) -> Self {
        let mut out = Self::default();
        let Some(defs) = definitions() else {
            return out;
        };
        for (mod_id, name, png) in &defs.sprites {
            if let Err(e) = sprites.insert_bytes(&format!("{mod_id}:{name}"), png) {
                log::warn!("Horizon: mod {mod_id} picture {name}: {e}");
            }
        }
        if defs.rules.is_empty() {
            return out;
        }
        let mut blocks = HashMap::new();
        out.block = (0..registry.len() as u32)
            .map(|id| {
                let name = registry
                    .state(BlockStateId(id))
                    .map_or("", |s| s.name.as_str());
                let next = blocks.len() as u32;
                *blocks.entry(name.to_owned()).or_insert(next)
            })
            .collect();
        let needs_faces = defs
            .rules
            .iter()
            .any(|(_, r)| matches!(r.connect, RuleConnect::Tile) || !r.sprites.is_empty());
        if needs_faces {
            out.faces = (0..registry.len() as u32)
                .map(|id| {
                    state_faces(BlockStateId(id))
                        .into_iter()
                        .map(|(facing, layer)| (facing, layer as u16))
                        .collect()
                })
                .collect();
        }
        for (mod_id, rule) in &defs.rules {
            let index = out.rules.len() as u16;
            let tiles: Vec<Option<u16>> = rule
                .tiles
                .iter()
                .map(|name| {
                    let key = if name.contains(':') {
                        name.clone()
                    } else {
                        format!("{mod_id}:{name}")
                    };
                    let layer = sprites.layers.get(&key).map(|l| *l as u16);
                    if layer.is_none() {
                        log::warn!("Horizon: mod {mod_id} rule: no picture {key}");
                    }
                    layer
                })
                .collect();
            for sprite in &rule.sprites {
                if let Some(layer) = sprites.layers.get(sprite) {
                    out.by_layer.entry(*layer).or_default().push(index);
                }
            }
            if !rule.blocks.is_empty() {
                for id in 0..registry.len() as u32 {
                    let Some(state) = registry.state(BlockStateId(id)) else {
                        continue;
                    };
                    let hit = rule.blocks.iter().any(|f| {
                        f.id == state.name
                            && f.properties
                                .iter()
                                .all(|(k, v)| state.properties.get(k) == Some(v))
                    });
                    if hit {
                        out.by_state.entry(id).or_default().push(index);
                    }
                }
            }
            out.rules.push(Rule {
                method: rule.method,
                connect: rule.connect,
                faces: rule.faces,
                tiles,
                pattern: rule.pattern_tiles.clone(),
                quarters: rule.quarter_tiles.clone(),
                grid: rule.grid_tiles.clone(),
                width: rule.width.max(1),
                height: rule.height.max(1),
                weights: rule.weights.clone(),
            });
        }
        log::info!("Horizon: {} mod texture rules", out.rules.len());
        out
    }

    fn connected(&self, rule: &Rule, own: u32, layer: u32, face: Direction, other: u32) -> bool {
        match rule.connect {
            RuleConnect::Block => {
                self.block.get(own as usize).is_some()
                    && self.block.get(own as usize) == self.block.get(other as usize)
            }
            RuleConnect::State => own == other,
            RuleConnect::Tile => self.faces.get(other as usize).is_some_and(|faces| {
                faces
                    .iter()
                    .any(|&(f, l)| f == face as u8 && u32::from(l) == layer)
            }),
        }
    }

    /// The tile face `face` of `state` (drawn with picture `layer`) shows at
    /// `pos`; `at` gives the state at an offset.
    pub fn resolve(
        &self,
        state: u32,
        layer: u32,
        face: Direction,
        pos: [i32; 3],
        at: impl Fn([i32; 3]) -> u32,
    ) -> Resolved {
        let empty = Vec::new();
        let by_layer = self.by_layer.get(&layer).unwrap_or(&empty);
        let by_state = self.by_state.get(&state).unwrap_or(&empty);
        let (mut i, mut j) = (0, 0);
        // The two lists are in order of registration; try the rules in it.
        while i < by_layer.len() || j < by_state.len() {
            let next = match (by_layer.get(i), by_state.get(j)) {
                (Some(a), Some(b)) if a == b => {
                    i += 1;
                    j += 1;
                    *a
                }
                (Some(a), Some(b)) if a < b => {
                    i += 1;
                    *a
                }
                (Some(a), None) => {
                    i += 1;
                    *a
                }
                (_, Some(b)) => {
                    j += 1;
                    *b
                }
                (None, None) => break,
            };
            let rule = &self.rules[next as usize];
            if rule.faces & face_bit(face) == 0 {
                continue;
            }
            if let Some(resolved) = self.apply(rule, state, layer, face, pos, &at) {
                return resolved;
            }
        }
        Resolved::Keep
    }

    fn apply(
        &self,
        rule: &Rule,
        state: u32,
        layer: u32,
        face: Direction,
        pos: [i32; 3],
        at: &impl Fn([i32; 3]) -> u32,
    ) -> Option<Resolved> {
        let (right, up) = axes(face);
        let near = |r: i32, u: i32| {
            let other = at([
                right[0] * r + up[0] * u,
                right[1] * r + up[1] * u,
                right[2] * r + up[2] * u,
            ]);
            self.connected(rule, state, layer, face, other)
        };
        let tile = |entry: u32| {
            if entry == NONE {
                return None;
            }
            rule.tiles.get(entry as usize).copied().flatten()
        };
        match rule.method {
            RuleMethod::Pattern => {
                let neighbours = [
                    (-1, 0),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (1, 0),
                    (1, 1),
                    (0, 1),
                    (-1, 1),
                ];
                let mut bits = 0usize;
                for (bit, (r, u)) in neighbours.into_iter().enumerate() {
                    if near(r, u) {
                        bits |= 1 << bit;
                    }
                }
                tile(*rule.pattern.get(bits)?).map(Resolved::Layer)
            }
            RuleMethod::Random => {
                let weight = |i: usize| rule.weights.get(i).copied().unwrap_or(1);
                let total: u64 = (0..rule.tiles.len()).map(|i| u64::from(weight(i))).sum();
                if total == 0 {
                    return None;
                }
                let mut pick = u64::from(hash(pos, face)) % total;
                for i in 0..rule.tiles.len() {
                    let w = u64::from(weight(i));
                    if pick < w {
                        return rule.tiles[i].map(Resolved::Layer);
                    }
                    pick -= w;
                }
                None
            }
            RuleMethod::Position => {
                let dot = |a: [i32; 3]| a[0] * pos[0] + a[1] * pos[1] + a[2] * pos[2];
                let u = dot(right);
                let v = -dot(up);
                let cell = (v.rem_euclid(rule.height as i32) as u32) * rule.width
                    + u.rem_euclid(rule.width as i32) as u32;
                tile(*rule.grid.get(cell as usize)?).map(Resolved::Layer)
            }
            RuleMethod::Quarters => {
                let mut out = [0u16; 4];
                for (q, slot) in out.iter_mut().enumerate() {
                    let side = if q % 2 == 0 { -1 } else { 1 };
                    let vertical = if q < 2 { 1 } else { -1 };
                    let h = near(side, 0);
                    let v = near(0, vertical);
                    let d = near(side, vertical);
                    let index = q * 8 + usize::from(h) + 2 * usize::from(v) + 4 * usize::from(d);
                    *slot = tile(*rule.quarters.get(index)?)?;
                }
                Some(Resolved::Quarters(out))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(method: RuleMethod, connect: RuleConnect) -> Rule {
        Rule {
            method,
            connect,
            faces: 63,
            tiles: (0..4).map(|i| Some(10 + i)).collect(),
            pattern: Vec::new(),
            quarters: Vec::new(),
            grid: Vec::new(),
            width: 1,
            height: 1,
            weights: Vec::new(),
        }
    }
    fn rules(rule: Rule) -> ModRules {
        let mut r = ModRules::default();
        r.rules.push(rule);
        r.by_state.insert(1, vec![0]);
        r.block = vec![0, 1, 1];
        r
    }

    #[test]
    fn a_pattern_rule_picks_the_tile_of_the_connected_neighbours() {
        let mut rule = rule(RuleMethod::Pattern, RuleConnect::State);
        rule.pattern = vec![NONE; 256];
        rule.pattern[0] = 0;
        rule.pattern[16] = 1; // a neighbour on the right only
        rule.pattern[1] = 2; // a neighbour on the left only
        rule.pattern[17] = 3; // both
        rule.pattern[64] = NONE; // above only: no tile
        let rules = rules(rule);
        // A row of state 1 along x; the south face's right is +x.
        let row = |dx: i32| if (-1..=1).contains(&dx) { 1 } else { 0 };
        let at = |[dx, dy, dz]: [i32; 3]| if dy == 0 && dz == 0 { row(dx) } else { 0 };
        let tile = |x: i32| {
            rules.resolve(1, 5, Direction::South, [x, 0, 0], |[dx, dy, dz]| {
                at([dx + x, dy, dz])
            })
        };
        assert_eq!(tile(0), Resolved::Layer(13));
        assert_eq!(tile(-1), Resolved::Layer(11));
        assert_eq!(tile(1), Resolved::Layer(12));
        // Nothing connected: tile 0.
        assert_eq!(tile(5), Resolved::Layer(10));
        // A pattern without a tile leaves the face as it is.
        let above = rules.resolve(1, 5, Direction::South, [0; 3], |[dx, dy, dz]| {
            if [dx, dy, dz] == [0, 1, 0] { 1 } else { 0 }
        });
        assert_eq!(above, Resolved::Keep);
    }

    #[test]
    fn a_rule_only_matches_its_faces() {
        let mut rule = rule(RuleMethod::Position, RuleConnect::Block);
        rule.grid = vec![1];
        rule.faces = 16; // up
        let rules = rules(rule);
        assert_eq!(
            rules.resolve(1, 0, Direction::Up, [0; 3], |_| 0),
            Resolved::Layer(11)
        );
        assert_eq!(
            rules.resolve(1, 0, Direction::North, [0; 3], |_| 0),
            Resolved::Keep
        );
    }

    #[test]
    fn a_position_rule_repeats_its_grid_along_the_face() {
        let mut rule = rule(RuleMethod::Position, RuleConnect::Block);
        rule.width = 2;
        rule.height = 2;
        rule.grid = vec![0, 1, 2, 3];
        let rules = rules(rule);
        let cell = |x: i32, y: i32| rules.resolve(1, 0, Direction::South, [x, y, 0], |_| 0);
        // On the south face u grows with x and v with -y.
        assert_eq!(cell(0, 0), Resolved::Layer(10));
        assert_eq!(cell(1, 0), Resolved::Layer(11));
        assert_eq!(cell(0, -1), Resolved::Layer(12));
        assert_eq!(cell(1, -1), Resolved::Layer(13));
        assert_eq!(cell(2, 2), Resolved::Layer(10));
        // Seen from the north, u runs against x.
        let north = |x: i32| rules.resolve(1, 0, Direction::North, [x, 0, 0], |_| 0);
        assert_eq!(north(0), Resolved::Layer(10));
        assert_eq!(north(-1), Resolved::Layer(11));
    }

    #[test]
    fn a_random_rule_keeps_its_tile_and_honours_weights() {
        let mut rule = rule(RuleMethod::Random, RuleConnect::Block);
        rule.weights = vec![0, 0, 5, 0];
        let rules = rules(rule);
        for x in 0..20 {
            assert_eq!(
                rules.resolve(1, 0, Direction::Up, [x, 3, 7], |_| 0),
                Resolved::Layer(12)
            );
        }
        let mut even = rule_of_four();
        even.weights = Vec::new();
        let even = self::rules(even);
        let picks: std::collections::HashSet<_> = (0..64)
            .map(
                |x| match even.resolve(1, 0, Direction::Up, [x, 0, 0], |_| 0) {
                    Resolved::Layer(l) => l,
                    other => panic!("{other:?}"),
                },
            )
            .collect();
        assert!(picks.len() > 1);
        assert_eq!(
            even.resolve(1, 0, Direction::Up, [3, 0, 0], |_| 0),
            even.resolve(1, 0, Direction::Up, [3, 0, 0], |_| 0)
        );
    }
    fn rule_of_four() -> Rule {
        rule(RuleMethod::Random, RuleConnect::Block)
    }

    #[test]
    fn a_quarters_rule_looks_at_each_quarters_own_neighbours() {
        let mut rule = rule(RuleMethod::Quarters, RuleConnect::State);
        // Quarter q draws tile q when nothing connects, and tile 3 when the
        // side, the vertical neighbour and the corner all do.
        rule.quarters = (0..32)
            .map(|i| if i % 8 == 7 { 3 } else { (i / 8) as u32 })
            .collect();
        let rules = rules(rule);
        let alone = rules.resolve(1, 0, Direction::South, [0; 3], |_| 0);
        assert_eq!(alone, Resolved::Quarters([10, 11, 12, 13]));
        // Everything around is connected.
        let full = rules.resolve(1, 0, Direction::South, [0; 3], |[x, y, z]| {
            if [x, y, z] == [0, 0, 0] { 0 } else { 1 }
        });
        assert_eq!(full, Resolved::Quarters([13, 13, 13, 13]));
    }
}
