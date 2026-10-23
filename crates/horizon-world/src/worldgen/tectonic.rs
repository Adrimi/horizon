//! Tectonic's configuration values.
//!
//! Tectonic's density functions ask its configuration for numbers by name:
//! a constant (`tectonic:config_constant`) and the scale of a noise
//! (`tectonic:config_noise`). The mod writes its configuration file on the
//! first start, so a world generated with the mod's own defaults uses the
//! values below. The file itself is not read here: the values are the mod's
//! defaults, and a world that changes them needs a mod that changes them.

/// A noise the configuration scales: the factor its coordinates get, the
/// factor its value gets and the value it is offset by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoiseState {
    /// The factor the noise's X and Z coordinates get.
    pub scale: f32,
    /// The factor the noise's value gets.
    pub multiplier: f32,
    /// The value the noise's result is offset by.
    pub offset: f32,
}

/// The configuration's number for `key`.
///
/// A key the configuration does not name is zero, as in the mod: the
/// configuration answers every name it is asked for, and only the ones it
/// lists have a value.
pub fn constant(key: &str) -> f32 {
    match key {
        // The terrain's height scale and the extra it gets from the height.
        "vertical_scale" => 1.125,
        "elevation_boost" => 0.0,
        // The world's build limits.
        "min_y" => -64.0,
        "max_y" => 320.0,
        // Whether the lava tunnels are carved at all.
        "lava_tunnels" => 1.0,
        // Where the ocean's floor sits below the sea level.
        "ocean_offset" => -0.8,
        // The erosion noise's scale, four times the continents' own.
        "alternate_erosion_scale" => 1.0,
        // Whether the underground rivers are carved.
        "underground_rivers" => -1.0,
        // How much the flat terrain leans.
        "flat_terrain_skew" => 0.1,
        // Whether the rolling hills and the jungle pillars are there.
        "rolling_hills" => 1.0,
        "jungle_pillars" => 1.0,
        // The oceans' two depths.
        "ocean_depth" => -0.22,
        "deep_ocean_depth" => -0.45,
        // Where the caves start and how deep they reach.
        "depth_cutoff_start" => 0.1,
        "depth_cutoff_size" => 0.1,
        // The cheese caves and their strength.
        "cheese_enabled" => 1.0,
        "cheese_additive" => 0.27,
        // The noodle caves and their strength.
        "noodle_enabled" => 1.0,
        "noodle_additive" => -0.075,
        _ => 0.0,
    }
}

/// The configuration's scale for the noise `key`.
///
/// A key the configuration does not name has no scale: the density function
/// that asks for it is refused, as in the mod.
pub fn noise_state(key: &str) -> Option<NoiseState> {
    let state = match key {
        // The continents' noise, scaled by the continents' own factor.
        "continents" => NoiseState {
            scale: 0.13,
            multiplier: 1.0,
            offset: 0.0,
        },
        // The islands' noise.
        "island" => NoiseState {
            scale: 0.11,
            multiplier: 1.0,
            offset: 0.0,
        },
        // The erosion's and the ridges' noises.
        "erosion" => NoiseState {
            scale: 0.25,
            multiplier: 1.0,
            offset: 0.0,
        },
        "ridge" => NoiseState {
            scale: 0.25,
            multiplier: 1.0,
            offset: 0.0,
        },
        // The climate's noises, which the biomes are placed by.
        "temperature" => NoiseState {
            scale: 0.25,
            multiplier: 1.0,
            offset: 0.0,
        },
        "vegetation" => NoiseState {
            scale: 0.25,
            multiplier: 1.0,
            offset: 0.0,
        },
        _ => return None,
    };
    Some(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_configuration_holds_the_mods_own_defaults() {
        assert_eq!(constant("vertical_scale"), 1.125);
        assert_eq!(constant("min_y"), -64.0);
        assert_eq!(constant("max_y"), 320.0);
        assert_eq!(constant("ocean_depth"), -0.22);
        assert_eq!(constant("deep_ocean_depth"), -0.45);
        assert_eq!(constant("underground_rivers"), -1.0);
        // A name the configuration does not list is zero, as in the mod.
        assert_eq!(constant("spaghetti_enabled"), 0.0);
        assert_eq!(constant("no_such_setting"), 0.0);
        // The noises the configuration scales.
        assert_eq!(
            noise_state("continents"),
            Some(NoiseState {
                scale: 0.13,
                multiplier: 1.0,
                offset: 0.0
            })
        );
        assert_eq!(
            noise_state("ridge"),
            Some(NoiseState {
                scale: 0.25,
                multiplier: 1.0,
                offset: 0.0
            })
        );
        assert_eq!(noise_state("no_such_noise"), None);
    }
}
