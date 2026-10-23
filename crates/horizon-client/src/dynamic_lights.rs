//! Dynamic lights: the light sources the mods give the world, packed for
//! the shaders (`Globals::dynamic_lights` in `gfx.rs`). A mod lists its
//! sources with `lighting.set-lights` (API 0.1.13); the game keeps the
//! ones nearest the camera and the shaders raise the block light of what
//! they reach, one level less per block of distance as block light
//! spreads, before the lightmap. Nothing is relit. With no mod on, the
//! list is empty and the picture is vanilla's.
use glam::DVec3;

/// Sources the shaders read (`Globals::dynamic_lights`).
pub const MAX_LIGHTS: usize = 16;

/// A light something gives off, in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub pos: DVec3,
    pub level: u8,
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
}
