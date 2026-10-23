//! The map surface of a chunk, for the mods' `world.get-chunk-surface`
//! (API 0.1.4): each column's map colour, height and water depth, as the
//! map item sees them. The scan is the game's, so a mod does not walk the
//! blocks itself.
use horizon_client::{
    blocks::{BlockFacts, Fluid},
    chunk::{Chunk, MIN_Y, SECTIONS},
};

/// Water's and lava's map colours (`MapColor.WATER`, `MapColor.FIRE`).
const WATER: u8 = 12;
const LAVA: u8 = 4;
/// The flags of a block state in the scan.
const FLUID: u8 = 3;
const AIR: u8 = 4;
const STURDY_TOP: u8 = 8;

/// What the scan needs of each block state.
#[derive(Clone, Debug, Default)]
pub struct Palette {
    /// Map colour id, 0 for none.
    color: Vec<u8>,
    /// 0 none, 1 water, 2 lava; | 4 for air; | 8 when sturdy on top.
    flags: Vec<u8>,
}
impl Palette {
    /// The colours of every block state, built once.
    pub fn new(facts: &BlockFacts) -> Self {
        let mut color = Vec::with_capacity(facts.states.len());
        let mut flags = Vec::with_capacity(facts.states.len());
        for (i, f) in facts.states.iter().enumerate() {
            color.push(f.map_color);
            let mut b = match f.fluid {
                Fluid::None => 0,
                Fluid::Water => 1,
                Fluid::Lava => 2,
            };
            if facts.is_air(horizon_world::registry::BlockStateId(i as u32)) {
                b |= AIR;
            }
            // isFaceSturdy(UP, FULL): Direction.UP's ordinal is 1.
            if f.sturdy_full >> 1 & 1 != 0 {
                b |= STURDY_TOP;
            }
            flags.push(b);
        }
        Self { color, flags }
    }
    /// A state without a colour: the scan finds nothing (before the
    /// resources load).
    /// The map colour and the flags of a state.
    fn get(&self, id: u32) -> (u8, u8) {
        let i = id as usize;
        (
            self.color.get(i).copied().unwrap_or(0),
            self.flags.get(i).copied().unwrap_or(AIR),
        )
    }
}

/// One chunk's columns as the map sees them, `z * 16 + x`.
#[derive(Clone, Debug, PartialEq)]
pub struct Columns {
    /// The map colour of each column, 0 for none.
    pub color: [u8; 256],
    /// The height of each column: the Y of its topmost coloured block.
    pub height: [i16; 256],
    /// The water (or lava) depth over each column's surface.
    pub depth: [u8; 256],
}

/// The highest block of a chunk worth scanning from.
fn top_of(chunk: &Chunk, air: u32) -> i32 {
    (0..SECTIONS)
        .rev()
        .find(|&s| {
            chunk
                .sections
                .get(s)
                .is_some_and(|sec| !sec.is_uniform(air))
        })
        .map_or(MIN_Y, |s| MIN_Y + s as i32 * 16 + 15)
}

/// `MapItem.update` for one column: the first block with a map colour
/// going down, the fluid's colour over a fluid unless the block is sturdy
/// on top, and the fluid's depth.
fn scan_column(chunk: &Chunk, palette: &Palette, x: i32, z: i32, top: i32) -> (u8, i16, u8) {
    let mut y = top;
    while y > MIN_Y {
        let (mut color, flags) = palette.get(chunk.get(x, y, z));
        if color == 0 {
            y -= 1;
            continue;
        }
        let mut depth = 0u8;
        let fluid = flags & FLUID;
        if fluid != 0 {
            let mut below = y - 1;
            while below > MIN_Y && palette.get(chunk.get(x, below, z)).1 & FLUID != 0 {
                depth = depth.saturating_add(1);
                below -= 1;
            }
            depth = depth.saturating_add(1);
            if flags & STURDY_TOP == 0 {
                color = if fluid == 2 { LAVA } else { WATER };
            }
        }
        return (color, y as i16, depth);
    }
    (0, MIN_Y as i16, 0)
}

/// Scan one chunk's columns.
pub fn scan_chunk(chunk: &Chunk, palette: &Palette, air: u32) -> Columns {
    scan_chunk_from(chunk, palette, air, i32::MAX)
}
/// `scan_chunk` going down from the height `from` (a cave map).
pub fn scan_chunk_from(chunk: &Chunk, palette: &Palette, air: u32, from: i32) -> Columns {
    let top = top_of(chunk, air).min(from);
    let mut columns = Columns {
        color: [0; 256],
        height: [MIN_Y as i16; 256],
        depth: [0; 256],
    };
    for z in 0..16 {
        for x in 0..16 {
            let (c, h, d) = scan_column(chunk, palette, x, z, top);
            let i = (z * 16 + x) as usize;
            columns.color[i] = c;
            columns.height[i] = h;
            columns.depth[i] = d;
        }
    }
    columns
}
