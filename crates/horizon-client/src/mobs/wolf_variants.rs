//! Wolf variants read from data (`data/<ns>/wolf_variant/<name>.json`): the
//! spawn conditions of a mod that replaces vanilla's (Terralith), by biome
//! or biome tag and priority. Without such files vanilla's rules in
//! `tame::wolf_variant_for` stand.
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
    sync::OnceLock,
};

struct Rule {
    variant: u8,
    priority: i64,
    biomes: Vec<String>,
}

#[derive(Default)]
struct WolfVariants {
    rules: Vec<Rule>,
    tags: HashMap<String, HashSet<String>>,
}

static VARIANTS: OnceLock<WolfVariants> = OnceLock::new();

/// Reads the root's `wolf-variants.json` and biome tags, once at start.
pub fn init(root: &Path) {
    let _ = VARIANTS.set(load(root).unwrap_or_default());
}

fn load(root: &Path) -> Option<WolfVariants> {
    let files: BTreeMap<String, String> =
        serde_json::from_slice(&std::fs::read(root.join("wolf-variants.json")).ok()?).ok()?;
    let tag_files: BTreeMap<String, String> = std::fs::read(root.join("worldgen/biome_tags.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let mut tags = HashMap::new();
    for id in tag_files.keys() {
        let mut out = HashSet::new();
        resolve(id, &tag_files, &mut out, &mut HashSet::new());
        tags.insert(id.clone(), out);
    }
    let mut rules = Vec::new();
    for (name, text) in &files {
        let Some(variant) = super::tame::WOLF_VARIANTS
            .iter()
            .position(|(n, _)| format!("minecraft:{n}") == *name)
        else {
            continue;
        };
        let doc: Value = serde_json::from_str(text).ok()?;
        for condition in doc["spawn_conditions"].as_array().into_iter().flatten() {
            let priority = condition["priority"].as_i64().unwrap_or(0);
            let inner = &condition["condition"];
            let biomes = match &inner["biomes"] {
                Value::String(s) => vec![s.clone()],
                Value::Array(a) => a
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect(),
                _ => Vec::new(),
            };
            rules.push(Rule {
                variant: variant as u8,
                priority,
                biomes,
            });
        }
    }
    Some(WolfVariants { rules, tags })
}

fn resolve(
    id: &str,
    files: &BTreeMap<String, String>,
    out: &mut HashSet<String>,
    seen: &mut HashSet<String>,
) {
    if !seen.insert(id.to_owned()) {
        return;
    }
    let Some(doc) = files
        .get(id)
        .and_then(|t| serde_json::from_str::<Value>(t).ok())
    else {
        return;
    };
    for value in doc["values"].as_array().into_iter().flatten() {
        let text = value.as_str().or_else(|| value["id"].as_str());
        if let Some(t) = text {
            match t.strip_prefix('#') {
                Some(inner) => resolve(inner, files, out, seen),
                None => {
                    out.insert(t.to_owned());
                }
            }
        }
    }
}

/// The variant the data picks for `biome`: the highest priority among the
/// rules that match; `None` when no mod replaced vanilla's rules or none
/// matches.
pub fn variant_for(biome: &str) -> Option<u8> {
    pick(VARIANTS.get()?, biome)
}

fn pick(data: &WolfVariants, biome: &str) -> Option<u8> {
    data.rules
        .iter()
        .filter(|r| {
            r.biomes.iter().any(|b| match b.strip_prefix('#') {
                Some(tag) => data.tags.get(tag).is_some_and(|s| s.contains(biome)),
                None => b == biome,
            })
        })
        .max_by_key(|r| r.priority)
        .map(|r| r.variant)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highest_priority_rule_wins_by_biome_or_tag() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/wolf-variant-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("worldgen")).unwrap();
        let rule = |biomes: &str, priority: i32| {
            format!(
                r#"{{"spawn_conditions": [{{"condition": {{"type": "minecraft:biome", "biomes": {biomes}}}, "priority": {priority}}}]}}"#
            )
        };
        let files = serde_json::json!({
            "minecraft:pale": rule("\"#c:has_wolf_variant/pale\"", 1),
            "minecraft:ashen": rule("\"minecraft:taiga\"", 5),
            "minecraft:unknown_kind": rule("\"minecraft:taiga\"", 9),
        });
        std::fs::write(dir.join("wolf-variants.json"), files.to_string()).unwrap();
        let tags = serde_json::json!({
            "c:has_wolf_variant/pale": r##"{"values": ["#c:other", "minecraft:plains"]}"##,
            "c:other": r#"{"values": ["terra:hills"]}"#,
        });
        std::fs::write(dir.join("worldgen/biome_tags.json"), tags.to_string()).unwrap();
        let data = load(&dir).unwrap();
        assert_eq!(pick(&data, "terra:hills"), Some(0));
        assert_eq!(pick(&data, "minecraft:plains"), Some(0));
        assert_eq!(pick(&data, "minecraft:taiga"), Some(4));
        assert_eq!(pick(&data, "minecraft:desert"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
