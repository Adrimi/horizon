//! Prints the terrain height and the biome of every column of a grid, for a
//! world generated with the given content mods. An audit tool: it compares
//! Horizon's world generation with the original mod's.
//!
//! worldgen_columns <resources dir> [--mod <folder>]... [--seed S]
//!     [--center X Z] [--size N] [--cache DIR] [--out FILE]
//!     [--world-type ID]
//!
//! The grid is `size` chunks wide and deep, centred on chunk `--center`.
//! Every line is `x z height biome`, with the height the Y of the highest
//! non-air terrain block of the column (features are not placed).
//! A worldgen mod's world type (`--world-type`) is the root its world
//! generation is read from.
#![forbid(unsafe_code)]
use horizon_client::mod_content::{self, ContentMod};
use horizon_core::density::{DensitySampler, SamplerContext, graph::DensityCompiler};
use horizon_core::position::ChunkPos;
use horizon_mods::{Content, Package};
use horizon_world::{
    DATA_VERSION,
    nbt::Tag,
    registry::BlockRegistry,
    worldgen::{
        density::DensityRegistry,
        generator::{GeneratorData, OverworldGenerator},
    },
};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

type Error = Box<dyn std::error::Error>;

fn tag(v: &serde_json::Value) -> Tag {
    use serde_json::Value;
    match v {
        Value::String(s) => Tag::String(s.as_str().into()),
        Value::Number(n) => Tag::Int(n.as_i64().unwrap() as i32),
        Value::Array(a) => Tag::List(a.iter().map(tag).collect()),
        Value::Object(o) => {
            Tag::Compound(o.iter().map(|(k, v)| (k.as_str().into(), tag(v))).collect())
        }
        _ => panic!("invalid registry metadata"),
    }
}

/// The block registry the generator needs, from the prepared `facts.json`.
fn registry(resources: &std::path::Path) -> Result<&'static BlockRegistry, Error> {
    let facts: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(resources.join("facts.json"))?)?;
    let registry = BlockRegistry::from_nbt(&Tag::compound([
        ("DataVersion", Tag::Int(DATA_VERSION)),
        (
            "states",
            Tag::List(
                facts["states"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| tag(&v[4]))
                    .collect(),
            ),
        ),
        ("defaults", tag(&facts["defaults"])),
    ]))?;
    Ok(Box::leak(Box::new(registry)))
}

/// `BlockPos.spiralAround(ZERO, radius, EAST, SOUTH)`: the offsets, in order.
fn spiral(radius: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    let (mut x, mut z) = (0, 0);
    for r in 0..=radius {
        x = r;
        z = 0;
        out.push((x, z));
        for _ in 0..r {
            z += 1;
            out.push((x, z));
        }
        for _ in 0..r {
            x += 1;
            out.push((x, z));
        }
        for _ in 0..r {
            z -= 1;
            out.push((x, z));
        }
        for _ in 0..r {
            x -= 1;
            out.push((x, z));
        }
    }
    out
}

fn main() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    let base = PathBuf::from(args.next().expect("client resource directory"));
    let (mut seed, mut size, mut center) = (2i64, 4i32, (0, 0));
    let mut mods: Vec<PathBuf> = Vec::new();
    let mut cache = PathBuf::from("/tmp/horizon-worldgen-columns");
    let mut out: Option<PathBuf> = None;
    let mut world_type: Option<String> = None;
    let mut samples: Vec<(String, [i32; 3])> = Vec::new();
    let mut injected: Vec<(String, String)> = Vec::new();
    let mut locate: Vec<(String, f64, f64)> = Vec::new();
    let mut injected_noises: Vec<(String, String)> = Vec::new();
    while let Some(arg) = args.next() {
        let mut next = || args.next().expect("argument value");
        match arg.as_str() {
            "--seed" => seed = next().parse()?,
            "--size" => size = next().parse()?,
            "--center" => center = (next().parse()?, next().parse()?),
            "--mod" => mods.push(PathBuf::from(next())),
            "--cache" => cache = PathBuf::from(next()),
            "--out" => out = Some(PathBuf::from(next())),
            "--world-type" => world_type = Some(next()),
            "--sample" => {
                let spec = next();
                let (id, point) = spec.rsplit_once(':').expect("id:x,y,z");
                let mut parts = point.split(',');
                let x = parts.next().expect("x").parse()?;
                let y = parts.next().expect("y").parse()?;
                let z = parts.next().expect("z").parse()?;
                samples.push((id.to_owned(), [x, y, z]));
            }
            "--inject-noise" => {
                let spec = next();
                let (id, json) = spec.split_once('=').expect("id=json");
                injected_noises.push((id.to_owned(), json.to_owned()));
            }
            "--inject" => {
                let spec = next();
                let (id, json) = spec.split_once('=').expect("id=json");
                injected.push((id.to_owned(), json.to_owned()));
            }
            "--locate" => {
                let spec = next();
                let mut parts = spec.rsplitn(3, ':');
                let max: f64 = parts.next().expect("max").parse()?;
                let min: f64 = parts.next().expect("min").parse()?;
                let id = parts.next().expect("id").to_owned();
                locate.push((id, min, max));
            }
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    // The content mods, as the game reads them: a folder with a manifest.
    let mut content = Vec::new();
    for path in &mods {
        let package = Package::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let read = Content::read(path, &package.manifest)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        content.push(ContentMod {
            id: package.manifest.id.clone(),
            content: read,
        });
    }
    // The world's resources: the prepared game data with the mods' data on top.
    let root = if content.is_empty() {
        base.clone()
    } else {
        let built = cache.join("mod-content");
        let _ = fs::remove_dir_all(&built);
        mod_content::build(&base, &content, &built).map_err(|e| -> Error { e.into() })?;
        built
    };
    let registry = registry(&root)?;
    // A worldgen mod's world type: its own root and world preset.
    let (root, preset) = match &world_type {
        Some(id) => {
            mod_content::init_world_types(&root);
            let kind = mod_content::world_type(id).ok_or_else(|| format!("no world type {id}"))?;
            (PathBuf::from(&kind.dir), kind.preset.clone())
        }
        None => (root.clone(), "minecraft:normal".to_owned()),
    };
    let data = GeneratorData::load(&root.join("worldgen"), registry)?;
    // The nearest position a set of density function ranges holds at, the way
    // Tectonic's own `locate` command searches for it.
    if !locate.is_empty() {
        let bundle = |name: &str| -> BTreeMap<String, String> {
            let text = fs::read_to_string(root.join("worldgen").join(format!("{name}.json")))
                .unwrap_or_else(|e| panic!("{name}.json: {e}"));
            serde_json::from_str(&text).expect("a bundle")
        };
        let config = fs::read_to_string(root.join("worldgen/config.json")).unwrap_or_default();
        let mut noises = bundle("noises");
        for (id, json) in &injected_noises {
            noises.insert(id.clone(), json.clone());
        }
        let registry = DensityRegistry::from_json_with_config(
            &noises,
            &bundle("densities"),
            &horizon_world::worldgen::tectonic::WorldgenConfig::from_json(&config)
                .unwrap_or_default(),
        )?;
        let mut compiler = DensityCompiler::new_seeded(registry.graph().clone(), seed, false);
        let samplers = locate
            .iter()
            .map(|(id, _, _)| {
                let value = registry
                    .value(id)
                    .ok_or_else(|| format!("no {id}"))
                    .map_err(|e| -> Error { e.into() })?;
                compiler
                    .get_sampler(value)
                    .map(|s| s as Arc<dyn DensitySampler>)
                    .map_err(|e| -> Error { e.to_string().into() })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut context = SamplerContext::cached();
        // `BlockPos.spiralAround(ZERO, 400, EAST, SOUTH)`, scaled by 64.
        for (ox, oz) in spiral(400) {
            let x = ox * 64;
            let z = oz * 64;
            let mut all = true;
            for ((_, min, max), sampler) in locate.iter().zip(samplers.iter()) {
                let value = sampler.sample_value(&mut context, [x, 0, z])? as f64;
                if !(value >= *min && value <= *max) {
                    all = false;
                    break;
                }
            }
            if all {
                println!("locate {x} {z}");
                return Ok(());
            }
        }
        println!("locate: nothing within 25000 blocks");
        return Ok(());
    }
    // Samples of named density functions, for comparing with the original.
    if !samples.is_empty() {
        let bundle = |name: &str| -> BTreeMap<String, String> {
            let text = fs::read_to_string(root.join("worldgen").join(format!("{name}.json")))
                .unwrap_or_else(|e| panic!("{name}.json: {e}"));
            serde_json::from_str(&text).expect("a bundle")
        };
        let mut densities = bundle("densities");
        for (id, json) in &injected {
            densities.insert(id.clone(), json.clone());
        }
        let config = fs::read_to_string(root.join("worldgen/config.json")).unwrap_or_default();
        let mut noises = bundle("noises");
        for (id, json) in &injected_noises {
            noises.insert(id.clone(), json.clone());
        }
        let registry = DensityRegistry::from_json_with_config(
            &noises,
            &densities,
            &horizon_world::worldgen::tectonic::WorldgenConfig::from_json(&config)
                .unwrap_or_default(),
        )?;
        let mut compiler = DensityCompiler::new_seeded(registry.graph().clone(), seed, false);
        for (id, point) in &samples {
            let Some(value) = registry.value(id) else {
                println!("{id} at {point:?}: no such density function");
                continue;
            };
            let sampler = compiler.get_sampler(value)?;
            let mut context = SamplerContext::cached();
            let value = sampler.sample_value(&mut context, *point)?;
            println!("{id} at {point:?}: {:.6}", value);
        }
        return Ok(());
    }
    let generator = OverworldGenerator::for_dimension(data, seed, &preset, "minecraft:overworld")?;
    for (feature, error) in generator.unsupported_features() {
        eprintln!("unsupported {feature}: {error}");
    }
    let mut worker = generator.worker();
    let (x0, z0) = (center.0 - size / 2, center.1 - size / 2);
    let mut lines = String::new();
    for cz in z0..z0 + size {
        for cx in x0..x0 + size {
            let chunk = worker.generate_proto(ChunkPos::new(cx, cz))?;
            let heightmap = chunk
                .terrain
                .heightmap(horizon_world::heightmap::HeightmapType::WorldSurfaceWg)
                .expect("the worldgen surface heightmap");
            for lz in 0..16 {
                for lx in 0..16 {
                    let x = cx * 16 + lx;
                    let z = cz * 16 + lz;
                    let height = heightmap.highest_taken(lx, lz).unwrap_or(-64);
                    let biome = chunk.biome_at(x, height.max(-64), z).to_owned();
                    lines.push_str(&format!("{x} {z} {height} {biome}\n"));
                }
            }
        }
    }
    match out {
        Some(path) => {
            fs::write(&path, &lines)?;
            eprintln!(
                "wrote {} lines to {}",
                lines.lines().count(),
                path.display()
            );
        }
        None => print!("{lines}"),
    }
    Ok(())
}
