use benchlib::{bench_dataset, generate_vec, Generate, Rng};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u8)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Generate for GameType {
    fn generate<R: Rng>(rand: &mut R) -> Self {
        match rand.gen_range(0..4) {
            0 => GameType::Survival,
            1 => GameType::Creative,
            2 => GameType::Adventure,
            3 => GameType::Spectator,
            _ => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Item {
    count: i8,
    slot: u8,
    id: String,
}

impl Generate for Item {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        const IDS: [&str; 8] = [
            "dirt",
            "stone",
            "pickaxe",
            "sand",
            "gravel",
            "shovel",
            "chestplate",
            "steak",
        ];
        Self {
            count: rng.gen(),
            slot: rng.gen(),
            id: IDS[rng.gen_range(0..IDS.len())].to_string(),
        }
    }
}

#[derive(Archive, Serialize, Clone, Copy, Deserialize, Debug)]
pub struct Abilities {
    walk_speed: f32,
    fly_speed: f32,
    may_fly: bool,
    flying: bool,
    invulnerable: bool,
    may_build: bool,
    instabuild: bool,
}

impl Generate for Abilities {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        Self {
            walk_speed: rng.gen(),
            fly_speed: rng.gen(),
            may_fly: rng.gen_bool(0.5),
            flying: rng.gen_bool(0.5),
            invulnerable: rng.gen_bool(0.5),
            may_build: rng.gen_bool(0.5),
            instabuild: rng.gen_bool(0.5),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Entity {
    id: String,
    pos: [f64; 3],
    motion: [f64; 3],
    rotation: [f32; 2],
    fall_distance: f32,
    fire: u16,
    air: u16,
    on_ground: bool,
    no_gravity: bool,
    invulnerable: bool,
    portal_cooldown: i32,
    uuid: [u32; 4],
    custom_name: Option<String>,
    custom_name_visible: bool,
    silent: bool,
    glowing: bool,
}

impl Generate for Entity {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        const IDS: [&str; 8] = [
            "cow", "sheep", "zombie", "skeleton", "spider", "creeper",
            "parrot", "bee",
        ];
        const CUSTOM_NAMES: [&str; 8] = [
            "rainbow", "princess", "steve", "johnny", "missy", "coward",
            "fairy", "howard",
        ];

        Self {
            id: IDS[rng.gen_range(0..IDS.len())].to_string(),
            pos: <[f64; 3] as Generate>::generate(rng),
            motion: <[f64; 3] as Generate>::generate(rng),
            rotation: <[f32; 2] as Generate>::generate(rng),
            fall_distance: rng.gen(),
            fire: rng.gen(),
            air: rng.gen(),
            on_ground: rng.gen_bool(0.5),
            no_gravity: rng.gen_bool(0.5),
            invulnerable: rng.gen_bool(0.5),
            portal_cooldown: rng.gen(),
            uuid: <[u32; 4] as Generate>::generate(rng),
            custom_name: <Option<()> as Generate>::generate(rng).map(|_| {
                CUSTOM_NAMES[rng.gen_range(0..CUSTOM_NAMES.len())].to_string()
            }),
            custom_name_visible: rng.gen_bool(0.5),
            silent: rng.gen_bool(0.5),
            glowing: rng.gen_bool(0.5),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct RecipeBook {
    recipes: Vec<String>,
    to_be_displayed: Vec<String>,
    is_filtering_craftable: bool,
    is_gui_open: bool,
    is_furnace_filtering_craftable: bool,
    is_furnace_gui_open: bool,
    is_blasting_furnace_filtering_craftable: bool,
    is_blasting_furnace_gui_open: bool,
    is_smoker_filtering_craftable: bool,
    is_smoker_gui_open: bool,
}

impl Generate for RecipeBook {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        const RECIPES: [&str; 8] = [
            "pickaxe",
            "torch",
            "bow",
            "crafting table",
            "furnace",
            "shears",
            "arrow",
            "tnt",
        ];
        const MAX_RECIPES: usize = 30;
        const MAX_DISPLAYED_RECIPES: usize = 10;
        Self {
            recipes: generate_vec::<_, ()>(rng, 0..MAX_RECIPES)
                .iter()
                .map(|_| RECIPES[rng.gen_range(0..RECIPES.len())].to_string())
                .collect(),
            to_be_displayed: generate_vec::<_, ()>(
                rng,
                0..MAX_DISPLAYED_RECIPES,
            )
            .iter()
            .map(|_| RECIPES[rng.gen_range(0..RECIPES.len())].to_string())
            .collect(),
            is_filtering_craftable: rng.gen_bool(0.5),
            is_gui_open: rng.gen_bool(0.5),
            is_furnace_filtering_craftable: rng.gen_bool(0.5),
            is_furnace_gui_open: rng.gen_bool(0.5),
            is_blasting_furnace_filtering_craftable: rng.gen_bool(0.5),
            is_blasting_furnace_gui_open: rng.gen_bool(0.5),
            is_smoker_filtering_craftable: rng.gen_bool(0.5),
            is_smoker_gui_open: rng.gen_bool(0.5),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct RootVehicle {
    attach: [u32; 4],
    entity: Entity,
}

impl Generate for RootVehicle {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        Self {
            attach: <[u32; 4] as Generate>::generate(rng),
            entity: <Entity as Generate>::generate(rng),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Player {
    game_type: GameType,
    previous_game_type: GameType,
    score: u64,
    dimension: String,
    selected_item_slot: u32,
    selected_item: Item,
    spawn_dimension: Option<String>,
    spawn_x: i64,
    spawn_y: i64,
    spawn_z: i64,
    spawn_forced: Option<bool>,
    sleep_timer: u16,
    food_exhaustion_level: f32,
    food_saturation_level: f32,
    food_tick_timer: u32,
    xp_level: u32,
    xp_p: f32,
    xp_total: i32,
    xp_seed: i32,
    inventory: Vec<Item>,
    ender_items: Vec<Item>,
    abilities: Abilities,
    entered_nether_position: Option<[f64; 3]>,
    root_vehicle: Option<RootVehicle>,
    shoulder_entity_left: Option<Entity>,
    shoulder_entity_right: Option<Entity>,
    seen_credits: bool,
    recipe_book: RecipeBook,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Players {
    pub players: Vec<Player>,
}

impl Generate for Player {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        const DIMENSIONS: [&str; 3] = ["overworld", "nether", "end"];
        const MAX_ITEMS: usize = 40;
        const MAX_ENDER_ITEMS: usize = 27;
        Self {
            game_type: GameType::generate(rng),
            previous_game_type: GameType::generate(rng),
            score: rng.gen(),
            dimension: DIMENSIONS[rng.gen_range(0..DIMENSIONS.len())]
                .to_string(),
            selected_item_slot: rng.gen(),
            selected_item: Item::generate(rng),
            spawn_dimension: <Option<()> as Generate>::generate(rng).map(
                |_| DIMENSIONS[rng.gen_range(0..DIMENSIONS.len())].to_string(),
            ),
            spawn_x: rng.gen(),
            spawn_y: rng.gen(),
            spawn_z: rng.gen(),
            spawn_forced: <Option<bool> as Generate>::generate(rng),
            sleep_timer: rng.gen(),
            food_exhaustion_level: rng.gen(),
            food_saturation_level: rng.gen(),
            food_tick_timer: rng.gen(),
            xp_level: rng.gen(),
            xp_p: rng.gen(),
            xp_total: rng.gen(),
            xp_seed: rng.gen(),
            inventory: generate_vec(rng, 0..MAX_ITEMS),
            ender_items: generate_vec(rng, 0..MAX_ENDER_ITEMS),
            abilities: Abilities::generate(rng),
            entered_nether_position: <Option<[f64; 3]> as Generate>::generate(
                rng,
            ),
            root_vehicle: <Option<RootVehicle> as Generate>::generate(rng),
            shoulder_entity_left: <Option<Entity> as Generate>::generate(rng),
            shoulder_entity_right: <Option<Entity> as Generate>::generate(rng),
            seen_credits: rng.gen_bool(0.5),
            recipe_book: RecipeBook::generate(rng),
        }
    }
}

pub fn generate_players() -> Players {
    let mut rng = benchlib::rng();

    const PLAYERS: usize = 500;
    Players {
        players: generate_vec::<_, Player>(&mut rng, PLAYERS..PLAYERS + 1),
    }
}

bench_dataset!(Players = generate_players());
