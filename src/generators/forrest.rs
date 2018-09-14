use generators::GeneratedWorld;

use color;
use formula;
use item::{self, Item};
use level::{Tile, TileKind};
use monster::{Kind, Monster};
use player::Modifier;
use point::Point;
use rand::distributions::{Distribution, Weighted, WeightedChoice};
use rand::Rng;

// TODO: Instead of `map_size`, use a Rectangle with the world
// positions here. We want to expose the non-world coordinates in as
// few places as possible.
fn generate_map<R: Rng, G: Rng>(
    rng: &mut R,
    throwavay_rng: &mut G,
    map_size: Point,
    player_pos: Point,
) -> Vec<(Point, Tile)> {
    let mut weights = [
        Weighted {
            weight: 610,
            item: TileKind::Empty,
        },
        Weighted {
            weight: 390,
            item: TileKind::Tree,
        },
    ];
    let opts = WeightedChoice::new(&mut weights);
    let mut result = vec![];
    // NOTE: starting with `y` seems weird but it'll generate the right pattern:
    // start at top left corner, moving to the right
    for y in 0..map_size.y {
        for x in 0..map_size.x {
            // TODO: due to coordinate conversion, this is wrong for
            // every chunk but the one the player is in.
            //
            // Player always starts at an empty space:
            let kind = if player_pos == (x, y) {
                TileKind::Empty
            } else {
                opts.sample(rng)
            };

            let mut tile = Tile::new(kind);
            if tile.kind == TileKind::Tree {
                let options = [color::tree_1, color::tree_2, color::tree_3];
                tile.fg_color = *throwavay_rng.choose(&options).unwrap();
            }

            result.push((Point::new(x, y), tile));
        }
    }
    result
}

fn generate_monsters<R: Rng>(rng: &mut R, map: &[(Point, Tile)]) -> Vec<Monster> {
    // 3% chance a monster gets spawned
    let monster_count = 5;
    let monster_chance = 30;
    let mut weights = [
        Weighted {
            weight: 1000 - monster_chance,
            item: None,
        },
        Weighted {
            weight: monster_chance / monster_count,
            item: Some(Kind::Anxiety),
        },
        Weighted {
            weight: monster_chance / monster_count,
            item: Some(Kind::Depression),
        },
        Weighted {
            weight: monster_chance / monster_count,
            item: Some(Kind::Hunger),
        },
        Weighted {
            weight: monster_chance / monster_count,
            item: Some(Kind::Shadows),
        },
        Weighted {
            weight: monster_chance / monster_count,
            item: Some(Kind::Voices),
        },
        Weighted {
            weight: 2,
            item: Some(Kind::Npc),
        },
    ];
    let opts = WeightedChoice::new(&mut weights);
    let mut result = vec![];
    for &(pos, tile) in map.iter() {
        if tile.kind != TileKind::Empty {
            continue;
        }
        if let Some(kind) = opts.sample(rng) {
            let mut monster = Monster::new(kind, pos);
            if kind == Kind::Npc {
                use color;
                use monster::CompanionBonus::*;
                let bonus = rng.gen();
                monster.companion_bonus = Some(bonus);
                monster.color = match bonus {
                    DoubleWillGrowth => color::npc_will,
                    HalveExhaustion => color::npc_mind,
                    ExtraActionPoint => color::npc_speed,
                };
            };
            result.push(monster);
        }
    }
    result
}

fn new_item<R: Rng>(kind: item::Kind, rng: &mut R) -> Item {
    use item::Kind::*;
    match kind {
        Dose => {
            let mut item = formula::DOSE_PREFAB;
            if let Modifier::Intoxication {
                ref mut state_of_mind,
                ..
            } = item.modifier
            {
                *state_of_mind += formula::DOSE_MIND_VARIANCE.random(rng);
            };
            item
        }
        StrongDose => {
            let mut item = formula::STRONG_DOSE_PREFAB;
            if let Modifier::Intoxication {
                ref mut state_of_mind,
                ..
            } = item.modifier
            {
                *state_of_mind += formula::STRONG_DOSE_MIND_VARIANCE.random(rng);
            };
            item
        }
        CardinalDose => {
            let mut item = formula::CARDINAL_DOSE_PREFAB;
            if let Modifier::Intoxication {
                ref mut state_of_mind,
                ..
            } = item.modifier
            {
                *state_of_mind += formula::CARDINAL_DOSE_MIND_VARIANCE.random(rng);
            };
            item
        }
        DiagonalDose => {
            let mut item = formula::DIAGONAL_DOSE_PREFAB;
            if let Modifier::Intoxication {
                ref mut state_of_mind,
                ..
            } = item.modifier
            {
                *state_of_mind += formula::DIAGONAL_DOSE_MIND_VARIANCE.random(rng);
            };
            item
        }
        Food => formula::FOOD_PREFAB,
    }
}

fn generate_items<R: Rng>(rng: &mut R, map: &[(Point, Tile)]) -> Vec<(Point, Item)> {
    use item::Kind::*;

    let mut weights = [
        Weighted {
            weight: 1000,
            item: None,
        },
        Weighted {
            weight: 8,
            item: Some(Dose),
        },
        Weighted {
            weight: 3,
            item: Some(StrongDose),
        },
        Weighted {
            weight: 2,
            item: Some(CardinalDose),
        },
        Weighted {
            weight: 2,
            item: Some(DiagonalDose),
        },
        Weighted {
            weight: 5,
            item: Some(Food),
        },
    ];

    let generator = WeightedChoice::new(&mut weights);

    let mut result = vec![];
    for &(pos, tile) in map.iter() {
        match tile.kind {
            TileKind::Tree => {
                // Occupied tile, do nothing.
            }
            TileKind::Empty => {
                if let Some(kind) = generator.sample(rng) {
                    result.push((pos, new_item(kind, rng)));
                }
            }
        }
    }
    result
}

pub fn generate<R: Rng, G: Rng>(
    rng: &mut R,
    throwavay_rng: &mut G,
    size: Point,
    player: Point,
) -> GeneratedWorld {
    let map = generate_map(rng, throwavay_rng, size, player);
    let monsters = generate_monsters(rng, &map);
    let items = generate_items(rng, &map);
    (map, monsters, items)
}
