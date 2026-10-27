use serde::{Deserialize, Serialize};
use sim::{
    tick, Boar, Bot, ChatMessage, Command, EntityId, GameEvent, ItemKind, LeaderboardEntry,
    LootDrop, Player, Pond, Resource, Structure, StructureKind, Vec2, World, FIXED_DT, MAP_SIZE,
};
use wasm_bindgen::prelude::*;

const SAVE_KEY: &str = "ferrous.survival.save.v1";
const SAVE_VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct SaveData {
    version: u8,
    player: Player,
    structures: Vec<Structure>,
}

#[derive(Serialize)]
struct RenderData<'a> {
    alpha: f32,
    time: f32,
    player: &'a Player,
    bots: &'a [Bot],
    resources: &'a [Resource],
    structures: &'a [Structure],
    drops: &'a [LootDrop],
    boars: &'a [Boar],
    chat: &'a [ChatMessage],
    leaderboard: &'a [LeaderboardEntry],
    online_count: u32,
    ping: u32,
    resource_revision: u64,
    structure_revision: u64,
    context: String,
    events: Vec<GameEvent>,
}

#[derive(Serialize)]
struct MapData<'a> {
    size: f32,
    grid: usize,
    heights: Vec<f32>,
    biomes: Vec<u8>,
    ponds: &'a [Pond],
}

#[wasm_bindgen]
pub struct GameState {
    world: World,
    save_timer: f32,
    storage_available: bool,
}

#[wasm_bindgen]
impl GameState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        let seed = (js_sys::Date::now() as u64) ^ 0xf377_05;
        let mut world = World::new(seed, "Survivor".to_string());
        let mut storage_available = true;
        match load_save() {
            Ok(Some(save)) if save.version == SAVE_VERSION => {
                let mut player = save.player;
                player.id = EntityId(1);
                player.name = "Survivor".to_string();
                player.pos.x = if player.pos.x.is_finite() {
                    player.pos.x.clamp(-56.0, 56.0)
                } else {
                    0.0
                };
                player.pos.z = if player.pos.z.is_finite() {
                    player.pos.z.clamp(-56.0, 56.0)
                } else {
                    0.0
                };
                player.prev = player.pos;
                player.health = player.health.clamp(1.0, 100.0);
                player.hunger = player.hunger.clamp(15.0, 100.0);
                player.thirst = player.thirst.clamp(15.0, 100.0);
                player.temperature = player.temperature.clamp(10.0, 90.0);
                player.alive = true;
                player.poisoned = 0.0;
                player.inventory.validate();
                world.player = player;
                world.structures = save
                    .structures
                    .into_iter()
                    .filter(|structure| {
                        structure.owner == EntityId(1)
                            && structure.pos.x.is_finite()
                            && structure.pos.z.is_finite()
                    })
                    .take(180)
                    .collect();
            }
            Ok(_) => {}
            Err(_) => {
                storage_available = false;
            }
        }
        Self {
            world,
            save_timer: 4.0,
            storage_available,
        }
    }

    /// JS calls this once per animation frame. Rust advances only at a fixed 20 Hz.
    pub fn tick(&mut self, dt: f32) {
        tick(&mut self.world, dt);
        self.save_timer -= dt.clamp(0.0, 0.25);
        if self.save_timer <= 0.0 {
            self.save_timer = 4.0;
            self.save_now();
        }
    }

    /// Snapshot consumed by Three.js. Positions include previous/current pairs for interpolation.
    pub fn get_render_data(&mut self) -> JsValue {
        let context = self.world.context_hint();
        let data = RenderData {
            alpha: self.world.accumulator / FIXED_DT,
            time: self.world.time,
            player: &self.world.player,
            bots: &self.world.bots,
            resources: &self.world.resources,
            structures: &self.world.structures,
            drops: &self.world.drops,
            boars: &self.world.boars,
            chat: &self.world.chat.messages,
            leaderboard: &self.world.leaderboard.entries,
            online_count: self.world.online_count,
            ping: self.world.ping,
            resource_revision: self.world.resource_revision,
            structure_revision: self.world.structure_revision,
            context,
            events: self.world.events.clone(),
        };
        self.world.events.clear();
        serde_wasm_bindgen::to_value(&data).unwrap_or(JsValue::NULL)
    }

    pub fn get_map_data(&self) -> JsValue {
        let grid = 61;
        let mut heights = Vec::with_capacity(grid * grid);
        let mut biomes = Vec::with_capacity(grid * grid);
        for z in 0..grid {
            for x in 0..grid {
                let pos = Vec2::new(
                    -MAP_SIZE * 0.5 + x as f32 * MAP_SIZE / (grid - 1) as f32,
                    -MAP_SIZE * 0.5 + z as f32 * MAP_SIZE / (grid - 1) as f32,
                );
                heights.push(self.world.terrain_height(pos));
                biomes.push(self.world.biome_at(pos));
            }
        }
        serde_wasm_bindgen::to_value(&MapData {
            size: MAP_SIZE,
            grid,
            heights,
            biomes,
            ponds: &self.world.ponds,
        })
        .unwrap_or(JsValue::NULL)
    }

    /// Movement is continuous; action is an optional one-shot command.
    pub fn handle_input(
        &mut self,
        move_x: f32,
        move_z: f32,
        action: Option<String>,
        aim_x: f32,
        aim_z: f32,
    ) {
        self.world
            .set_input(move_x, move_z, Vec2::new(aim_x, aim_z));
        let Some(action) = action else {
            return;
        };
        let command = match action.as_str() {
            "gather" => Some(Command::Gather),
            "attack" => Some(Command::Attack),
            "cook" => Some(Command::Cook),
            "drink" => Some(Command::Drink),
            "respawn" => Some(Command::Respawn),
            "build:wall" => Some(Command::Build(StructureKind::Wall, Vec2::new(aim_x, aim_z))),
            "build:door" => Some(Command::Build(StructureKind::Door, Vec2::new(aim_x, aim_z))),
            "build:campfire" => Some(Command::Build(
                StructureKind::Campfire,
                Vec2::new(aim_x, aim_z),
            )),
            value if value.starts_with("craft:") => {
                ItemKind::parse(&value[6..]).map(Command::Craft)
            }
            value if value.starts_with("eat:") => ItemKind::parse(&value[4..]).map(Command::Eat),
            _ => None,
        };
        if let Some(command) = command {
            self.world.commands.push(command);
        }
    }

    pub fn send_chat(&mut self, text: String) {
        self.world.send_chat(&text);
    }

    pub fn save_now(&mut self) {
        if !self.storage_available {
            return;
        }
        let save = SaveData {
            version: SAVE_VERSION,
            player: self.world.player.clone(),
            structures: self
                .world
                .structures
                .iter()
                .filter(|structure| structure.owner == self.world.player.id)
                .cloned()
                .collect(),
        };
        if let Ok(json) = serde_json::to_string(&save) {
            if storage()
                .and_then(|storage| storage.set_item(SAVE_KEY, &json).ok())
                .is_none()
            {
                self.storage_available = false;
            }
        }
    }

    pub fn reset_save(&mut self) {
        if let Some(storage) = storage() {
            let _ = storage.remove_item(SAVE_KEY);
        }
        let seed = (js_sys::Date::now() as u64) ^ 0xf377_05;
        self.world = World::new(seed, "Survivor".to_string());
        self.storage_available = true;
        self.save_timer = 4.0;
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
}

fn load_save() -> Result<Option<SaveData>, ()> {
    let storage = storage().ok_or(())?;
    let value = storage.get_item(SAVE_KEY).map_err(|_| ())?;
    match value {
        Some(json) => serde_json::from_str(&json).map(Some).map_err(|_| ()),
        None => Ok(None),
    }
}
