//! The item and recipe browser extra (`docs/extras.md`), after JEI and EMI:
//! every item in a panel beside the inventory, and for any item the recipes
//! that make it and the ones that use it. Off by default; vanilla has no such
//! screen. The recipes are the local data's (vanilla's and the content mods'):
//! since 1.21.2 a server sends a client only the recipe book's displays of
//! recipes it unlocked, not the recipes themselves.
use crate::{
    crafting::{Ingredient, Recipe, Recipes},
    stations::{Smithing, StationRecipes},
};
use std::collections::HashMap;

/// One recipe as the browser draws it: each slot is the items it accepts
/// (one, or a tag's or list's items cycled through).
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    Crafting {
        width: usize,
        height: usize,
        cells: Vec<Vec<String>>,
        shapeless: bool,
        result: String,
        count: u32,
    },
    Cooking {
        /// `minecraft:smelting`, `blasting`, `smoking` or `campfire_cooking`.
        kind: String,
        input: Vec<String>,
        result: String,
        count: u32,
        time: u32,
        experience: f32,
    },
    Stonecutting {
        input: Vec<String>,
        result: String,
        count: u32,
    },
    /// A trim keeps the base item: `result` is None.
    Smithing {
        template: Vec<String>,
        base: Vec<String>,
        addition: Vec<String>,
        result: Option<String>,
    },
}
impl Shown {
    /// The item made; a trim makes its base items.
    pub fn results(&self) -> Vec<&str> {
        match self {
            Shown::Crafting { result, .. }
            | Shown::Cooking { result, .. }
            | Shown::Stonecutting { result, .. } => vec![result.as_str()],
            Shown::Smithing {
                result: Some(result),
                ..
            } => vec![result.as_str()],
            Shown::Smithing { base, .. } => base.iter().map(String::as_str).collect(),
        }
    }
    /// Every slot that takes items.
    pub fn inputs(&self) -> Vec<&[String]> {
        match self {
            Shown::Crafting { cells, .. } => cells
                .iter()
                .filter(|c| !c.is_empty())
                .map(Vec::as_slice)
                .collect(),
            Shown::Cooking { input, .. } | Shown::Stonecutting { input, .. } => {
                vec![input.as_slice()]
            }
            Shown::Smithing {
                template,
                base,
                addition,
                ..
            } => vec![template.as_slice(), base.as_slice(), addition.as_slice()],
        }
    }
    /// The block the recipe is made at, as an item for its icon.
    pub fn station(&self) -> &'static str {
        match self {
            Shown::Crafting { .. } => "minecraft:crafting_table",
            Shown::Cooking { kind, .. } => match kind.as_str() {
                "minecraft:blasting" => "minecraft:blast_furnace",
                "minecraft:smoking" => "minecraft:smoker",
                "minecraft:campfire_cooking" => "minecraft:campfire",
                _ => "minecraft:furnace",
            },
            Shown::Stonecutting { .. } => "minecraft:stonecutter",
            Shown::Smithing { .. } => "minecraft:smithing_table",
        }
    }
    pub fn title(&self) -> &'static str {
        match self {
            Shown::Crafting {
                shapeless: true, ..
            } => "Crafting (shapeless)",
            Shown::Crafting { .. } => "Crafting",
            Shown::Cooking { kind, .. } => match kind.as_str() {
                "minecraft:blasting" => "Blasting",
                "minecraft:smoking" => "Smoking",
                "minecraft:campfire_cooking" => "Campfire",
                _ => "Smelting",
            },
            Shown::Stonecutting { .. } => "Stonecutting",
            Shown::Smithing { result: None, .. } => "Smithing (trim)",
            Shown::Smithing { .. } => "Smithing",
        }
    }
}

/// The 3x3 (or 2x2) layout of a shapeless recipe's ingredients, row by row.
fn shapeless_cells(ingredients: &[Ingredient]) -> (usize, Vec<Vec<String>>) {
    let width = if ingredients.len() > 4 { 3 } else { 2 };
    let mut cells: Vec<Vec<String>> = ingredients.iter().map(|i| i.1.clone()).collect();
    let rows = cells.len().div_ceil(width).max(1);
    cells.resize(rows * width, Vec::new());
    (width, cells)
}

/// Every recipe the browser shows, and which make or use each item.
#[derive(Default)]
pub struct Lookup {
    recipes: Vec<Shown>,
    making: HashMap<String, Vec<usize>>,
    using: HashMap<String, Vec<usize>>,
}
impl Lookup {
    pub fn new(recipes: Vec<Shown>) -> Self {
        let mut making: HashMap<String, Vec<usize>> = HashMap::new();
        let mut using: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, r) in recipes.iter().enumerate() {
            for item in r.results() {
                let list = making.entry(item.to_owned()).or_default();
                if list.last() != Some(&i) {
                    list.push(i);
                }
            }
            for slot in r.inputs() {
                for item in slot {
                    let list = using.entry(item.clone()).or_default();
                    if list.last() != Some(&i) {
                        list.push(i);
                    }
                }
            }
        }
        Self {
            recipes,
            making,
            using,
        }
    }
    /// The crafting recipes with a pattern or ingredient list (the special
    /// ones, dyeing armour or copying maps, have code and no fixed slots),
    /// then the cooking, stonecutting and smithing ones.
    pub fn collect(recipes: &Recipes, stations: &StationRecipes) -> Self {
        let mut shown = Vec::new();
        for (_, recipe) in &recipes.crafting {
            match recipe {
                Recipe::Shaped {
                    width,
                    height,
                    cells,
                    result,
                } => {
                    if let Some(item) = &result.item {
                        shown.push(Shown::Crafting {
                            width: *width,
                            height: *height,
                            cells: cells
                                .iter()
                                .map(|c| c.as_ref().map(|i| i.1.clone()).unwrap_or_default())
                                .collect(),
                            shapeless: false,
                            result: item.to_string(),
                            count: result.count,
                        });
                    }
                }
                Recipe::Shapeless {
                    ingredients,
                    result,
                } => {
                    if let Some(item) = &result.item {
                        let (width, cells) = shapeless_cells(ingredients);
                        shown.push(Shown::Crafting {
                            width,
                            height: cells.len() / width,
                            cells,
                            shapeless: true,
                            result: item.to_string(),
                            count: result.count,
                        });
                    }
                }
                Recipe::Special(_) => {}
            }
        }
        for c in &recipes.cooking {
            shown.push(Shown::Cooking {
                kind: c.kind.clone(),
                input: c.ingredient.1.clone(),
                result: c.result.item.to_string(),
                count: c.result.count,
                time: c.time,
                experience: c.experience,
            });
        }
        for s in &stations.stonecutting {
            shown.push(Shown::Stonecutting {
                input: s.input.1.clone(),
                result: s.result.item.to_string(),
                count: s.result.count,
            });
        }
        for s in &stations.smithing {
            shown.push(match s {
                Smithing::Transform {
                    template,
                    base,
                    addition,
                    result,
                } => Shown::Smithing {
                    template: template.1.clone(),
                    base: base.1.clone(),
                    addition: addition.1.clone(),
                    result: Some(result.clone()),
                },
                Smithing::Trim {
                    template,
                    base,
                    addition,
                    ..
                } => Shown::Smithing {
                    template: template.1.clone(),
                    base: base.1.clone(),
                    addition: addition.1.clone(),
                    result: None,
                },
            });
        }
        Self::new(shown)
    }
    pub fn get(&self, index: usize) -> Option<&Shown> {
        self.recipes.get(index)
    }
    pub fn len(&self) -> usize {
        self.recipes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
    /// The recipes whose result is `item`.
    pub fn making(&self, item: &str) -> &[usize] {
        self.making.get(item).map_or(&[], Vec::as_slice)
    }
    /// The recipes with `item` in a slot.
    pub fn using(&self, item: &str) -> &[usize] {
        self.using.get(item).map_or(&[], Vec::as_slice)
    }
}

/// The panel's query: `@namespace` words keep the items of a namespace
/// (a prefix, as JEI's), the rest is the creative search's term.
pub fn split_query(query: &str) -> (Option<String>, String) {
    let mut namespace = None;
    let mut rest = Vec::new();
    for word in query.split_whitespace() {
        match word.strip_prefix('@') {
            Some(ns) if !ns.is_empty() => namespace = Some(ns.to_lowercase()),
            Some(_) => {}
            None => rest.push(word),
        }
    }
    (namespace, rest.join(" "))
}
/// Whether item `id` is in the namespace the query names.
pub fn in_namespace(id: &str, namespace: Option<&str>) -> bool {
    let Some(ns) = namespace else {
        return true;
    };
    let item_ns = id.split_once(':').map_or("minecraft", |(n, _)| n);
    item_ns.starts_with(ns)
}
/// The number of pages of `count` items, `per_page` a page (one at least).
pub fn pages(count: usize, per_page: usize) -> usize {
    count.div_ceil(per_page.max(1)).max(1)
}
/// The item a slot of several shows at `millis`: the next one each second.
pub fn cycled(items: &[String], millis: u128) -> Option<&str> {
    if items.is_empty() {
        return None;
    }
    Some(&items[(millis / 1000) as usize % items.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_owned()).collect()
    }
    fn sample() -> Lookup {
        let planks = items(&["minecraft:oak_planks", "minecraft:birch_planks"]);
        Lookup::new(vec![
            // Sticks from planks, the tag in two cells.
            Shown::Crafting {
                width: 1,
                height: 2,
                cells: vec![planks.clone(), planks.clone()],
                shapeless: false,
                result: "minecraft:stick".into(),
                count: 4,
            },
            Shown::Crafting {
                width: 1,
                height: 1,
                cells: vec![items(&["minecraft:oak_log"])],
                shapeless: true,
                result: "minecraft:oak_planks".into(),
                count: 4,
            },
            Shown::Cooking {
                kind: "minecraft:smelting".into(),
                input: items(&["minecraft:oak_log", "minecraft:birch_log"]),
                result: "minecraft:charcoal".into(),
                count: 1,
                time: 200,
                experience: 0.15,
            },
            Shown::Smithing {
                template: items(&["minecraft:coast_armor_trim_smithing_template"]),
                base: items(&["minecraft:iron_chestplate"]),
                addition: items(&["minecraft:copper_ingot"]),
                result: None,
            },
        ])
    }

    #[test]
    fn recipes_that_make_an_item() {
        let l = sample();
        assert_eq!(l.making("minecraft:stick"), &[0]);
        assert_eq!(l.making("minecraft:oak_planks"), &[1]);
        assert_eq!(l.making("minecraft:charcoal"), &[2]);
        // A trim makes its base.
        assert_eq!(l.making("minecraft:iron_chestplate"), &[3]);
        assert!(l.making("minecraft:diamond").is_empty());
    }

    #[test]
    fn recipes_that_use_an_item_once_each() {
        let l = sample();
        // Two cells of the same tag list the recipe once.
        assert_eq!(l.using("minecraft:birch_planks"), &[0]);
        assert_eq!(l.using("minecraft:oak_log"), &[1, 2]);
        assert_eq!(l.using("minecraft:copper_ingot"), &[3]);
        assert!(l.using("minecraft:stick").is_empty());
        assert_eq!(l.get(2).unwrap().station(), "minecraft:furnace");
        assert_eq!(l.get(3).unwrap().title(), "Smithing (trim)");
    }

    #[test]
    fn shapeless_ingredients_fill_a_grid() {
        let one = Ingredient::of(items(&["minecraft:dirt"]));
        let (w, cells) = shapeless_cells(&[one.clone(), one.clone(), one.clone()]);
        assert_eq!((w, cells.len()), (2, 4));
        assert!(cells[3].is_empty());
        let (w, cells) = shapeless_cells(&vec![one; 5]);
        assert_eq!((w, cells.len()), (3, 6));
    }

    #[test]
    fn the_query_takes_a_namespace() {
        assert_eq!(split_query("stone"), (None, "stone".into()));
        assert_eq!(
            split_query("@Mine red dye"),
            (Some("mine".into()), "red dye".into())
        );
        assert_eq!(split_query("@ log"), (None, "log".into()));
        assert!(in_namespace("minecraft:stone", Some("mine")));
        assert!(!in_namespace("t_and_t:lantern", Some("minecraft")));
        assert!(in_namespace("stone", Some("minecraft")));
        assert!(in_namespace("t_and_t:lantern", None));
    }

    #[test]
    fn pages_and_cycling() {
        assert_eq!(pages(0, 20), 1);
        assert_eq!(pages(41, 20), 3);
        assert_eq!(pages(5, 0), 5);
        let tag = items(&["a", "b", "c"]);
        assert_eq!(cycled(&tag, 0), Some("a"));
        assert_eq!(cycled(&tag, 1_500), Some("b"));
        assert_eq!(cycled(&tag, 3_200), Some("a"));
        assert_eq!(cycled(&[], 0), None);
    }

    #[test]
    fn every_recipe_kind_names_its_station() {
        for (kind, station, title) in [
            ("minecraft:smelting", "minecraft:furnace", "Smelting"),
            ("minecraft:blasting", "minecraft:blast_furnace", "Blasting"),
            ("minecraft:smoking", "minecraft:smoker", "Smoking"),
            (
                "minecraft:campfire_cooking",
                "minecraft:campfire",
                "Campfire",
            ),
        ] {
            let cooking = Shown::Cooking {
                kind: kind.into(),
                input: items(&["minecraft:beef"]),
                result: "minecraft:cooked_beef".into(),
                count: 1,
                time: 200,
                experience: 0.1,
            };
            assert_eq!(cooking.station(), station);
            assert_eq!(cooking.title(), title);
            assert_eq!(cooking.results(), ["minecraft:cooked_beef"]);
        }
        let stonecutting = Shown::Stonecutting {
            input: items(&["minecraft:stone"]),
            result: "minecraft:stone_slab".into(),
            count: 2,
        };
        assert_eq!(stonecutting.station(), "minecraft:stonecutter");
        assert_eq!(stonecutting.title(), "Stonecutting");
        assert_eq!(stonecutting.inputs(), [items(&["minecraft:stone"])]);
        let smithing = Shown::Smithing {
            template: items(&["minecraft:netherite_upgrade_smithing_template"]),
            base: items(&["minecraft:diamond_sword"]),
            addition: items(&["minecraft:netherite_ingot"]),
            result: Some("minecraft:netherite_sword".into()),
        };
        assert_eq!(smithing.station(), "minecraft:smithing_table");
        assert_eq!(smithing.results(), ["minecraft:netherite_sword"]);
        assert_eq!(smithing.inputs().len(), 3);
    }
}
