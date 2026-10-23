//! Dynamic lights, a Horizon extra after LambDynamicLights: a held torch,
//! a burning mob or a dropped lantern lights the blocks and entities around
//! it while it moves. Nothing is relit: the terrain and entity shaders raise
//! each vertex's block light to what the nearest sources give it
//! (`dynamic_block_light` in terrain.wgsl and entity.wgsl), one level less
//! per block of distance as block light spreads, before the lightmap.
//! Off (the default) leaves the list empty and the picture vanilla's.
use glam::DVec3;

/// Sources the shaders read (`Globals::dynamic_lights`).
pub const MAX_LIGHTS: usize = 16;
/// A burning entity, as fire's own emission.
pub const BURNING: u8 = 15;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Off,
    /// Sources gathered every `FAST_EVERY` frames.
    Fast,
    /// Every frame.
    Fancy,
}
/// Frames between two gatherings in `Mode::Fast`.
pub const FAST_EVERY: u64 = 4;
impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Off, Mode::Fast, Mode::Fancy];
    pub fn name(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Fast => "fast",
            Mode::Fancy => "fancy",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Mode::Off => "Off",
            Mode::Fast => "Fast",
            Mode::Fancy => "Fancy",
        }
    }
    pub fn parse(text: &str, default: Mode) -> Mode {
        Mode::ALL
            .into_iter()
            .find(|m| m.name() == text)
            .unwrap_or(default)
    }
    /// Whether the sources are gathered again on frame `frame`.
    pub fn refresh(self, frame: u64) -> bool {
        match self {
            Mode::Off => false,
            Mode::Fast => frame.is_multiple_of(FAST_EVERY),
            Mode::Fancy => true,
        }
    }
}

/// A light something gives off this frame, in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub pos: DVec3,
    pub level: u8,
}

/// The light of an item that places no glowing block (a block item gives
/// its block's emission instead).
pub fn item_level(id: &str) -> u8 {
    match id.strip_prefix("minecraft:").unwrap_or(id) {
        "lava_bucket" => 15,
        "glow_berries" => 14,
        "blaze_rod" | "fire_charge" | "glow_ink_sac" | "nether_star" => 10,
        "blaze_powder" | "glowstone_dust" | "magma_cream" | "spectral_arrow" => 8,
        "prismarine_crystals" | "experience_bottle" => 6,
        _ => 0,
    }
}

/// The light a mob gives off by its kind, burning or not.
pub fn entity_level(id: &str) -> u8 {
    match id.strip_prefix("minecraft:").unwrap_or(id) {
        "blaze" => 10,
        "magma_cube" | "glow_squid" => 8,
        "allay" | "vex" => 6,
        _ => 0,
    }
}

/// The block light a source of `level` gives `distance` blocks away.
pub fn level_at(level: f32, distance: f32) -> f32 {
    (level - distance).max(0.0)
}

/// The `MAX_LIGHTS` sources nearest the camera that can light something
/// within `reach` blocks of it, camera relative and packed as the shaders
/// read them: xyz, then the level.
pub fn pack(sources: &[Source], camera: DVec3, reach: f64) -> Vec<[f32; 4]> {
    let mut lit: Vec<(f64, &Source)> = sources
        .iter()
        .filter(|s| s.level > 0)
        .map(|s| (s.pos.distance(camera), s))
        .filter(|(d, s)| *d - f64::from(s.level) <= reach)
        .collect();
    lit.sort_by(|a, b| a.0.total_cmp(&b.0));
    lit.into_iter()
        .take(MAX_LIGHTS)
        .map(|(_, s)| {
            let p = s.pos - camera;
            [p.x as f32, p.y as f32, p.z as f32, f32::from(s.level)]
        })
        .collect()
}

/// What the shaders do with the packed list for a vertex at `pos` (camera
/// relative) whose own block light is `block`.
pub fn block_light(packed: &[[f32; 4]], pos: [f32; 3], block: f32) -> f32 {
    packed.iter().fold(block, |light, s| {
        let d =
            ((pos[0] - s[0]).powi(2) + (pos[1] - s[1]).powi(2) + (pos[2] - s[2]).powi(2)).sqrt();
        light.max(level_at(s[3], d))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(x: f64, level: u8) -> Source {
        Source {
            pos: DVec3::new(x, 64.0, 0.0),
            level,
        }
    }

    #[test]
    fn light_falls_one_level_a_block() {
        assert_eq!(level_at(14.0, 0.0), 14.0);
        assert_eq!(level_at(14.0, 3.0), 11.0);
        assert_eq!(level_at(14.0, 14.0), 0.0);
        assert_eq!(level_at(14.0, 20.0), 0.0);
    }

    #[test]
    fn the_nearest_sources_are_kept_camera_relative() {
        let camera = DVec3::new(0.5, 64.0, 0.0);
        let mut sources: Vec<_> = (0..40).map(|i| source(f64::from(i) * 2.0, 14)).collect();
        sources.push(source(1.0, 0));
        let packed = pack(&sources, camera, 512.0);
        assert_eq!(packed.len(), MAX_LIGHTS);
        assert_eq!(packed[0], [-0.5, 0.0, 0.0, 14.0]);
        assert!(packed.windows(2).all(|w| w[0][0].abs() <= w[1][0].abs()));
        assert!(packed.iter().all(|s| s[3] > 0.0));
    }

    #[test]
    fn sources_too_far_to_reach_anything_are_dropped() {
        let camera = DVec3::new(0.0, 64.0, 0.0);
        let packed = pack(&[source(100.0, 15), source(30.0, 15)], camera, 20.0);
        assert_eq!(packed, vec![[30.0, 0.0, 0.0, 15.0]]);
    }

    #[test]
    fn a_vertex_keeps_the_brighter_of_its_own_and_the_dynamic_light() {
        let packed = [[0.0, 0.0, 0.0, 14.0], [10.0, 0.0, 0.0, 8.0]];
        assert_eq!(block_light(&packed, [2.0, 0.0, 0.0], 3.0), 12.0);
        assert_eq!(block_light(&packed, [2.0, 0.0, 0.0], 13.0), 13.0);
        assert_eq!(block_light(&packed, [40.0, 0.0, 0.0], 3.0), 3.0);
        assert_eq!(block_light(&[], [0.0, 0.0, 0.0], 5.0), 5.0);
    }

    #[test]
    fn items_and_mobs_give_their_light() {
        assert_eq!(item_level("minecraft:lava_bucket"), 15);
        assert_eq!(item_level("minecraft:blaze_rod"), 10);
        assert_eq!(item_level("minecraft:stick"), 0);
        assert_eq!(entity_level("minecraft:blaze"), 10);
        assert_eq!(entity_level("minecraft:zombie"), 0);
    }

    #[test]
    fn the_extra_is_off_by_default_and_reads_its_mode() {
        use crate::extras::Extras;
        assert_eq!(Extras::default().dynamic_lights, Mode::Off);
        let e = Extras::parse("dynamicLights:fancy\n");
        assert_eq!(e.dynamic_lights, Mode::Fancy);
        assert!(e.serialize().contains("dynamicLights:fancy"));
        let e = Extras::parse("dynamicLights:nonsense\n");
        assert_eq!(e.dynamic_lights, Mode::Off);
    }

    #[test]
    fn modes_parse_and_refresh() {
        assert_eq!(Mode::parse("fancy", Mode::Off), Mode::Fancy);
        assert_eq!(Mode::parse("nonsense", Mode::Fast), Mode::Fast);
        assert!(!Mode::Off.refresh(0));
        assert!(Mode::Fancy.refresh(7));
        assert!(Mode::Fast.refresh(8) && !Mode::Fast.refresh(9));
    }
}
