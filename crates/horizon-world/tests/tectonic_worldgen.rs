//! Tectonic's world generation: the density functions of the mod's data, and
//! the vanilla ones they must leave alone.
//!
//! The tests that need the prepared game data or the port's data are skipped
//! when those are not there: the rest of the suite runs everywhere.
use horizon_core::{
    density::{DensitySampler, SamplerContext, graph::DensityCompiler},
    noise::DensityVolume,
};
use horizon_world::worldgen::density::DensityRegistry;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

/// The prepared game data (vanilla's world generation).
fn prepared() -> Option<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("../../.local/horizon-client/26.3"),
        manifest.join("../../../../.local/horizon-client/26.3"),
    ]
    .into_iter()
    .find(|root| root.join("worldgen/densities.json").exists())
}

/// The port's data (the mod's world generation).
fn port() -> Option<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("../../.local/home/ports/tectonic/mods/tectonic/carry/data"),
        manifest.join("../../../../.local/home/ports/tectonic/mods/tectonic/carry/data"),
    ]
    .into_iter()
    .find(|root| root.join("tectonic/worldgen/density_function").exists())
}

/// The bundle of a prepared root: id → the file's text.
fn bundle(root: &Path, name: &str) -> BTreeMap<String, String> {
    let text = fs::read_to_string(root.join("worldgen").join(format!("{name}.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The files of one worldgen registry of the port: id → the file's text.
fn mod_bundle(root: &Path, namespace: &str, kind: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let base = root.join(namespace).join("worldgen").join(kind);
    let mut stack = vec![base.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".json") {
                continue;
            }
            let relative = path
                .strip_prefix(root.join(namespace).join("worldgen").join(kind))
                .unwrap_or(&path);
            let id = format!(
                "{namespace}:{}",
                relative
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            out.insert(id, fs::read_to_string(&path).unwrap());
        }
    }
    out
}

/// The sampler of a named density function, for the seed the tests use.
fn sampler(registry: &DensityRegistry, name: &str) -> Arc<dyn DensitySampler> {
    let id = registry.value(name).expect("a density function");
    let mut compiler = DensityCompiler::new_seeded(registry.graph().clone(), 2, false);
    compiler.get_sampler(id).expect("a sampler")
}

/// The sampler's values at the points.
fn sample(sampler: &dyn DensitySampler, points: &[[i32; 3]]) -> Vec<f32> {
    let mut context = SamplerContext::default();
    points
        .iter()
        .map(|point| {
            sampler
                .sample_value(&mut context, *point)
                .expect("a sample")
        })
        .collect()
}

#[test]
fn a_config_constant_is_the_number_its_configuration_holds() {
    let densities = BTreeMap::from([
        (
            "minecraft:scale".to_owned(),
            r#"{"type":"tectonic:config_constant","key":"vertical_scale"}"#.to_owned(),
        ),
        (
            "minecraft:ocean".to_owned(),
            r#"{"type":"tectonic:config_constant","key":"ocean_depth"}"#.to_owned(),
        ),
        (
            "minecraft:unknown".to_owned(),
            r#"{"type":"tectonic:config_constant","key":"no_such_setting"}"#.to_owned(),
        ),
    ]);
    let registry = DensityRegistry::from_json(&BTreeMap::new(), &densities).unwrap();
    let values = sample(&*sampler(&registry, "minecraft:scale"), &[[0, 0, 0]]);
    assert_eq!(values[0], 1.125);
    let values = sample(&*sampler(&registry, "minecraft:ocean"), &[[0, 0, 0]]);
    assert_eq!(values[0], -0.22);
    // A name the configuration does not hold is zero, as in the mod.
    let values = sample(&*sampler(&registry, "minecraft:unknown"), &[[0, 0, 0]]);
    assert_eq!(values[0], 0.0);
}

#[test]
fn a_config_noise_samples_like_the_noise_it_scales() {
    let noises = BTreeMap::from([(
        "minecraft:test".to_owned(),
        r#"{"base_octave":-7,"octave_count":3}"#.to_owned(),
    )]);
    let densities = BTreeMap::from([
        (
            "minecraft:scaled".to_owned(),
            r#"{"type":"tectonic:config_noise","noise":"minecraft:test","key":"continents","shift_x":0,"shift_z":0}"#
                .to_owned(),
        ),
        (
            "minecraft:plain".to_owned(),
            r#"{"type":"minecraft:noise","noise":"minecraft:test","xz_scale":0.13,"y_scale":0}"#
                .to_owned(),
        ),
    ]);
    let registry = DensityRegistry::from_json(&noises, &densities).unwrap();
    let points = [
        [0, 0, 0],
        [17, 64, -33],
        [-120, -40, 900],
        [30_000, 12, -70_000],
    ];
    let scaled = sample(&*sampler(&registry, "minecraft:scaled"), &points);
    let plain = sample(&*sampler(&registry, "minecraft:plain"), &points);
    for (a, b) in scaled.iter().zip(plain.iter()) {
        // The mod scales the noise's coordinates in single precision, Horizon
        // in double: the two agree to within the noise's own precision.
        let difference = (a - b).abs();
        assert!(
            difference <= 1e-5 * b.abs().max(1.0),
            "{a} and {b} differ by {difference}"
        );
    }
    // A noise the configuration does not scale is refused.
    let densities = BTreeMap::from([(
        "minecraft:unknown".to_owned(),
        r#"{"type":"tectonic:config_noise","noise":"minecraft:test","key":"no_such_noise","shift_x":0,"shift_z":0}"#
            .to_owned(),
    )]);
    assert!(DensityRegistry::from_json(&noises, &densities).is_err());
}

#[test]
fn a_config_clamp_clamps_between_its_limits() {
    let densities = BTreeMap::from([
        (
            "minecraft:low".to_owned(),
            r#"{"type":"minecraft:constant","value":-1.0}"#.to_owned(),
        ),
        (
            "minecraft:high".to_owned(),
            r#"{"type":"minecraft:constant","value":1.0}"#.to_owned(),
        ),
        (
            "minecraft:clamped".to_owned(),
            r#"{"type":"tectonic:config_clamp","input":5.0,"min":"minecraft:low","max":"minecraft:high"}"#
                .to_owned(),
        ),
        (
            "minecraft:under".to_owned(),
            r#"{"type":"tectonic:config_clamp","input":-5.0,"min":"minecraft:low","max":"minecraft:high"}"#
                .to_owned(),
        ),
    ]);
    let registry = DensityRegistry::from_json(&BTreeMap::new(), &densities).unwrap();
    let values = sample(&*sampler(&registry, "minecraft:clamped"), &[[0, 0, 0]]);
    assert_eq!(values[0], 1.0);
    let values = sample(&*sampler(&registry, "minecraft:under"), &[[0, 0, 0]]);
    assert_eq!(values[0], -1.0);
}

#[test]
fn the_slope_condition_reads_its_range() {
    use horizon_world::worldgen::material_condition::MaterialCondition;
    let condition = MaterialCondition::from_json(
        r#"{"type":"lithostitched:slope","height_difference":{"min_inclusive":4}}"#,
    )
    .unwrap();
    assert!(matches!(
        condition,
        MaterialCondition::Slope {
            min: 4,
            max: i32::MAX
        }
    ));
    // The condition survives a round trip through its own encoding.
    let json = condition.to_json().unwrap();
    let again = MaterialCondition::from_json(&json).unwrap();
    assert_eq!(again.to_json().unwrap(), json);
    // A range that ends before it starts is refused.
    assert!(
        MaterialCondition::from_json(
            r#"{"type":"lithostitched:slope","height_difference":{"min_inclusive":8,"max_inclusive":4}}"#
        )
        .is_err()
    );
}

#[test]
fn the_vanilla_overworld_samples_the_same_as_before() {
    let Some(root) = prepared() else {
        return;
    };
    let registry =
        DensityRegistry::from_json(&bundle(&root, "noises"), &bundle(&root, "densities")).unwrap();
    let points = [[0, 64, 0], [128, 96, -256], [-1024, 32, 2048], [7, 200, 13]];
    // The values vanilla's own density functions give, unchanged by the mod's
    // own density functions being read.
    let expected: [(&str, [u32; 4]); 3] = [
        (
            "minecraft:overworld/final_density",
            [1026029387, 1032449883, 1019010796, 3203050155],
        ),
        (
            "minecraft:overworld/continents",
            [1051159668, 1054698743, 1033995804, 1051524912],
        ),
        (
            "minecraft:overworld/erosion",
            [3203756091, 3204294232, 1041333207, 3203624105],
        ),
    ];
    for (name, golden) in expected {
        let values = sample(&*sampler(&registry, name), &points);
        let bits: Vec<u32> = values.iter().map(|v| v.to_bits()).collect();
        assert_eq!(bits, golden.to_vec(), "{name}");
    }
}

#[test]
fn the_mods_data_decodes_and_moves_the_terrain() {
    let (Some(root), Some(port)) = (prepared(), port()) else {
        return;
    };
    let mut noises = bundle(&root, "noises");
    noises.extend(mod_bundle(&port, "tectonic", "noise"));
    let mut densities = bundle(&root, "densities");
    densities.extend(mod_bundle(&port, "tectonic", "density_function"));
    // The mod replaces the Overworld's own density functions.
    densities.extend(mod_bundle(&port, "minecraft", "density_function"));
    let registry = DensityRegistry::from_json(&noises, &densities).unwrap();
    let vanilla =
        DensityRegistry::from_json(&bundle(&root, "noises"), &bundle(&root, "densities")).unwrap();

    // The surface of every column of a grid over the same seed: the mod's
    // oceans are deeper and its mountains higher than vanilla's.
    let mut columns = Vec::new();
    for x in (0..12).map(|i| i * 431 - 2500) {
        for z in (0..12).map(|i| i * 617 - 2500) {
            columns.push([x, 0, z]);
        }
    }
    let surfaces = |registry: &DensityRegistry| {
        let id = registry.value("minecraft:overworld/final_density").unwrap();
        let mut compiler = DensityCompiler::new_seeded(registry.graph().clone(), 2, false);
        let sampler = compiler.get_sampler(id).unwrap();
        let mut context = SamplerContext::cached();
        let mut heights = Vec::new();
        for column in &columns {
            let volume =
                DensityVolume::new([1, 384, 1], [column[0], -64, column[2]], [1, 1, 1]).unwrap();
            let mut buffer = vec![0.0f32; 384];
            sampler
                .sample_volume(&mut context, &mut buffer, volume)
                .unwrap();
            let top = buffer
                .iter()
                .rposition(|value| *value > 0.0)
                .map(|offset| offset as i32 - 64)
                .unwrap_or(-64);
            heights.push(top);
        }
        heights
    };
    let mod_heights = surfaces(&registry);
    let vanilla_heights = surfaces(&vanilla);
    let lowest = |heights: &[i32]| heights.iter().copied().min().unwrap_or(-64);
    let highest = |heights: &[i32]| heights.iter().copied().max().unwrap_or(-64);
    let mean = |heights: &[i32]| {
        heights.iter().map(|h| *h as i64).sum::<i64>() as f64 / heights.len() as f64
    };
    println!(
        "vanilla: lowest {}, highest {}, mean {:.1}; the mod: lowest {}, highest {}, mean {:.1}",
        lowest(&vanilla_heights),
        highest(&vanilla_heights),
        mean(&vanilla_heights),
        lowest(&mod_heights),
        highest(&mod_heights),
        mean(&mod_heights),
    );
    assert!(
        lowest(&mod_heights) < lowest(&vanilla_heights),
        "the mod's deepest sea floor: {}, vanilla {}",
        lowest(&mod_heights),
        lowest(&vanilla_heights)
    );
    assert!(
        highest(&mod_heights) > highest(&vanilla_heights),
        "the mod's highest mountain: {}, vanilla {}",
        highest(&mod_heights),
        highest(&vanilla_heights)
    );
}
