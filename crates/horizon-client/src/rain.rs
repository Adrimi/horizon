//! Horizon's particle rain: rain drops, snowflakes and sandstorms as
//! particles, the splashes and ripples they leave, and the storm's haze.
//!
//! Vanilla draws the weather as camera-facing columns of texture
//! (`weather.rs`). Horizon's extra puts real particles on top of them:
//! drops that fall and splash where they land, flakes that drift down with
//! the wind, and the sandstorms that blow in the deserts where vanilla has
//! no rain at all.
//!
//! Everything here is the client alone: the server never hears of it.
use crate::{
    biome::{BiomeInfo, Biomes},
    blocks::{BlockFacts, Fluid},
    particles::ParticleEngine,
    world::World,
};
use horizon_core::random::{LegacyRandom, RandomSource};

/// How far around the player the weather shows (blocks).
pub const RADIUS: i32 = 14;
/// How far above and below the player the rain falls (blocks).
pub const SPAN: i32 = 14;
/// The particles a tick gets at full intensity and `Amount` 1.
const RAIN_PER_TICK: f32 = 20.0;
const SNOW_PER_TICK: f32 = 6.0;
const SAND_PER_TICK: f32 = 12.0;
/// Never more particles than this in a tick, however high the `Amount`.
const MAX_PER_TICK: usize = 256;
/// The colour the haze of a heavy rain puts into the distance.
const RAIN_HAZE: [f32; 3] = [0.62, 0.66, 0.72];
/// The colour the haze of a sandstorm puts into the distance.
const SAND_HAZE: [f32; 3] = [0.85, 0.78, 0.58];
/// The colour of sand, for the ground the map colour does not name.
const SAND_COLOR: u32 = 0xDBD3A0;

/// What falls where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    /// Drops of rain.
    Rain,
    /// Snowflakes.
    Snow,
    /// The sand a desert storm blows.
    Sand,
}

/// The Particles option (0 all, 1 decreased, 2 minimal) as a factor of the
/// density: none at all on Minimal.
pub fn particle_factor(particles: u8) -> f32 {
    match particles {
        0 => 1.0,
        1 => 1.0 / 3.0,
        _ => 0.0,
    }
}

/// A biome where vanilla has no rain and a sandstorm blows instead: the
/// deserts and the badlands.
pub fn is_dry(name: &str) -> bool {
    name.contains("desert") || name.contains("badlands")
}

/// The weather of a biome at a height: sand in a desert, snow where it is
/// cold enough to snow, rain where it is warm, and nothing where the biome
/// has no precipitation at all.
pub fn weather_of(biome: &BiomeInfo, y: i32) -> Option<Weather> {
    if is_dry(&biome.name) {
        return Some(Weather::Sand);
    }
    if !biome.precipitation {
        return None;
    }
    // Biome.getTemperature: colder the higher it is.
    let t = if y > 80 {
        biome.temperature - (y - 80) as f32 * 0.05 / 40.0
    } else {
        biome.temperature
    };
    Some(if t < 0.15 {
        Weather::Snow
    } else {
        Weather::Rain
    })
}

/// The intensity of the weather: a thunderstorm rains harder.
pub fn intensity(rain: f32, thunder: f32) -> f32 {
    rain + thunder * 0.5
}

/// How dense the weather's particles are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Density {
    /// The `Amount` option: times Horizon's own density.
    pub amount: u8,
    /// The Particles option (0 all, 1 decreased, 2 minimal).
    pub particles: u8,
}

/// How many particles of a kind a tick gets.
pub fn count(kind: Weather, rain: f32, thunder: f32, density: Density) -> usize {
    let base = match kind {
        Weather::Rain => RAIN_PER_TICK,
        Weather::Snow => SNOW_PER_TICK,
        Weather::Sand => SAND_PER_TICK,
    };
    let n = base
        * intensity(rain, thunder)
        * f32::from(density.amount)
        * particle_factor(density.particles);
    (n as usize).min(MAX_PER_TICK)
}

/// The wind of the storm: it turns slowly and blows in a direction.
pub fn wind(ticks: i64) -> [f64; 3] {
    let t = f64::from(ticks as i32 % 12_000) * 0.001;
    [t.sin() * 0.9, 0.0, (t * 0.7 + 1.0).cos() * 0.9]
}

/// Everything the spawner and the haze read about the world.
pub struct Spawn<'a> {
    pub world: &'a World,
    pub facts: &'a BlockFacts,
    pub biomes: &'a Biomes,
    /// The player's eye.
    pub eye: [f64; 3],
    /// The rain level (0..1).
    pub rain: f32,
    /// The thunder level (0..1, already scaled by the rain).
    pub thunder: f32,
}
impl Spawn<'_> {
    /// The weather of a column.
    pub fn weather(&self, x: i32, y: i32, z: i32) -> Option<Weather> {
        let chunk = self.world.chunks.get(&(x >> 4, z >> 4))?;
        weather_of(self.biomes.get(chunk.biome(x, y, z)), y)
    }
    /// The first block from `from` downwards that stops the rain
    /// (`Heightmap.MOTION_BLOCKING`), plus one: the air above the surface.
    pub fn surface(&self, x: i32, z: i32, from: i32) -> i32 {
        let mut y = from.min(crate::chunk::HEIGHT - 1);
        while y > crate::chunk::MIN_Y {
            let f = self.facts.get(self.world.block(x, y, z));
            if f.full_collision || f.fluid != Fluid::None || f.name.contains("leaves") {
                return y + 1;
            }
            y -= 1;
        }
        crate::chunk::MIN_Y
    }
}

/// The colour of the sand blocks a desert storm blows over.
fn sand_color(name: &str) -> Option<u32> {
    Some(match name {
        "minecraft:sand" => 0xDBD3A0,
        "minecraft:red_sand" => 0xBF6B24,
        "minecraft:sandstone" => 0xDCD3A0,
        "minecraft:cut_sandstone" | "minecraft:chiseled_sandstone" => 0xDCD3A0,
        "minecraft:red_sandstone" | "minecraft:cut_red_sandstone" => 0xC06A2C,
        "minecraft:chiseled_red_sandstone" => 0xC06A2C,
        "minecraft:terracotta" => 0x985E44,
        "minecraft:white_terracotta" => 0xD1B1A1,
        "minecraft:orange_terracotta" => 0xA05325,
        "minecraft:brown_terracotta" => 0x4C3223,
        "minecraft:yellow_terracotta" => 0xD8A13A,
        "minecraft:red_terracotta" => 0x8F4C38,
        "minecraft:gravel" => 0x8C8C8C,
        "minecraft:clay" => 0xA0A7B4,
        "minecraft:dirt" | "minecraft:coarse_dirt" | "minecraft:rooted_dirt" => 0x8A6243,
        "minecraft:podzol" => 0x6B4E2E,
        _ => return None,
    })
}

/// The colour of the ground under a column: the grains of a sandstorm take
/// the colour of the block they blow over.
pub fn ground_color(s: &Spawn, x: i32, z: i32) -> [f32; 3] {
    let below = s.surface(x, z, s.eye[1].floor() as i32) - 1;
    let f = s.facts.get(s.world.block(x, below, z));
    let c = sand_color(&f.name).unwrap_or_else(|| {
        crate::blocks::MAP_COLORS
            .get(f.map_color as usize)
            .copied()
            .unwrap_or(SAND_COLOR)
    });
    [
        (c >> 16 & 255) as f32 / 255.0,
        (c >> 8 & 255) as f32 / 255.0,
        (c & 255) as f32 / 255.0,
    ]
}

/// The storm's haze: what the fog takes of the weather.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Haze {
    /// How much of the fog the storm takes (0..1).
    pub amount: f32,
    /// The colour the fog takes.
    pub color: [f32; 3],
}

/// The fog's colour with the storm's haze in it. The fog has four channels
/// and the storm's colour three, so the fog's alpha is left alone.
pub fn mix_fog(fog: [f32; 4], haze: [f32; 3], amount: f32) -> [f32; 4] {
    let mut out = fog;
    for (i, c) in haze.iter().enumerate() {
        out[i] = fog[i] + (c - fog[i]) * amount;
    }
    out
}

/// The haze of the storm over the player: sand-coloured in a sandstorm, a
/// grey drizzle in a heavy rain or a thunderstorm.
pub fn haze(s: &Spawn) -> Option<Haze> {
    if s.rain <= 0.0 {
        return None;
    }
    let cx = s.eye[0].floor() as i32;
    let cy = s.eye[1].floor() as i32;
    let cz = s.eye[2].floor() as i32;
    match s.weather(cx, cy, cz) {
        Some(Weather::Sand) => Some(Haze {
            amount: (s.rain * 0.9).min(1.0),
            color: SAND_HAZE,
        }),
        _ => {
            let amount = ((s.rain - 0.4) / 0.6).clamp(0.0, 1.0) * 0.4 + s.thunder * 0.2;
            (amount > 0.0).then_some(Haze {
                amount: amount.min(1.0),
                color: RAIN_HAZE,
            })
        }
    }
}

/// Spawns the weather's particles for one tick.
pub fn tick(
    engine: &mut ParticleEngine,
    s: &Spawn,
    density: Density,
    ticks: i64,
    random: &mut LegacyRandom,
) {
    if s.rain <= 0.0 {
        return;
    }
    let cx = s.eye[0].floor() as i32;
    let cy = s.eye[1].floor() as i32;
    let cz = s.eye[2].floor() as i32;
    let Some(kind) = s.weather(cx, cy, cz) else {
        return;
    };
    let count = count(kind, s.rain, s.thunder, density);
    if count == 0 {
        return;
    }
    let wind = wind(ticks);
    match kind {
        // The sandstorm blows around the player, on the surface only.
        Weather::Sand => {
            if s.surface(cx, cz, crate::chunk::HEIGHT - 1) > cy + 2 {
                return;
            }
            for _ in 0..count {
                let x = cx + random.next_int_bound(2 * RADIUS + 1).unwrap_or(0) - RADIUS;
                let z = cz + random.next_int_bound(2 * RADIUS + 1).unwrap_or(0) - RADIUS;
                let y = s.eye[1] + (random.next_double() - 0.5) * 8.0;
                let color = ground_color(s, x, z);
                engine.add_sand_grain(
                    [
                        f64::from(x) + random.next_double(),
                        y,
                        f64::from(z) + random.next_double(),
                    ],
                    [wind[0] * 0.6, 0.0, wind[2] * 0.6],
                    color,
                );
            }
        }
        Weather::Rain | Weather::Snow => {
            for _ in 0..count {
                let x = cx + random.next_int_bound(2 * RADIUS + 1).unwrap_or(0) - RADIUS;
                let z = cz + random.next_int_bound(2 * RADIUS + 1).unwrap_or(0) - RADIUS;
                // Only where it rains in this column.
                let Some(column) = s.weather(x, cy, z) else {
                    continue;
                };
                // The rain falls from the sky onto the column's surface: the
                // same column vanilla draws its rain in.
                let top = s.surface(x, z, cy + SPAN + 10);
                let low = top.max(cy - SPAN) as f64;
                let high = top.max(cy + SPAN) as f64;
                if high <= low {
                    continue;
                }
                let y = low + random.next_double() * (high - low);
                let p = [
                    f64::from(x) + random.next_double(),
                    y,
                    f64::from(z) + random.next_double(),
                ];
                match column {
                    Weather::Rain => engine.add_rain_drop(
                        p,
                        [
                            wind[0] * 0.05,
                            -0.45 - random.next_double() * 0.1,
                            wind[2] * 0.05,
                        ],
                    ),
                    Weather::Snow => {
                        engine.add_snow_flake(p, [wind[0] * 0.12, -0.05, wind[2] * 0.12])
                    }
                    // The sandstorm is the player's own column's business.
                    Weather::Sand => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biome::BiomeInfo;
    use std::collections::HashMap;

    fn biome(name: &str, temperature: f32, precipitation: bool) -> BiomeInfo {
        BiomeInfo {
            spawns: HashMap::new(),
            spawn_costs: HashMap::new(),
            creature_probability: 0.1,
            name: name.to_owned(),
            temperature,
            downfall: 0.5,
            precipitation,
            grass: 0x79C05A,
            modifier: crate::biome::GrassModifier::None,
            foliage: 0x48B518,
            dry_foliage: 0x5D4930,
            water: 0x3F76E4,
            sky: None,
            fog: None,
            water_fog: 0x050533,
        }
    }

    #[test]
    fn a_desert_has_a_sandstorm_where_vanilla_has_no_rain() {
        let desert = biome("minecraft:desert", 2.0, false);
        assert_eq!(weather_of(&desert, 64), Some(Weather::Sand));
        let badlands = biome("minecraft:wooded_badlands", 2.0, false);
        assert_eq!(weather_of(&badlands, 64), Some(Weather::Sand));
        // A hot savanna still rains in vanilla.
        let savanna = biome("minecraft:savanna", 1.2, true);
        assert_eq!(weather_of(&savanna, 64), Some(Weather::Rain));
    }

    #[test]
    fn a_cold_biome_has_snow_and_a_warm_one_rain() {
        let snowy = biome("minecraft:snowy_plains", -0.5, true);
        assert_eq!(weather_of(&snowy, 64), Some(Weather::Snow));
        let plains = biome("minecraft:plains", 0.8, true);
        assert_eq!(weather_of(&plains, 64), Some(Weather::Rain));
        // High up it is colder, so it snows there even in a cool biome.
        let cool = biome("minecraft:meadow", 0.2, true);
        assert_eq!(weather_of(&cool, 64), Some(Weather::Rain));
        assert_eq!(weather_of(&cool, 300), Some(Weather::Snow));
        // A biome without precipitation has no weather at all.
        let swamp = biome("minecraft:swamp", 0.8, false);
        assert_eq!(weather_of(&swamp, 64), None);
    }

    #[test]
    fn the_density_follows_the_weather_the_amount_and_the_particles_setting() {
        let all = Density {
            amount: 1,
            particles: 0,
        };
        let decreased = Density {
            amount: 1,
            particles: 1,
        };
        let minimal = Density {
            amount: 1,
            particles: 2,
        };
        // No rain, no particles.
        assert_eq!(count(Weather::Rain, 0.0, 0.0, all), 0);
        // A clear rain gets particles, a thunderstorm more of them.
        let rain = count(Weather::Rain, 1.0, 0.0, all);
        let storm = count(Weather::Rain, 1.0, 1.0, all);
        assert!(rain > 0);
        assert!(storm > rain);
        // The amount multiplies them.
        assert_eq!(
            count(
                Weather::Rain,
                1.0,
                0.0,
                Density {
                    amount: 4,
                    particles: 0
                }
            ),
            rain * 4
        );
        // Fewer with the particles setting, none on Minimal.
        assert_eq!(count(Weather::Rain, 1.0, 0.0, decreased), rain / 3);
        assert_eq!(count(Weather::Rain, 1.0, 0.0, minimal), 0);
        // Snow and sand are sparser than rain.
        assert!(count(Weather::Snow, 1.0, 0.0, all) < rain);
        assert!(count(Weather::Sand, 1.0, 0.0, all) < rain);
        // Never more than the cap, however high the amount.
        assert!(
            count(
                Weather::Rain,
                1.0,
                1.0,
                Density {
                    amount: 255,
                    particles: 0
                }
            ) <= MAX_PER_TICK
        );
    }

    #[test]
    fn the_wind_turns_slowly() {
        let a = wind(0);
        let b = wind(1000);
        assert_ne!(a[0], b[0]);
        assert_eq!(a[1], 0.0);
        // The wind never blows hard enough to throw things away.
        assert!(a[0].abs() <= 1.0 && a[2].abs() <= 1.0);
    }

    #[test]
    fn the_sand_colours_are_the_ones_of_the_ground() {
        assert_eq!(sand_color("minecraft:sand"), Some(0xDBD3A0));
        assert_eq!(sand_color("minecraft:red_sand"), Some(0xBF6B24));
        assert_eq!(sand_color("minecraft:stone"), None);
    }

    #[test]
    fn the_haze_mixes_into_the_fog_colour() {
        let fog = [0.5, 0.6, 0.7, 1.0];
        // No haze, no change.
        assert_eq!(mix_fog(fog, SAND_HAZE, 0.0), fog);
        // The storm's colour takes the fog's place.
        let mixed = mix_fog(fog, SAND_HAZE, 1.0);
        assert_eq!(&mixed[..3], &SAND_HAZE);
        // The fog's alpha is left alone.
        assert_eq!(mixed[3], 1.0);
    }
}
