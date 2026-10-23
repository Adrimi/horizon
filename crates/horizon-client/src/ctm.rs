//! Connected textures from resource packs in OptiFine's CTM format
//! (`assets/<namespace>/optifine/ctm/**/*.properties`), the extra Continuity
//! brings to Java: a face of a matched block shows the tile that fits the
//! matching blocks around it on the same plane. Off by default
//! (`horizon-extras.txt`, `connectedTextures`); with it off, or without a pack
//! that has CTM, sections mesh exactly as vanilla's.
//!
//! Methods: `ctm` (47 tiles), `ctm_compact` (5 tiles, drawn by quarters),
//! `horizontal`, `vertical`, `top`, `random` and `repeat`.
use crate::sprites::Sprites;
use horizon_core::geometry::Direction;
use horizon_world::registry::{BlockRegistry, BlockStateId};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// The `connectedTextures` extra; the mesher reads it on its workers.
pub static ENABLED: AtomicBool = AtomicBool::new(false);
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Ctm,
    Compact,
    Horizontal,
    Vertical,
    Top,
    Random,
    Repeat,
}
impl Method {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ctm" | "glass" => Self::Ctm,
            "ctm_compact" => Self::Compact,
            "horizontal" | "bookshelf" => Self::Horizontal,
            "vertical" => Self::Vertical,
            "top" => Self::Top,
            "random" => Self::Random,
            "repeat" => Self::Repeat,
            _ => return None,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connect {
    Block,
    Tile,
    /// The neighbour's whole state (block and properties) must be equal.
    State,
}

/// A `matchBlocks` entry: a block and optional property values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMatch {
    pub name: String,
    pub properties: Vec<(String, Vec<String>)>,
}
impl BlockMatch {
    fn parse(entry: &str) -> Self {
        // `name`, `ns:name`, then `:key=v1,v2` filters.
        let mut parts: Vec<&str> = entry.split(':').collect();
        let mut properties = Vec::new();
        while parts.len() > 1 && parts.last().is_some_and(|p| p.contains('=')) {
            let (k, v) = parts.pop().unwrap().split_once('=').unwrap();
            properties.push((k.to_owned(), v.split(',').map(str::to_owned).collect()));
        }
        properties.reverse();
        let name = match parts.as_slice() {
            [name] => format!("minecraft:{name}"),
            [ns, name, ..] => format!("{ns}:{name}"),
            [] => String::new(),
        };
        Self { name, properties }
    }
    fn matches(&self, name: &str, properties: &std::collections::BTreeMap<String, String>) -> bool {
        self.name == name
            && self.properties.iter().all(|(k, values)| {
                properties
                    .get(k)
                    .is_some_and(|v| values.iter().any(|w| w == v))
            })
    }
}

/// One `.properties` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Properties {
    pub method: Method,
    pub tiles: Vec<String>,
    pub match_tiles: Vec<String>,
    pub match_blocks: Vec<BlockMatch>,
    /// Bit per `Direction` index.
    pub faces: u8,
    pub connect: Option<Connect>,
    pub width: u32,
    pub height: u32,
    pub weights: Vec<u32>,
}
const ALL_FACES: u8 = 0b11_1111;
const SIDES: u8 = 0b11_1100;

fn faces(value: &str) -> u8 {
    value
        .split([' ', ','])
        .filter(|s| !s.is_empty())
        .map(|f| match f {
            "all" => ALL_FACES,
            "sides" => SIDES,
            "top" | "up" => 1 << Direction::Up as u8,
            "bottom" | "down" => 1 << Direction::Down as u8,
            "north" => 1 << Direction::North as u8,
            "south" => 1 << Direction::South as u8,
            "west" => 1 << Direction::West as u8,
            "east" => 1 << Direction::East as u8,
            _ => 0,
        })
        .fold(0, |a, b| a | b)
}
/// `0-3 7 9-10` style lists to their entries.
fn expand(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in value.split_whitespace() {
        if let Some((a, b)) = token.split_once('-')
            && let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>())
        {
            out.extend((a..=b).map(|n| n.to_string()));
        } else {
            out.push(token.to_owned());
        }
    }
    out
}

/// Parses a properties file; `stem` is its file name without extension,
/// which names the matched tile or block when the file does not.
pub fn parse(text: &str, stem: &str) -> Result<Properties, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some(split) = line.find(['=', ':']) else {
            continue;
        };
        map.insert(line[..split].trim(), line[split + 1..].trim());
    }
    let method = map
        .get("method")
        .map_or(Some(Method::Ctm), |m| Method::parse(m))
        .ok_or_else(|| format!("unsupported method {:?}", map["method"]))?;
    let tiles = map.get("tiles").map(|t| expand(t)).unwrap_or_default();
    if tiles.is_empty() {
        return Err("no tiles".into());
    }
    let mut match_tiles: Vec<String> = map
        .get("matchTiles")
        .map(|t| t.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default();
    let mut match_blocks: Vec<BlockMatch> = map
        .get("matchBlocks")
        .map(|b| b.split_whitespace().map(BlockMatch::parse).collect())
        .unwrap_or_default();
    if match_tiles.is_empty() && match_blocks.is_empty() {
        match stem.strip_prefix("block_") {
            Some(block) => match_blocks.push(BlockMatch::parse(block)),
            None => match_tiles.push(stem.to_owned()),
        }
    }
    let number = |key: &str| map.get(key).and_then(|v| v.parse().ok());
    Ok(Properties {
        method,
        tiles,
        match_tiles,
        match_blocks,
        faces: map.get("faces").map_or(ALL_FACES, |f| faces(f)),
        connect: match map.get("connect").copied() {
            Some("block") => Some(Connect::Block),
            Some("tile") => Some(Connect::Tile),
            Some("state") => Some(Connect::State),
            _ => None,
        },
        width: number("width").unwrap_or(1).max(1),
        height: number("height").unwrap_or(1).max(1),
        weights: map
            .get("weights")
            .map(|w| {
                w.split_whitespace()
                    .filter_map(|n| n.parse().ok())
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// OptiFine's 47-tile template: the tile of every connection pattern, indexed
/// by the bits of the sides and inner corners around a face:
///
/// ```text
/// 128  64  32      top-left    up     top-right
///   1   *   16      left        face   right
///   2   4   8       bottom-left down   bottom-right
/// ```
///
/// A corner bit is set only when both its sides connect and the diagonal
/// block does too; the notched variants are the template's inner corners.
pub const INDEX: [u8; 256] = [
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    36, 17, 36, 17, 24, 19, 24, 43, 36, 17, 36, 17, 24, 19, 24, 43, //
    16, 18, 16, 18, 6, 46, 6, 21, 16, 18, 16, 18, 28, 9, 28, 22, //
    36, 17, 36, 17, 24, 19, 24, 43, 36, 17, 36, 17, 24, 19, 24, 43, //
    37, 40, 37, 40, 30, 8, 30, 34, 37, 40, 37, 40, 25, 23, 25, 45, //
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    36, 39, 36, 39, 24, 41, 24, 27, 36, 39, 36, 39, 24, 41, 24, 27, //
    16, 42, 16, 42, 6, 20, 6, 10, 16, 42, 16, 42, 28, 35, 28, 44, //
    36, 39, 36, 39, 24, 41, 24, 27, 36, 39, 36, 39, 24, 41, 24, 27, //
    37, 38, 37, 38, 30, 11, 30, 32, 37, 38, 37, 38, 25, 33, 25, 26, //
];

/// The tile of a connection pattern (see [`INDEX`]).
pub fn ctm_index(connections: u8) -> usize {
    usize::from(INDEX[usize::from(connections)])
}
/// A `ctm_compact` quarter's tile from its horizontal and vertical sides and
/// its diagonal: 0 unconnected, 1 fully connected, 2 vertical, 3 horizontal,
/// 4 both sides but an open inner corner.
pub fn compact_index(horizontal: bool, vertical: bool, diagonal: bool) -> usize {
    match (horizontal, vertical) {
        (false, false) => 0,
        (true, false) => 3,
        (false, true) => 2,
        (true, true) if diagonal => 1,
        (true, true) => 4,
    }
}

/// The world directions a face's texture runs along: right (u grows) and up
/// (v shrinks), as vanilla's cube faces map their UVs.
pub fn axes(face: Direction) -> ([i32; 3], [i32; 3]) {
    match face {
        Direction::Up => ([1, 0, 0], [0, 0, -1]),
        Direction::Down => ([1, 0, 0], [0, 0, 1]),
        Direction::North => ([-1, 0, 0], [0, 1, 0]),
        Direction::South => ([1, 0, 0], [0, 1, 0]),
        Direction::West => ([0, 0, 1], [0, 1, 0]),
        Direction::East => ([0, 0, -1], [0, 1, 0]),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tile {
    Layer(u16),
    /// `<default>`: the face keeps its own sprite.
    Default,
}
struct Rule {
    method: Method,
    tiles: Vec<Tile>,
    faces: u8,
    connect: Connect,
    width: u32,
    height: u32,
    weights: Vec<u32>,
    /// States that count as the same tile for `connect=tile`.
    members: HashSet<u32>,
}

/// What a face draws instead of its sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    Keep,
    Layer(u16),
    /// `ctm_compact`: top-left, top-right, bottom-left, bottom-right quarters.
    Quarters([u16; 4]),
}

/// The loaded rules of every pack.
#[derive(Default)]
pub struct Ctm {
    rules: Vec<Rule>,
    by_layer: HashMap<u32, Vec<u16>>,
    by_state: HashMap<u32, Vec<u16>>,
    /// Each state's block, for `connect=block`.
    block: Vec<u32>,
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "properties") {
            out.push(path);
        }
    }
}
/// A `matchTiles` name as a sprite name: `glass`, `block/glass` and
/// `minecraft:block/glass` all name `minecraft:block/glass`.
fn sprite_name(name: &str) -> String {
    let name = name.trim_end_matches(".png");
    let (ns, path) = name.split_once(':').unwrap_or(("minecraft", name));
    let path = path.strip_prefix("textures/").unwrap_or(path);
    if path.contains('/') {
        format!("{ns}:{path}")
    } else {
        format!("{ns}:block/{path}")
    }
}
/// A `tiles` entry's image: numbers and bare names sit beside the
/// properties file, `~/` is the namespace's `optifine/` folder, and a path
/// with a namespace or folder is a texture.
fn tile_path(root: &Path, dir: &Path, namespace: &str, entry: &str) -> PathBuf {
    let file = if entry.ends_with(".png") {
        entry.to_owned()
    } else {
        format!("{entry}.png")
    };
    if let Some(rest) = file.strip_prefix("~/") {
        return root.join(format!("assets/{namespace}/optifine/{rest}"));
    }
    if let Some(rest) = file.strip_prefix("./") {
        return dir.join(rest);
    }
    if let Some((ns, path)) = file.split_once(':') {
        return root.join(format!("assets/{ns}/textures/{path}"));
    }
    if file.contains('/') {
        return root.join(format!("assets/{namespace}/textures/{file}"));
    }
    dir.join(file)
}

impl Ctm {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
    /// Reads every pack's CTM files under `root` (the merged resources) and
    /// adds their tiles to `sprites`; `models` names the sprites each state
    /// shows, for `connect=tile`.
    pub fn load(
        root: &Path,
        registry: &BlockRegistry,
        sprites: &mut Sprites,
        state_layers: impl Fn(BlockStateId) -> Vec<u32>,
    ) -> Self {
        let mut files = Vec::new();
        if let Ok(namespaces) = fs::read_dir(root.join("assets")) {
            for ns in namespaces.flatten() {
                walk(&ns.path().join("optifine/ctm"), &mut files);
            }
        }
        files.sort();
        let mut ctm = Self::default();
        if files.is_empty() {
            return ctm;
        }
        let mut blocks = HashMap::new();
        ctm.block = (0..registry.len() as u32)
            .map(|id| {
                let name = registry
                    .state(BlockStateId(id))
                    .map_or("", |s| s.name.as_str());
                let next = blocks.len() as u32;
                *blocks.entry(name.to_owned()).or_insert(next)
            })
            .collect();
        let mut layers_of: Option<Vec<Vec<u32>>> = None;
        for path in files {
            let stem = path
                .file_stem()
                .map_or(String::new(), |s| s.to_string_lossy().into());
            let namespace = path
                .strip_prefix(root.join("assets"))
                .ok()
                .and_then(|p| p.components().next())
                .map_or("minecraft".into(), |c| {
                    c.as_os_str().to_string_lossy().into_owned()
                });
            let parsed = fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| parse(&t, &stem));
            let props = match parsed {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("Horizon: CTM {}: {e}", path.display());
                    continue;
                }
            };
            let dir = path.parent().unwrap_or(root);
            let mut tiles = Vec::new();
            for (i, entry) in props.tiles.iter().enumerate() {
                if entry == "<default>" || entry == "<skip>" {
                    tiles.push(Tile::Default);
                    continue;
                }
                let file = tile_path(root, dir, &namespace, entry);
                let key = format!("ctm:{}#{i}", path.display());
                match sprites.insert_file(&key, &file) {
                    Ok(layer) => tiles.push(Tile::Layer(layer as u16)),
                    Err(e) => {
                        log::warn!("Horizon: CTM tile {}: {e}", file.display());
                        tiles.push(Tile::Default);
                    }
                }
            }
            let index = ctm.rules.len() as u16;
            let mut members = HashSet::new();
            let match_layers: Vec<u32> = props
                .match_tiles
                .iter()
                .filter_map(|t| sprites.layers.get(&sprite_name(t)).copied())
                .collect();
            for layer in &match_layers {
                ctm.by_layer.entry(*layer).or_default().push(index);
            }
            if !props.match_blocks.is_empty() {
                for id in 0..registry.len() as u32 {
                    let Some(state) = registry.state(BlockStateId(id)) else {
                        continue;
                    };
                    if props
                        .match_blocks
                        .iter()
                        .any(|m| m.matches(&state.name, &state.properties))
                    {
                        ctm.by_state.entry(id).or_default().push(index);
                        members.insert(id);
                    }
                }
            }
            if !match_layers.is_empty() {
                let all = layers_of.get_or_insert_with(|| {
                    (0..registry.len() as u32)
                        .map(|id| state_layers(BlockStateId(id)))
                        .collect()
                });
                for (id, layers) in all.iter().enumerate() {
                    if layers.iter().any(|l| match_layers.contains(l)) {
                        members.insert(id as u32);
                    }
                }
            }
            let connect = props.connect.unwrap_or(if props.match_blocks.is_empty() {
                Connect::Tile
            } else {
                Connect::Block
            });
            ctm.rules.push(Rule {
                method: props.method,
                tiles,
                faces: props.faces,
                connect,
                width: props.width,
                height: props.height,
                weights: props.weights,
                members,
            });
        }
        log::info!("Horizon: {} connected-texture rules", ctm.rules.len());
        ctm
    }

    fn connected(&self, rule: &Rule, own: u32, other: u32) -> bool {
        match rule.connect {
            Connect::Block => {
                self.block.get(own as usize).is_some()
                    && self.block.get(own as usize) == self.block.get(other as usize)
            }
            Connect::Tile => rule.members.contains(&other),
            Connect::State => own == other,
        }
    }

    /// The tile face `face` of state `state`, drawn with sprite `layer`, shows
    /// at world position `pos`; `at` gives the state at an offset.
    pub fn resolve(
        &self,
        state: u32,
        layer: u32,
        face: Direction,
        pos: [i32; 3],
        at: impl Fn([i32; 3]) -> u32,
    ) -> Resolved {
        let rules = self
            .by_layer
            .get(&layer)
            .into_iter()
            .chain(self.by_state.get(&state))
            .flatten();
        for &index in rules {
            let rule = &self.rules[index as usize];
            if rule.faces & (1 << face as u8) == 0 {
                continue;
            }
            if let Some(resolved) = self.apply(rule, state, face, pos, &at) {
                return resolved;
            }
        }
        Resolved::Keep
    }

    fn apply(
        &self,
        rule: &Rule,
        state: u32,
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
            self.connected(rule, state, other)
        };
        let tile = |i: usize| match rule.tiles.get(i) {
            Some(Tile::Layer(l)) => Some(Resolved::Layer(*l)),
            _ => None,
        };
        // Texture-space sides (left, down, right, up) and corners
        // (bottom-left, bottom-right, top-right, top-left).
        let sides = [near(-1, 0), near(0, -1), near(1, 0), near(0, 1)];
        let mut connections = 0u8;
        for (i, side) in sides.iter().enumerate() {
            if *side {
                connections |= 1 << (i * 2);
            }
        }
        for (i, [dr, du]) in [[-1, -1], [1, -1], [1, 1], [-1, 1]].into_iter().enumerate() {
            let next = (i + 1) % 4;
            if sides[i] && sides[next] && near(dr, du) {
                connections |= 1 << (i * 2 + 1);
            }
        }
        match rule.method {
            Method::Ctm => tile(ctm_index(connections)),
            Method::Horizontal => {
                tile([3, 2, 0, 1][usize::from(sides[0]) + 2 * usize::from(sides[2])])
            }
            Method::Vertical => {
                tile([3, 2, 0, 1][usize::from(sides[1]) + 2 * usize::from(sides[3])])
            }
            Method::Top => {
                if matches!(face, Direction::Up | Direction::Down) {
                    return None;
                }
                let above = at([0, 1, 0]);
                self.connected(rule, state, above)
                    .then(|| tile(0))
                    .flatten()
            }
            Method::Random => {
                let weights = if rule.weights.len() == rule.tiles.len() {
                    rule.weights.clone()
                } else {
                    vec![1; rule.tiles.len()]
                };
                let total: u64 = weights.iter().map(|&w| u64::from(w)).sum();
                if total == 0 {
                    return None;
                }
                let mut roll = hash(pos, face as i32) % total;
                for (i, w) in weights.iter().enumerate() {
                    if roll < u64::from(*w) {
                        return tile(i);
                    }
                    roll -= u64::from(*w);
                }
                None
            }
            Method::Repeat => {
                // The pattern runs along the face's texture (Continuity's
                // per-face coordinates, which follow OptiFine's quirk of
                // mirroring the rows on side faces).
                let [x, y, z] = pos;
                let (sx, sy) = match face {
                    Direction::Down => (x, -z - 1),
                    Direction::Up => (x, z),
                    Direction::North => (-x - 1, -y),
                    Direction::South => (x, -y),
                    Direction::West => (z, -y),
                    Direction::East => (-z - 1, -y),
                };
                let (w, h) = (rule.width as i32, rule.height as i32);
                tile((sy.rem_euclid(h) * w + sx.rem_euclid(w)) as usize)
            }
            Method::Compact => {
                let quarter = |horizontal: bool, vertical: bool, [dr, du]: [i32; 2]| {
                    let diagonal = horizontal && vertical && near(dr, du);
                    match rule
                        .tiles
                        .get(compact_index(horizontal, vertical, diagonal))
                    {
                        Some(Tile::Layer(l)) => Some(*l),
                        _ => None,
                    }
                };
                Some(Resolved::Quarters([
                    quarter(sides[0], sides[3], [-1, 1])?,
                    quarter(sides[2], sides[3], [1, 1])?,
                    quarter(sides[0], sides[1], [-1, -1])?,
                    quarter(sides[2], sides[1], [1, -1])?,
                ]))
            }
        }
    }
}

/// A stable per-face hash for `random`.
fn hash([x, y, z]: [i32; 3], face: i32) -> u64 {
    let mut h = (x as i64).wrapping_mul(3_129_871) as u64
        ^ (z as i64).wrapping_mul(116_129_781) as u64
        ^ (y as i64) as u64
        ^ ((face as u64) << 40);
    h = h.wrapping_mul(h.wrapping_mul(42_317_861).wrapping_add(11));
    h ^ (h >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_parse_with_ranges_faces_and_blocks() {
        let p = parse(
            "# glass\nmethod=ctm\ntiles=0-3 7 <default>\nmatchBlocks=glass minecraft:oak_log:axis=y,x\nfaces=sides top\nconnect=tile\n",
            "glass",
        )
        .unwrap();
        assert_eq!(p.method, Method::Ctm);
        assert_eq!(p.tiles, ["0", "1", "2", "3", "7", "<default>"]);
        assert_eq!(p.faces, SIDES | 1 << Direction::Up as u8);
        assert_eq!(p.connect, Some(Connect::Tile));
        assert_eq!(p.match_blocks[0].name, "minecraft:glass");
        assert_eq!(p.match_blocks[1].name, "minecraft:oak_log");
        assert_eq!(
            p.match_blocks[1].properties,
            [("axis".to_owned(), vec!["y".to_owned(), "x".to_owned()])]
        );
        assert!(p.match_tiles.is_empty());
    }

    #[test]
    fn the_file_name_names_the_tile_or_block() {
        let tile = parse("method=horizontal\ntiles=0-3", "bookshelf").unwrap();
        assert_eq!(tile.match_tiles, ["bookshelf"]);
        assert_eq!(tile.faces, ALL_FACES);
        let block = parse("method=top\ntiles=0", "block_sandstone").unwrap();
        assert_eq!(block.match_blocks[0].name, "minecraft:sandstone");
        assert!(parse("method=spiral\ntiles=0", "x").is_err());
        assert!(parse("method=ctm", "x").is_err());
    }

    #[test]
    fn the_47_tile_index_follows_the_edges_and_corners() {
        // Bit layout: left 1, bottom-left 2, down 4, bottom-right 8,
        // right 16, top-right 32, up 64, top-left 128.
        assert_eq!(ctm_index(0), 0);
        assert_eq!(ctm_index(1), 3); // left
        assert_eq!(ctm_index(16), 1); // right
        assert_eq!(ctm_index(4), 12); // down
        assert_eq!(ctm_index(64), 36); // up
        assert_eq!(ctm_index(1 | 16), 2); // left and right
        assert_eq!(ctm_index(1 | 4), 5); // left and down
        assert_eq!(ctm_index(4 | 16), 4); // down and right
        // Surrounded on all sides and corners: the template's centre.
        assert_eq!(ctm_index(0xFF), 26);
        // An open inner corner changes the tile.
        assert_eq!(ctm_index(1 | 4 | 16 | 64), 46);
        assert_eq!(ctm_index(1 | 4 | 16 | 64 | 2), 21);
        // Every pattern is one of the 47 tiles, and all are used.
        let mut seen = [false; 47];
        for c in 0..=u8::MAX {
            let i = ctm_index(c);
            assert!(i < 47);
            seen[i] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn compact_quarters_pick_their_tile() {
        assert_eq!(compact_index(false, false, false), 0);
        assert_eq!(compact_index(true, true, true), 1);
        assert_eq!(compact_index(true, false, false), 3);
        assert_eq!(compact_index(false, true, false), 2);
        assert_eq!(compact_index(true, true, false), 4);
    }

    #[test]
    fn tile_paths_resolve_like_optifine() {
        let root = Path::new("/r");
        let dir = Path::new("/r/assets/minecraft/optifine/ctm/glass");
        assert_eq!(tile_path(root, dir, "minecraft", "3"), dir.join("3.png"));
        assert_eq!(
            tile_path(root, dir, "minecraft", "block/glass"),
            root.join("assets/minecraft/textures/block/glass.png")
        );
        assert_eq!(
            tile_path(root, dir, "minecraft", "~/ctm/x/1"),
            root.join("assets/minecraft/optifine/ctm/x/1.png")
        );
        assert_eq!(sprite_name("glass"), "minecraft:block/glass");
        assert_eq!(
            sprite_name("minecraft:block/glass"),
            "minecraft:block/glass"
        );
    }

    fn wall_rule(method: Method, tiles: u16) -> Ctm {
        // States 1 and 2 are one block, 3 another.
        Ctm {
            rules: vec![Rule {
                method,
                tiles: (0..tiles).map(|i| Tile::Layer(100 + i)).collect(),
                faces: ALL_FACES,
                connect: Connect::Block,
                width: 2,
                height: 2,
                weights: Vec::new(),
                members: HashSet::new(),
            }],
            by_layer: HashMap::from([(7, vec![0])]),
            by_state: HashMap::new(),
            block: vec![0, 1, 1, 2],
        }
    }

    #[test]
    fn a_wall_of_glass_connects_across_its_plane() {
        let ctm = wall_rule(Method::Ctm, 47);
        // A 3-wide south-facing row: x = -1, 0, 1 hold state 1.
        let at = |[x, y, z]: [i32; 3]| u32::from(z == 0 && y == 0 && (-1..=1).contains(&x));
        assert_eq!(
            ctm.resolve(1, 7, Direction::South, [0, 0, 0], at),
            Resolved::Layer(102)
        );
        // From the row's west end only the east neighbour connects.
        let west_end = |[x, y, z]: [i32; 3]| u32::from(z == 0 && y == 0 && (0..=2).contains(&x));
        assert_eq!(
            ctm.resolve(1, 7, Direction::South, [0, 0, 0], west_end),
            Resolved::Layer(101)
        );
        // Another sprite or a face of a lone block.
        assert_eq!(
            ctm.resolve(1, 8, Direction::South, [0, 0, 0], at),
            Resolved::Keep
        );
        assert_eq!(
            ctm.resolve(1, 7, Direction::Up, [0, 0, 0], |_| 3),
            Resolved::Layer(100)
        );
    }

    #[test]
    fn compact_and_repeat_resolve() {
        let ctm = wall_rule(Method::Compact, 5);
        // State 2 is the same block as the face's state 1: every quarter is
        // fully connected with its diagonal, so all use tile 1.
        let all = |_: [i32; 3]| 2;
        assert_eq!(
            ctm.resolve(1, 7, Direction::North, [0, 0, 0], all),
            Resolved::Quarters([101; 4])
        );
        let repeat = wall_rule(Method::Repeat, 4);
        let pick = |pos| match repeat.resolve(1, 7, Direction::South, pos, |_| 0) {
            Resolved::Layer(l) => l - 100,
            other => panic!("{other:?}"),
        };
        // South faces mirror the pattern's rows (OptiFine's coordinates).
        assert_eq!(
            [
                pick([0, 1, 0]),
                pick([1, 1, 0]),
                pick([0, 0, 0]),
                pick([1, 0, 0])
            ],
            [2, 3, 0, 1]
        );
    }

    #[test]
    fn inner_corners_follow_the_diagonal() {
        let ctm = wall_rule(Method::Ctm, 47);
        // A 3x3 south wall centred on (0, 0, 0); the face at (0, 0, 0) has
        // all four sides connected.
        let wall = |[x, y, z]: [i32; 3]| {
            u32::from(z == 0 && (-1..=1).contains(&x) && (-1..=1).contains(&y))
        };
        // All four diagonals are part of the wall: seamless corners.
        assert_eq!(
            ctm.resolve(1, 7, Direction::South, [0, 0, 0], wall),
            Resolved::Layer(100 + ctm_index(0xFF) as u16)
        );
        // Without the top-left diagonal the top-left corner gets its notch.
        let missing = |[x, y, z]: [i32; 3]| {
            u32::from(
                z == 0 && (-1..=1).contains(&x) && (-1..=1).contains(&y) && !(x == -1 && y == 1),
            )
        };
        assert_eq!(
            ctm.resolve(1, 7, Direction::South, [0, 0, 0], missing),
            Resolved::Layer(100 + ctm_index(0xFF & !128) as u16)
        );
        assert_ne!(ctm_index(0xFF), ctm_index(0xFF & !128));
    }

    #[test]
    fn connecting_by_state_ignores_the_block() {
        let mut ctm = wall_rule(Method::Ctm, 47);
        ctm.rules[0].connect = Connect::State;
        // State 1 on both sides of the face: the same state connects, the
        // other state of the same block (2) does not.
        let at = |[x, y, z]: [i32; 3]| match (x, y, z) {
            (1, 0, 0) => 1,
            (0, 1, 0) => 2,
            (0, 0, 0) => 1,
            _ => 0,
        };
        assert_eq!(
            ctm.resolve(1, 7, Direction::South, [0, 0, 0], at),
            Resolved::Layer(101)
        );
    }
}
