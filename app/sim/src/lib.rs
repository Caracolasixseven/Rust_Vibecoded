use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

pub const FIXED_DT: f32 = 1.0 / 20.0;
pub const MAP_SIZE: f32 = 120.0;
const BOT_COUNT: usize = 40;
const MAX_STRUCTURES: usize = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u32);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub z: f32,
}

impl Vec2 {
    pub fn new(x: f32, z: f32) -> Self {
        Self { x, z }
    }
    pub fn length(self) -> f32 {
        (self.x * self.x + self.z * self.z).sqrt()
    }
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0001 {
            Self::new(self.x / len, self.z / len)
        } else {
            Self::default()
        }
    }
    pub fn clamped(self) -> Self {
        let limit = MAP_SIZE * 0.5 - 2.0;
        Self::new(self.x.clamp(-limit, limit), self.z.clamp(-limit, limit))
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.z + rhs.z)
    }
}
impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.z - rhs.z)
    }
}
impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.z * rhs)
    }
}

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }
    pub fn usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max.max(1)
    }
    pub fn chance(&mut self, probability: f32) -> bool {
        self.f32() < probability
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Wood,
    Stone,
    MetalOre,
    Metal,
    Berry,
    RawMeat,
    CookedMeat,
    StoneHatchet,
    MetalHatchet,
    Campfire,
    WoodWall,
    WoodDoor,
}

impl ItemKind {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "stone_hatchet" => Self::StoneHatchet,
            "metal_hatchet" => Self::MetalHatchet,
            "campfire" => Self::Campfire,
            "wood_wall" => Self::WoodWall,
            "wood_door" => Self::WoodDoor,
            "berry" => Self::Berry,
            "raw_meat" => Self::RawMeat,
            "cooked_meat" => Self::CookedMeat,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemKind,
    pub count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<ItemStack>,
}

impl Inventory {
    pub fn count(&self, item: ItemKind) -> u32 {
        self.slots
            .iter()
            .find(|slot| slot.item == item)
            .map_or(0, |slot| slot.count)
    }
    pub fn add(&mut self, item: ItemKind, count: u32) -> bool {
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.item == item) {
            slot.count = slot.count.saturating_add(count);
            return true;
        }
        if self.slots.len() >= 12 {
            return false;
        }
        self.slots.push(ItemStack { item, count });
        true
    }
    pub fn remove(&mut self, item: ItemKind, count: u32) -> bool {
        if self.count(item) < count {
            return false;
        }
        let mut remaining = count;
        self.slots.retain_mut(|slot| {
            if slot.item != item || remaining == 0 {
                return true;
            }
            let taken = slot.count.min(remaining);
            slot.count -= taken;
            remaining -= taken;
            slot.count > 0
        });
        true
    }
    pub fn has(&self, item: ItemKind) -> bool {
        self.count(item) > 0
    }
    pub fn validate(&mut self) {
        self.slots
            .retain(|slot| slot.count > 0 && slot.count <= 9999);
        self.slots.truncate(12);
    }
}

pub fn recipe(item: ItemKind) -> &'static [(ItemKind, u32)] {
    match item {
        ItemKind::StoneHatchet => &[(ItemKind::Wood, 6), (ItemKind::Stone, 4)],
        ItemKind::MetalHatchet => &[
            (ItemKind::Wood, 4),
            (ItemKind::Metal, 6),
            (ItemKind::Stone, 8),
        ],
        ItemKind::Campfire => &[(ItemKind::Wood, 10), (ItemKind::Stone, 6)],
        ItemKind::WoodWall => &[(ItemKind::Wood, 8)],
        ItemKind::WoodDoor => &[(ItemKind::Wood, 12)],
        _ => &[],
    }
}

pub fn craft(inventory: &mut Inventory, item: ItemKind) -> bool {
    let ingredients = recipe(item);
    if ingredients.is_empty()
        || inventory.has(item) && matches!(item, ItemKind::StoneHatchet | ItemKind::MetalHatchet)
    {
        return false;
    }
    if !ingredients
        .iter()
        .all(|(kind, count)| inventory.count(*kind) >= *count)
    {
        return false;
    }
    for (kind, count) in ingredients {
        inventory.remove(*kind, *count);
    }
    inventory.add(item, 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotState {
    Idle,
    Wander,
    Gather,
    Craft,
    Build,
    Fight,
    Flee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Personality {
    Aggressive,
    Gatherer,
    Builder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    Tree,
    Rock,
    Bush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureKind {
    Wall,
    Door,
    Campfire,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: EntityId,
    pub name: String,
    pub pos: Vec2,
    pub prev: Vec2,
    pub facing: f32,
    pub health: f32,
    pub hunger: f32,
    pub thirst: f32,
    pub temperature: f32,
    pub inventory: Inventory,
    pub gathered: u32,
    pub kills: u32,
    pub built: u32,
    pub score: u32,
    pub alive: bool,
    pub attack_cooldown: f32,
    pub poisoned: f32,
    #[serde(default)]
    pub spawn_shield: f32,
}

impl Player {
    fn new(id: EntityId, name: String) -> Self {
        let mut inventory = Inventory::default();
        inventory.add(ItemKind::Wood, 8);
        inventory.add(ItemKind::Stone, 4);
        inventory.add(ItemKind::Berry, 3);
        Self {
            id,
            name,
            pos: Vec2::default(),
            prev: Vec2::default(),
            facing: 0.0,
            health: 100.0,
            hunger: 100.0,
            thirst: 100.0,
            temperature: 55.0,
            inventory,
            gathered: 0,
            kills: 0,
            built: 0,
            score: 0,
            alive: true,
            attack_cooldown: 0.0,
            poisoned: 0.0,
            spawn_shield: 18.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bot {
    pub id: EntityId,
    pub name: String,
    pub skin: u32,
    pub pos: Vec2,
    pub prev: Vec2,
    pub facing: f32,
    pub health: f32,
    pub hunger: f32,
    pub inventory: Inventory,
    pub state: BotState,
    pub personality: Personality,
    pub goal: Vec2,
    pub target: Option<EntityId>,
    pub state_timer: f32,
    pub respawn_timer: f32,
    pub attack_cooldown: f32,
    pub gathered: u32,
    pub kills: u32,
    pub built: u32,
    pub score: u32,
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: EntityId,
    pub kind: ResourceKind,
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub respawn_timer: f32,
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Structure {
    pub id: EntityId,
    pub owner: EntityId,
    pub kind: StructureKind,
    pub pos: Vec2,
    pub rotation: f32,
    pub hp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootDrop {
    pub id: EntityId,
    pub pos: Vec2,
    pub inventory: Inventory,
    pub ttl: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boar {
    pub id: EntityId,
    pub pos: Vec2,
    pub prev: Vec2,
    pub goal: Vec2,
    pub facing: f32,
    pub health: f32,
    pub alive: bool,
    pub respawn_timer: f32,
    pub attack_cooldown: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pond {
    pub pos: Vec2,
    pub radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: u32,
    pub sender: String,
    pub text: String,
    pub is_player: bool,
    pub time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChat {
    pub sender: String,
    pub text: String,
    pub delay: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatSimulator {
    pub messages: Vec<ChatMessage>,
    pub pending: Vec<PendingChat>,
    pub next_message: f32,
    pub next_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub id: EntityId,
    pub name: String,
    pub score: u32,
    pub kills: u32,
    pub is_player: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Leaderboard {
    pub entries: Vec<LeaderboardEntry>,
    pub timer: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub kind: String,
    pub message: String,
    pub pos: Vec2,
}

#[derive(Debug, Clone)]
pub enum Command {
    Gather,
    Attack,
    Craft(ItemKind),
    Build(StructureKind, Vec2),
    Eat(ItemKind),
    Cook,
    Drink,
    Respawn,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerInput {
    pub move_x: f32,
    pub move_z: f32,
    pub aim: Vec2,
}

pub struct NameGenerator;

impl NameGenerator {
    pub fn generate(rng: &mut Rng) -> String {
        const ADJECTIVES: &[&str] = &[
            "Dark", "Silent", "Rusty", "Wild", "Neon", "Mad", "Iron", "Lucky", "Toxic", "Sleepy",
        ];
        const NOUNS: &[&str] = &[
            "Slayer", "Wolf", "Raider", "Builder", "Ghost", "Boar", "Nomad", "Viper", "Miner",
            "Fox",
        ];
        const CLANS: &[&str] = &["TTV", "YT", "ESP", "RUS", "GG", "EZ"];
        const SPECIAL: &[&str] = &[
            "NoobMaster69",
            "RustLover99",
            "xX_DarkSlayer_Xx",
            "CargoEnjoyer",
            "StoneAgeAndy",
        ];
        if rng.chance(0.16) {
            return SPECIAL[rng.usize(SPECIAL.len())].to_string();
        }
        let adjective = ADJECTIVES[rng.usize(ADJECTIVES.len())];
        let noun = NOUNS[rng.usize(NOUNS.len())];
        let number = rng.usize(100);
        match rng.usize(5) {
            0 => format!("xX_{}{}_Xx", adjective, noun),
            1 => format!("{}{:02}", noun, number),
            2 => format!("[{}] {}{}", CLANS[rng.usize(CLANS.len())], adjective, noun),
            3 => format!("{}_{}{}", adjective, noun, number),
            _ => format!("{}{}", adjective, noun),
        }
    }
}

pub struct World {
    pub seed: u64,
    rng: Rng,
    pub next_id: u32,
    pub time: f32,
    pub accumulator: f32,
    pub player: Player,
    pub bots: Vec<Bot>,
    pub resources: Vec<Resource>,
    pub structures: Vec<Structure>,
    pub drops: Vec<LootDrop>,
    pub boars: Vec<Boar>,
    pub ponds: Vec<Pond>,
    pub chat: ChatSimulator,
    pub leaderboard: Leaderboard,
    pub online_count: u32,
    pub ping: u32,
    pub resource_revision: u64,
    pub structure_revision: u64,
    pub events: Vec<GameEvent>,
    pub input: PlayerInput,
    pub commands: Vec<Command>,
    ambience_timer: f32,
}

impl World {
    pub fn new(seed: u64, player_name: String) -> Self {
        let mut world = Self {
            seed,
            rng: Rng::new(seed ^ 0x9e37_79b9_7f4a_7c15),
            next_id: 2,
            time: 0.0,
            accumulator: 0.0,
            player: Player::new(EntityId(1), player_name),
            bots: Vec::with_capacity(BOT_COUNT),
            resources: Vec::new(),
            structures: Vec::new(),
            drops: Vec::new(),
            boars: Vec::new(),
            ponds: vec![
                Pond {
                    pos: Vec2::new(-19.0, -15.0),
                    radius: 5.5,
                },
                Pond {
                    pos: Vec2::new(24.0, 20.0),
                    radius: 6.5,
                },
                Pond {
                    pos: Vec2::new(30.0, -31.0),
                    radius: 4.5,
                },
            ],
            chat: ChatSimulator {
                next_message: 3.0,
                ..ChatSimulator::default()
            },
            leaderboard: Leaderboard::default(),
            online_count: 41,
            ping: 32,
            resource_revision: 0,
            structure_revision: 0,
            events: Vec::new(),
            input: PlayerInput::default(),
            commands: Vec::new(),
            ambience_timer: 2.0,
        };
        world.player.pos = world.random_land_position();
        world.player.prev = world.player.pos;
        for _ in 0..BOT_COUNT {
            world.spawn_bot();
        }
        for index in 0..165 {
            let kind = match index % 11 {
                0..=5 => ResourceKind::Tree,
                6..=9 => ResourceKind::Rock,
                _ => ResourceKind::Bush,
            };
            world.spawn_resource(kind);
        }
        for _ in 0..12 {
            world.spawn_boar();
        }
        world.push_chat("SERVIDOR", "Bienvenido a EU #04 · respeta el roleo", false);
        world.push_chat("RustLover99", "alguien tiene hierro?", false);
        world.refresh_leaderboard();
        world
    }

    fn alloc_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    fn random_position(&mut self) -> Vec2 {
        Vec2::new(
            self.rng.range(-MAP_SIZE * 0.47, MAP_SIZE * 0.47),
            self.rng.range(-MAP_SIZE * 0.47, MAP_SIZE * 0.47),
        )
    }

    pub fn random_land_position(&mut self) -> Vec2 {
        for _ in 0..24 {
            let pos = self.random_position();
            if !self.near_water(pos, 1.0) && pos.length() > 5.0 {
                return pos;
            }
        }
        Vec2::new(4.0, 4.0)
    }

    fn spawn_bot(&mut self) {
        let id = self.alloc_id();
        let name = NameGenerator::generate(&mut self.rng);
        let pos = self.random_land_position();
        let skin = self.rng.usize(8) as u32;
        let personality = match self.rng.usize(3) {
            0 => Personality::Aggressive,
            1 => Personality::Gatherer,
            _ => Personality::Builder,
        };
        let mut inventory = Inventory::default();
        inventory.add(ItemKind::Wood, self.rng.usize(18) as u32);
        inventory.add(ItemKind::Stone, self.rng.usize(12) as u32);
        inventory.add(ItemKind::Berry, 1 + self.rng.usize(3) as u32);
        self.bots.push(Bot {
            id,
            name,
            skin,
            pos,
            prev: pos,
            facing: self.rng.range(0.0, PI * 2.0),
            health: 100.0,
            hunger: self.rng.range(55.0, 100.0),
            inventory,
            state: BotState::Idle,
            personality,
            goal: pos,
            target: None,
            state_timer: self.rng.range(0.1, 2.0),
            respawn_timer: 0.0,
            attack_cooldown: 0.0,
            gathered: 0,
            kills: 0,
            built: 0,
            score: self.rng.usize(180) as u32,
            alive: true,
        });
    }

    fn spawn_resource(&mut self, kind: ResourceKind) {
        let id = self.alloc_id();
        let pos = self.random_land_position();
        let max_hp = match kind {
            ResourceKind::Tree => 100.0,
            ResourceKind::Rock => 130.0,
            ResourceKind::Bush => 45.0,
        };
        self.resources.push(Resource {
            id,
            kind,
            pos,
            hp: max_hp,
            max_hp,
            respawn_timer: 0.0,
            alive: true,
        });
    }

    fn spawn_boar(&mut self) {
        let id = self.alloc_id();
        let pos = self.random_land_position();
        self.boars.push(Boar {
            id,
            pos,
            prev: pos,
            goal: pos,
            facing: 0.0,
            health: 80.0,
            alive: true,
            respawn_timer: 0.0,
            attack_cooldown: 0.0,
        });
    }

    pub fn set_input(&mut self, move_x: f32, move_z: f32, aim: Vec2) {
        self.input = PlayerInput {
            move_x: move_x.clamp(-1.0, 1.0),
            move_z: move_z.clamp(-1.0, 1.0),
            aim,
        };
    }

    pub fn send_chat(&mut self, text: &str) {
        let clean: String = text.chars().filter(|c| !c.is_control()).take(120).collect();
        let clean = clean.trim();
        if clean.is_empty() {
            return;
        }
        let player_name = self.player.name.clone();
        self.push_chat(&player_name, clean, true);
        let lower = clean.to_lowercase();
        let response = if lower.contains('?') || lower.contains("hierro") || lower.contains("iron")
        {
            "yo tengo un poco, qué das?"
        } else if lower.contains("hola") || lower.contains("hello") || lower.contains("gg") {
            "gg bro"
        } else if lower.contains("trade") || lower.contains("cambio") {
            "tradeo madera por piedra, ven al lago"
        } else {
            "te leo, estoy farmeando por el bosque"
        };
        if let Some(bot) = self
            .bots
            .iter()
            .filter(|bot| bot.alive)
            .nth(self.rng.usize(12))
        {
            let delay = self.rng.range(1.0, 3.0);
            self.chat.pending.push(PendingChat {
                sender: bot.name.clone(),
                text: response.to_string(),
                delay,
            });
        }
    }

    pub fn push_chat(&mut self, sender: &str, text: &str, is_player: bool) {
        self.chat.next_id = self.chat.next_id.wrapping_add(1);
        self.chat.messages.push(ChatMessage {
            id: self.chat.next_id,
            sender: sender.to_string(),
            text: text.to_string(),
            is_player,
            time: self.time,
        });
        if self.chat.messages.len() > 14 {
            self.chat.messages.remove(0);
        }
    }

    pub fn context_hint(&self) -> String {
        if !self.player.alive {
            return "Tu supervivencia ha terminado".into();
        }
        if self
            .drops
            .iter()
            .any(|drop| drop.pos.distance(self.player.pos) < 3.0)
        {
            return "E · Recoger botín".into();
        }
        if let Some(resource) = self.resources.iter().filter(|r| r.alive).min_by(|a, b| {
            a.pos
                .distance(self.player.pos)
                .total_cmp(&b.pos.distance(self.player.pos))
        }) {
            if resource.pos.distance(self.player.pos) < 3.4 {
                return match resource.kind {
                    ResourceKind::Tree => "E · Talar árbol",
                    ResourceKind::Rock => "E · Picar roca",
                    ResourceKind::Bush => "E · Recoger bayas",
                }
                .into();
            }
        }
        if self.near_campfire(self.player.pos, 4.0) {
            return "F · Cocinar carne / fundir hierro".into();
        }
        if self.near_water(self.player.pos, 1.0) {
            return "Q · Beber agua".into();
        }
        if self.player.spawn_shield > 0.0 {
            format!(
                "Protección de aparición · {}s",
                self.player.spawn_shield.ceil() as u32
            )
        } else {
            "WASD para explorar · Espacio para atacar".into()
        }
    }

    pub fn near_water(&self, pos: Vec2, margin: f32) -> bool {
        self.ponds
            .iter()
            .any(|pond| pond.pos.distance(pos) < pond.radius + margin)
    }

    pub fn near_campfire(&self, pos: Vec2, radius: f32) -> bool {
        self.structures.iter().any(|structure| {
            structure.kind == StructureKind::Campfire && structure.pos.distance(pos) < radius
        })
    }

    pub fn terrain_height(&self, pos: Vec2) -> f32 {
        let broad = perlin(self.seed, pos.x * 0.022, pos.z * 0.022);
        let detail = perlin(self.seed ^ 77, pos.x * 0.065, pos.z * 0.065);
        let mut height = broad * 2.2 + detail * 0.45;
        for pond in &self.ponds {
            let distance = pond.pos.distance(pos);
            if distance < pond.radius * 1.45 {
                let influence = 1.0 - (distance / (pond.radius * 1.45)).clamp(0.0, 1.0);
                height -= influence * 3.2;
            }
        }
        height
    }

    pub fn biome_at(&self, pos: Vec2) -> u8 {
        let value = perlin(self.seed ^ 912, pos.x * 0.015 + 40.0, pos.z * 0.015 - 20.0);
        if value > 0.24 {
            2
        } else if value < -0.22 {
            1
        } else {
            0
        }
    }

    fn step(&mut self, dt: f32) {
        self.time += dt;
        self.player.attack_cooldown = (self.player.attack_cooldown - dt).max(0.0);
        self.player.spawn_shield = (self.player.spawn_shield - dt).max(0.0);
        let commands = std::mem::take(&mut self.commands);
        for command in commands {
            self.execute_command(command);
        }
        self.update_player(dt);
        self.update_bots(dt);
        self.update_boars(dt);
        self.update_resources(dt);
        self.update_chat(dt);
        self.update_ambience(dt);
        self.drops.retain_mut(|drop| {
            drop.ttl -= dt;
            drop.ttl > 0.0
        });
        self.leaderboard.timer -= dt;
        if self.leaderboard.timer <= 0.0 {
            self.leaderboard.timer = 2.0;
            self.refresh_leaderboard();
        }
    }

    fn execute_command(&mut self, command: Command) {
        if matches!(command, Command::Respawn) {
            self.respawn_player();
            return;
        }
        if !self.player.alive {
            return;
        }
        match command {
            Command::Gather => self.player_gather(),
            Command::Attack => self.player_attack(),
            Command::Craft(item) => {
                if craft(&mut self.player.inventory, item) {
                    self.event("craft", "Objeto fabricado", self.player.pos);
                } else {
                    self.event(
                        "notice",
                        "Faltan materiales o ya tienes esa herramienta",
                        self.player.pos,
                    );
                }
            }
            Command::Build(kind, position) => self.player_build(kind, position),
            Command::Eat(item) => self.player_eat(item),
            Command::Cook => self.player_cook(),
            Command::Drink => {
                if self.near_water(self.player.pos, 1.0) {
                    self.player.thirst = (self.player.thirst + 45.0).min(100.0);
                    self.event("drink", "Agua recuperada", self.player.pos);
                } else {
                    self.event("notice", "Busca una orilla para beber", self.player.pos);
                }
            }
            Command::Respawn => {}
        }
    }

    fn update_player(&mut self, dt: f32) {
        if !self.player.alive {
            return;
        }
        self.player.prev = self.player.pos;
        let movement = Vec2::new(self.input.move_x, self.input.move_z);
        let direction = movement.normalized();
        let speed = 5.4
            * if self.player.hunger < 15.0 || self.player.thirst < 15.0 {
                0.7
            } else {
                1.0
            };
        self.player.pos = (self.player.pos + direction * (speed * dt)).clamped();
        if direction.length() > 0.1 {
            self.player.facing = direction.z.atan2(direction.x);
            self.player.hunger = (self.player.hunger - dt * 0.055).max(0.0);
            self.player.thirst = (self.player.thirst - dt * 0.085).max(0.0);
        } else if self.input.aim.distance(self.player.pos) > 0.5 {
            let aim = (self.input.aim - self.player.pos).normalized();
            self.player.facing = aim.z.atan2(aim.x);
        }
        self.player.hunger = (self.player.hunger - dt * 0.035).max(0.0);
        self.player.thirst = (self.player.thirst - dt * 0.055).max(0.0);
        let biome = self.biome_at(self.player.pos);
        let mut target_temperature = match biome {
            2 => 76.0,
            1 => 43.0,
            _ => 54.0,
        };
        if self.near_campfire(self.player.pos, 5.0) {
            target_temperature = 88.0;
        }
        self.player.temperature += (target_temperature - self.player.temperature) * dt * 0.08;
        if self.player.hunger <= 0.0 || self.player.thirst <= 0.0 {
            self.damage_player(dt * 2.5, "inanición");
        }
        if self.player.temperature < 18.0 || self.player.temperature > 94.0 {
            self.damage_player(dt * 1.5, "exposición");
        }
        if self.player.poisoned > 0.0 {
            self.player.poisoned -= dt;
            self.damage_player(dt * 1.8, "intoxicación");
        }
    }

    fn player_gather(&mut self) {
        if self.player.attack_cooldown > 0.0 {
            return;
        }
        if let Some(drop_index) = self
            .drops
            .iter()
            .position(|drop| drop.pos.distance(self.player.pos) < 3.0)
        {
            let loot = self.drops.remove(drop_index);
            for stack in loot.inventory.slots {
                self.player.inventory.add(stack.item, stack.count);
            }
            self.event("pickup", "Botín recogido", self.player.pos);
            return;
        }
        let resource_index = self
            .resources
            .iter()
            .enumerate()
            .filter(|(_, resource)| resource.alive && resource.pos.distance(self.player.pos) < 3.5)
            .min_by(|(_, a), (_, b)| {
                a.pos
                    .distance(self.player.pos)
                    .total_cmp(&b.pos.distance(self.player.pos))
            })
            .map(|(index, _)| index);
        let Some(index) = resource_index else {
            self.event(
                "notice",
                "Acércate a un árbol, roca o arbusto",
                self.player.pos,
            );
            return;
        };
        let tool_damage = if self.player.inventory.has(ItemKind::MetalHatchet) {
            68.0
        } else if self.player.inventory.has(ItemKind::StoneHatchet) {
            42.0
        } else {
            22.0
        };
        self.player.attack_cooldown = 0.42;
        let pos = self.resources[index].pos;
        self.resources[index].hp -= tool_damage;
        self.resource_revision += 1;
        self.event("gather", "", pos);
        if self.resources[index].hp <= 0.0 {
            let kind = self.resources[index].kind;
            self.resources[index].alive = false;
            self.resources[index].respawn_timer = self.rng.range(42.0, 70.0);
            let (item, amount) = match kind {
                ResourceKind::Tree => (ItemKind::Wood, 7),
                ResourceKind::Rock => (ItemKind::Stone, 6),
                ResourceKind::Bush => (ItemKind::Berry, 4),
            };
            self.player.inventory.add(item, amount);
            if matches!(kind, ResourceKind::Rock) {
                self.player.inventory.add(ItemKind::MetalOre, 2);
            }
            self.player.gathered += amount;
            self.player.score += amount;
            self.event("deplete", format!("+{} recurso", amount), pos);
        }
    }

    fn player_attack(&mut self) {
        if self.player.attack_cooldown > 0.0 {
            return;
        }
        self.player.attack_cooldown = 0.48;
        let damage = if self.player.inventory.has(ItemKind::MetalHatchet) {
            55.0
        } else if self.player.inventory.has(ItemKind::StoneHatchet) {
            34.0
        } else {
            18.0
        };
        let pos = self.player.pos;
        let bot_index = self
            .bots
            .iter()
            .enumerate()
            .filter(|(_, bot)| bot.alive && bot.pos.distance(pos) < 3.1)
            .min_by(|(_, a), (_, b)| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
            .map(|(index, _)| index);
        if let Some(index) = bot_index {
            self.bots[index].health -= damage;
            self.event("hit", format!("-{}", damage as u32), self.bots[index].pos);
            if self.bots[index].health <= 0.0 {
                self.kill_bot(index);
                self.player.kills += 1;
                self.player.score += 100;
            }
            return;
        }
        let boar_index = self
            .boars
            .iter()
            .enumerate()
            .filter(|(_, boar)| boar.alive && boar.pos.distance(pos) < 3.1)
            .min_by(|(_, a), (_, b)| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
            .map(|(index, _)| index);
        if let Some(index) = boar_index {
            self.boars[index].health -= damage;
            self.event("hit", format!("-{}", damage as u32), self.boars[index].pos);
            if self.boars[index].health <= 0.0 {
                self.kill_boar(index);
            }
        }
    }

    fn player_build(&mut self, kind: StructureKind, requested: Vec2) {
        if self.structures.len() >= MAX_STRUCTURES {
            self.event("notice", "Límite de estructuras alcanzado", self.player.pos);
            return;
        }
        let item = match kind {
            StructureKind::Wall => ItemKind::WoodWall,
            StructureKind::Door => ItemKind::WoodDoor,
            StructureKind::Campfire => ItemKind::Campfire,
        };
        if !self.player.inventory.has(item) {
            self.event("notice", "Primero fabrica esta estructura", self.player.pos);
            return;
        }
        let offset = requested - self.player.pos;
        let position = if offset.length() > 0.5 {
            self.player.pos + offset.normalized() * offset.length().min(7.0)
        } else {
            self.player.pos + Vec2::new(self.player.facing.cos(), self.player.facing.sin()) * 3.0
        };
        let position = if matches!(kind, StructureKind::Campfire) {
            position
        } else {
            Vec2::new(
                (position.x / 2.0).round() * 2.0,
                (position.z / 2.0).round() * 2.0,
            )
        };
        if self.near_water(position, -0.5)
            || self
                .structures
                .iter()
                .any(|s| s.pos.distance(position) < 1.2)
        {
            self.event("notice", "No puedes construir aquí", self.player.pos);
            return;
        }
        self.player.inventory.remove(item, 1);
        let id = self.alloc_id();
        let rotation = if matches!(kind, StructureKind::Campfire) {
            0.0
        } else if (requested.x - self.player.pos.x).abs() > (requested.z - self.player.pos.z).abs()
        {
            PI * 0.5
        } else {
            0.0
        };
        let hp = match kind {
            StructureKind::Wall => 350.0,
            StructureKind::Door => 250.0,
            StructureKind::Campfire => 120.0,
        };
        self.structures.push(Structure {
            id,
            owner: self.player.id,
            kind,
            pos: position,
            rotation,
            hp,
        });
        self.player.built += 1;
        self.player.score += 40;
        self.structure_revision += 1;
        self.event("build", "Estructura construida", position);
    }

    fn player_eat(&mut self, item: ItemKind) {
        if !self.player.inventory.remove(item, 1) {
            self.event("notice", "No tienes ese alimento", self.player.pos);
            return;
        }
        match item {
            ItemKind::Berry => {
                self.player.hunger = (self.player.hunger + 14.0).min(100.0);
                self.player.thirst = (self.player.thirst + 5.0).min(100.0);
            }
            ItemKind::RawMeat => {
                self.player.hunger = (self.player.hunger + 30.0).min(100.0);
                if self.rng.chance(0.38) {
                    self.player.poisoned = 12.0;
                    self.event(
                        "notice",
                        "Intoxicación: la carne cruda era arriesgada",
                        self.player.pos,
                    );
                }
            }
            ItemKind::CookedMeat => {
                self.player.hunger = (self.player.hunger + 52.0).min(100.0);
                self.player.health = (self.player.health + 8.0).min(100.0);
            }
            _ => {}
        }
        self.event("eat", "", self.player.pos);
    }

    fn player_cook(&mut self) {
        if !self.near_campfire(self.player.pos, 4.0) {
            self.event(
                "notice",
                "Necesitas estar junto a una hoguera",
                self.player.pos,
            );
            return;
        }
        if self.player.inventory.remove(ItemKind::RawMeat, 1) {
            self.player.inventory.add(ItemKind::CookedMeat, 1);
            self.event("craft", "Carne cocinada", self.player.pos);
        } else if self.player.inventory.remove(ItemKind::MetalOre, 1) {
            self.player.inventory.add(ItemKind::Metal, 1);
            self.event("craft", "Mineral fundido", self.player.pos);
        } else {
            self.event(
                "notice",
                "No tienes carne cruda ni mineral",
                self.player.pos,
            );
        }
    }

    fn respawn_player(&mut self) {
        self.player.pos = self.random_land_position();
        self.player.prev = self.player.pos;
        self.player.health = 100.0;
        self.player.hunger = 80.0;
        self.player.thirst = 80.0;
        self.player.temperature = 55.0;
        self.player.poisoned = 0.0;
        self.player.spawn_shield = 12.0;
        self.player.alive = true;
        self.event("respawn", "Has vuelto al servidor", self.player.pos);
    }

    fn damage_player(&mut self, amount: f32, reason: &str) {
        if !self.player.alive {
            return;
        }
        self.player.health -= amount;
        if self.player.health <= 0.0 {
            self.player.health = 0.0;
            self.player.alive = false;
            self.event("death", format!("Moriste por {}", reason), self.player.pos);
            self.push_chat(
                "SERVIDOR",
                &format!("{} ha muerto por {}", self.player.name, reason),
                false,
            );
        }
    }

    fn event(&mut self, kind: &str, message: impl Into<String>, pos: Vec2) {
        self.events.push(GameEvent {
            kind: kind.to_string(),
            message: message.into(),
            pos,
        });
        if self.events.len() > 32 {
            self.events.remove(0);
        }
    }

    fn update_bots(&mut self, dt: f32) {
        for index in 0..self.bots.len() {
            self.bots[index].prev = self.bots[index].pos;
            if !self.bots[index].alive {
                self.bots[index].respawn_timer -= dt;
                if self.bots[index].respawn_timer <= 0.0 {
                    self.respawn_bot(index);
                }
                continue;
            }
            self.bots[index].state_timer -= dt;
            self.bots[index].attack_cooldown = (self.bots[index].attack_cooldown - dt).max(0.0);
            self.bots[index].hunger = (self.bots[index].hunger - dt * 0.06).max(0.0);
            if self.bots[index].hunger < 35.0
                && self.bots[index].inventory.remove(ItemKind::Berry, 1)
            {
                self.bots[index].hunger = (self.bots[index].hunger + 25.0).min(100.0);
            }
            if self.bots[index].hunger <= 0.0 {
                self.bots[index].health -= dt * 0.8;
            }
            if self.bots[index].health <= 0.0 {
                self.kill_bot(index);
                continue;
            }
            if self.bots[index].health < 26.0 && self.bots[index].state != BotState::Flee {
                self.bots[index].state = BotState::Flee;
                self.bots[index].state_timer = 4.0;
                self.bots[index].target = self.nearest_combat_target(index, 18.0);
            }
            let state = self.bots[index].state;
            if self.bots[index].state_timer <= 0.0
                && matches!(state, BotState::Idle | BotState::Wander | BotState::Flee)
            {
                self.choose_bot_state(index);
            }
            match self.bots[index].state {
                BotState::Idle => {}
                BotState::Wander => {
                    let goal = self.bots[index].goal;
                    self.move_bot_toward(index, goal, 2.4, dt);
                    if self.bots[index].pos.distance(goal) < 1.0 {
                        self.bots[index].state_timer = 0.0;
                    }
                }
                BotState::Gather => self.update_bot_gather(index, dt),
                BotState::Craft => {
                    if self.bots[index].state_timer <= 0.0 {
                        let has_tool = self.bots[index].inventory.has(ItemKind::StoneHatchet);
                        if !has_tool
                            && craft(&mut self.bots[index].inventory, ItemKind::StoneHatchet)
                        {
                            self.bots[index].score += 15;
                        }
                        self.bots[index].state = BotState::Wander;
                        self.bots[index].state_timer = self.rng.range(1.0, 3.0);
                    }
                }
                BotState::Build => {
                    if self.bots[index].state_timer <= 0.0 {
                        if self.bots[index].inventory.remove(ItemKind::Wood, 8)
                            && self.structures.len() < MAX_STRUCTURES
                        {
                            let pos = self.bots[index].pos;
                            let id = self.alloc_id();
                            let owner = self.bots[index].id;
                            let rotation = if self.rng.chance(0.5) { 0.0 } else { PI * 0.5 };
                            self.structures.push(Structure {
                                id,
                                owner,
                                kind: StructureKind::Wall,
                                pos,
                                rotation,
                                hp: 350.0,
                            });
                            self.bots[index].built += 1;
                            self.bots[index].score += 40;
                            self.structure_revision += 1;
                            self.event("build", "", pos);
                        }
                        self.bots[index].state = BotState::Wander;
                        self.bots[index].state_timer = self.rng.range(2.0, 5.0);
                    }
                }
                BotState::Fight => self.update_bot_fight(index, dt),
                BotState::Flee => {
                    let target = self.bots[index]
                        .target
                        .and_then(|id| self.position_of(id))
                        .unwrap_or(self.player.pos);
                    let away = (self.bots[index].pos - target).normalized();
                    let goal = (self.bots[index].pos + away * 8.0).clamped();
                    self.move_bot_toward(index, goal, 4.8, dt);
                }
            }
        }
    }

    fn choose_bot_state(&mut self, index: usize) {
        let personality = self.bots[index].personality;
        let roll = self.rng.f32();
        let aggression = match personality {
            Personality::Aggressive => 0.52,
            Personality::Gatherer => 0.12,
            Personality::Builder => 0.2,
        };
        if roll < aggression {
            if let Some(target) = self.nearest_combat_target(index, 15.0) {
                self.bots[index].state = BotState::Fight;
                self.bots[index].target = Some(target);
                self.bots[index].state_timer = 5.0;
                return;
            }
        }
        let wood = self.bots[index].inventory.count(ItemKind::Wood);
        let stone = self.bots[index].inventory.count(ItemKind::Stone);
        let has_tool = self.bots[index].inventory.has(ItemKind::StoneHatchet);
        if !has_tool && wood >= 6 && stone >= 4 {
            self.bots[index].state = BotState::Craft;
            self.bots[index].state_timer = 1.5;
            return;
        }
        if matches!(personality, Personality::Builder) && wood >= 8 && self.rng.chance(0.65) {
            self.bots[index].state = BotState::Build;
            self.bots[index].state_timer = 2.0;
            return;
        }
        if roll < 0.82 {
            if let Some(resource) = self.nearest_resource(self.bots[index].pos, 38.0) {
                self.bots[index].state = BotState::Gather;
                self.bots[index].target = Some(resource);
                self.bots[index].state_timer = 8.0;
                return;
            }
        }
        self.bots[index].state = if self.rng.chance(0.25) {
            BotState::Idle
        } else {
            BotState::Wander
        };
        self.bots[index].goal = self.random_land_position();
        self.bots[index].state_timer = self.rng.range(2.0, 6.0);
    }

    fn nearest_combat_target(&self, index: usize, radius: f32) -> Option<EntityId> {
        let pos = self.bots[index].pos;
        let id = self.bots[index].id;
        let mut best: Option<(EntityId, f32)> = None;
        if self.player.alive && self.player.spawn_shield <= 0.0 {
            let distance = self.player.pos.distance(pos);
            if distance < radius {
                best = Some((self.player.id, distance));
            }
        }
        for bot in &self.bots {
            if !bot.alive || bot.id == id {
                continue;
            }
            let distance = bot.pos.distance(pos);
            if distance < radius && best.map_or(true, |(_, best_distance)| distance < best_distance)
            {
                best = Some((bot.id, distance));
            }
        }
        best.map(|(id, _)| id)
    }

    fn nearest_resource(&self, pos: Vec2, radius: f32) -> Option<EntityId> {
        self.resources
            .iter()
            .filter(|resource| resource.alive && resource.pos.distance(pos) < radius)
            .min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
            .map(|resource| resource.id)
    }

    fn position_of(&self, id: EntityId) -> Option<Vec2> {
        if self.player.id == id && self.player.alive {
            return Some(self.player.pos);
        }
        self.bots
            .iter()
            .find(|bot| bot.id == id && bot.alive)
            .map(|bot| bot.pos)
    }

    fn move_bot_toward(&mut self, index: usize, goal: Vec2, speed: f32, dt: f32) {
        let offset = goal - self.bots[index].pos;
        if offset.length() < 0.05 {
            return;
        }
        let direction = offset.normalized();
        self.bots[index].pos = (self.bots[index].pos + direction * (speed * dt)).clamped();
        self.bots[index].facing = direction.z.atan2(direction.x);
    }

    fn update_bot_gather(&mut self, index: usize, _dt: f32) {
        let target = self.bots[index].target;
        let resource_index = target.and_then(|id| {
            self.resources
                .iter()
                .position(|resource| resource.id == id && resource.alive)
        });
        let Some(resource_index) = resource_index else {
            self.bots[index].state = BotState::Wander;
            self.bots[index].state_timer = 0.0;
            return;
        };
        let resource_pos = self.resources[resource_index].pos;
        if self.bots[index].pos.distance(resource_pos) > 2.7 {
            self.move_bot_toward(index, resource_pos, 3.0, FIXED_DT);
            return;
        }
        if self.bots[index].attack_cooldown > 0.0 {
            return;
        }
        self.bots[index].attack_cooldown = 0.75;
        let damage = if self.bots[index].inventory.has(ItemKind::StoneHatchet) {
            34.0
        } else {
            16.0
        };
        self.resources[resource_index].hp -= damage;
        self.resource_revision += 1;
        if self.resources[resource_index].hp <= 0.0 {
            let kind = self.resources[resource_index].kind;
            self.resources[resource_index].alive = false;
            self.resources[resource_index].respawn_timer = self.rng.range(45.0, 75.0);
            let (item, amount) = match kind {
                ResourceKind::Tree => (ItemKind::Wood, 6),
                ResourceKind::Rock => (ItemKind::Stone, 5),
                ResourceKind::Bush => (ItemKind::Berry, 3),
            };
            self.bots[index].inventory.add(item, amount);
            self.bots[index].gathered += amount;
            self.bots[index].score += amount;
            self.bots[index].state = BotState::Wander;
            self.bots[index].state_timer = 0.0;
        }
    }

    fn update_bot_fight(&mut self, index: usize, dt: f32) {
        let target = self.bots[index].target;
        let target_pos = target.and_then(|id| self.position_of(id));
        let Some(target_pos) = target_pos else {
            self.bots[index].state = BotState::Wander;
            self.bots[index].state_timer = 0.0;
            return;
        };
        let distance = self.bots[index].pos.distance(target_pos);
        if distance > 24.0 {
            self.bots[index].state = BotState::Wander;
            self.bots[index].state_timer = 0.0;
            return;
        }
        if distance > 2.4 {
            self.move_bot_toward(index, target_pos, 4.0, dt);
        }
        if distance <= 2.8 && self.bots[index].attack_cooldown <= 0.0 {
            self.bots[index].attack_cooldown = 0.72;
            self.bot_strike(index, target.unwrap());
        }
    }

    fn bot_strike(&mut self, attacker: usize, target: EntityId) {
        let damage = if self.bots[attacker].inventory.has(ItemKind::StoneHatchet) {
            26.0
        } else {
            13.0
        };
        let attacker_name = self.bots[attacker].name.clone();
        if target == self.player.id {
            if self.player.spawn_shield > 0.0 {
                return;
            }
            let pos = self.player.pos;
            self.damage_player(damage, &format!("{}", attacker_name));
            self.event("hurt", format!("-{}", damage as u32), pos);
            if !self.player.alive {
                self.bots[attacker].kills += 1;
                self.bots[attacker].score += 100;
            }
            return;
        }
        if let Some(target_index) = self
            .bots
            .iter()
            .position(|bot| bot.id == target && bot.alive)
        {
            self.bots[target_index].health -= damage;
            let pos = self.bots[target_index].pos;
            self.event("hit", "", pos);
            if self.bots[target_index].health <= 0.0 {
                self.kill_bot(target_index);
                self.bots[attacker].kills += 1;
                self.bots[attacker].score += 100;
            }
        }
    }

    fn kill_bot(&mut self, index: usize) {
        if !self.bots[index].alive {
            return;
        }
        let pos = self.bots[index].pos;
        let name = self.bots[index].name.clone();
        let inventory = std::mem::take(&mut self.bots[index].inventory);
        if !inventory.slots.is_empty() {
            let id = self.alloc_id();
            self.drops.push(LootDrop {
                id,
                pos,
                inventory,
                ttl: 90.0,
            });
        }
        self.bots[index].alive = false;
        self.bots[index].health = 0.0;
        self.bots[index].respawn_timer = 30.0;
        self.bots[index].state = BotState::Idle;
        self.bots[index].target = None;
        self.event("kill", format!("{} eliminado", name), pos);
        if self.rng.chance(0.35) {
            let message = if self.rng.chance(0.5) {
                "me mataron en el bosque"
            } else {
                "gg, vuelvo en 30"
            };
            self.push_chat(&name, message, false);
        }
    }

    fn respawn_bot(&mut self, index: usize) {
        let pos = self.random_land_position();
        let name = NameGenerator::generate(&mut self.rng);
        let skin = self.rng.usize(8) as u32;
        let mut inventory = Inventory::default();
        inventory.add(ItemKind::Berry, 2);
        inventory.add(ItemKind::Wood, self.rng.usize(8) as u32);
        self.bots[index].name = name;
        self.bots[index].skin = skin;
        self.bots[index].pos = pos;
        self.bots[index].prev = pos;
        self.bots[index].goal = pos;
        self.bots[index].health = 100.0;
        self.bots[index].hunger = 90.0;
        self.bots[index].inventory = inventory;
        self.bots[index].state = BotState::Wander;
        self.bots[index].state_timer = 1.0;
        self.bots[index].alive = true;
        self.bots[index].score = 0;
        self.bots[index].kills = 0;
        self.bots[index].gathered = 0;
        self.bots[index].built = 0;
    }

    fn update_boars(&mut self, dt: f32) {
        for index in 0..self.boars.len() {
            self.boars[index].prev = self.boars[index].pos;
            self.boars[index].attack_cooldown = (self.boars[index].attack_cooldown - dt).max(0.0);
            if !self.boars[index].alive {
                self.boars[index].respawn_timer -= dt;
                if self.boars[index].respawn_timer <= 0.0 {
                    let pos = self.random_land_position();
                    self.boars[index].pos = pos;
                    self.boars[index].prev = pos;
                    self.boars[index].goal = pos;
                    self.boars[index].health = 80.0;
                    self.boars[index].alive = true;
                }
                continue;
            }
            let pos = self.boars[index].pos;
            let distance = pos.distance(self.player.pos);
            let target = if self.player.alive && self.player.spawn_shield <= 0.0 && distance < 8.0 {
                Some(self.player.pos)
            } else {
                None
            };
            let goal = if let Some(player) = target {
                player
            } else {
                if pos.distance(self.boars[index].goal) < 1.0 || self.rng.chance(dt * 0.08) {
                    self.boars[index].goal = self.random_land_position();
                }
                self.boars[index].goal
            };
            let direction = (goal - pos).normalized();
            let speed = if target.is_some() { 4.2 } else { 1.4 };
            self.boars[index].pos = (pos + direction * (speed * dt)).clamped();
            if direction.length() > 0.1 {
                self.boars[index].facing = direction.z.atan2(direction.x);
            }
            if target.is_some() && distance < 2.1 && self.boars[index].attack_cooldown <= 0.0 {
                self.boars[index].attack_cooldown = 1.2;
                self.damage_player(12.0, "un jabalí");
                self.event("hurt", "-12", self.player.pos);
            }
        }
    }

    fn kill_boar(&mut self, index: usize) {
        let pos = self.boars[index].pos;
        self.boars[index].alive = false;
        self.boars[index].health = 0.0;
        self.boars[index].respawn_timer = 45.0;
        let mut inventory = Inventory::default();
        inventory.add(ItemKind::RawMeat, 3);
        inventory.add(ItemKind::Berry, 1);
        let id = self.alloc_id();
        self.drops.push(LootDrop {
            id,
            pos,
            inventory,
            ttl: 90.0,
        });
        self.player.score += 35;
        self.event("kill", "Jabalí cazado · recoge la carne", pos);
    }

    fn update_resources(&mut self, dt: f32) {
        for resource in &mut self.resources {
            if !resource.alive {
                resource.respawn_timer -= dt;
                if resource.respawn_timer <= 0.0 {
                    resource.alive = true;
                    resource.hp = resource.max_hp;
                    self.resource_revision += 1;
                }
            }
        }
    }

    fn update_chat(&mut self, dt: f32) {
        self.chat.next_message -= dt;
        let mut due = Vec::new();
        self.chat.pending.retain_mut(|pending| {
            pending.delay -= dt;
            if pending.delay <= 0.0 {
                due.push((pending.sender.clone(), pending.text.clone()));
                false
            } else {
                true
            }
        });
        for (sender, text) in due {
            self.push_chat(&sender, &text, false);
        }
        if self.chat.next_message <= 0.0 {
            const MESSAGES: &[&str] = &[
                "gg",
                "alguien tiene hierro?",
                "me mataron en el bosque",
                "tradeo madera por piedra",
                "hay un jabalí junto al lago",
                "quién quiere hacer team?",
                "mi base está en el desierto",
                "lag??",
                "acabo de empezar, soy pacífico",
                "ese muro no era tuyo jaja",
            ];
            let text = MESSAGES[self.rng.usize(MESSAGES.len())];
            let alive: Vec<String> = self
                .bots
                .iter()
                .filter(|bot| bot.alive)
                .map(|bot| bot.name.clone())
                .collect();
            if !alive.is_empty() {
                let sender = alive[self.rng.usize(alive.len())].clone();
                self.push_chat(&sender, text, false);
            }
            self.chat.next_message = self.rng.range(5.0, 15.0);
        }
    }

    fn update_ambience(&mut self, dt: f32) {
        self.ambience_timer -= dt;
        if self.ambience_timer <= 0.0 {
            self.ambience_timer = self.rng.range(3.0, 7.0);
            let alive_bots = self.bots.iter().filter(|bot| bot.alive).count() as i32;
            self.online_count = (alive_bots + 1).clamp(38, 44) as u32;
            self.ping = self.rng.usize(41) as u32 + 20;
        }
    }

    fn refresh_leaderboard(&mut self) {
        let mut entries = Vec::with_capacity(BOT_COUNT + 1);
        if self.player.alive {
            entries.push(LeaderboardEntry {
                id: self.player.id,
                name: self.player.name.clone(),
                score: self.player.score,
                kills: self.player.kills,
                is_player: true,
            });
        }
        for bot in self.bots.iter().filter(|bot| bot.alive) {
            entries.push(LeaderboardEntry {
                id: bot.id,
                name: bot.name.clone(),
                score: bot.score,
                kills: bot.kills,
                is_player: false,
            });
        }
        entries.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
        entries.truncate(10);
        self.leaderboard.entries = entries;
    }
}

pub fn tick(world: &mut World, dt: f32) {
    world.accumulator += dt.clamp(0.0, 0.25);
    while world.accumulator >= FIXED_DT {
        world.accumulator -= FIXED_DT;
        world.step(FIXED_DT);
    }
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn gradient(hash: u64, x: f32, z: f32) -> f32 {
    match hash & 3 {
        0 => x + z,
        1 => -x + z,
        2 => x - z,
        _ => -x - z,
    }
}

pub fn perlin(seed: u64, x: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let zi = z.floor() as i32;
    let xf = x - x.floor();
    let zf = z - z.floor();
    let hash = |hx: i32, hz: i32| -> u64 {
        let mut value = seed
            ^ (hx as i64 as u64).wrapping_mul(0x9e37_79b1)
            ^ (hz as i64 as u64).wrapping_mul(0x85eb_ca6b);
        value ^= value >> 16;
        value = value.wrapping_mul(0x7feb_352d);
        value ^= value >> 15;
        value
    };
    let u = fade(xf);
    let v = fade(zf);
    let aa = gradient(hash(xi, zi), xf, zf);
    let ba = gradient(hash(xi + 1, zi), xf - 1.0, zf);
    let ab = gradient(hash(xi, zi + 1), xf, zf - 1.0);
    let bb = gradient(hash(xi + 1, zi + 1), xf - 1.0, zf - 1.0);
    (lerp(lerp(aa, ba, u), lerp(ab, bb, u), v) * 0.7071).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_tick_is_deterministic() {
        let mut a = World::new(42, "Test".into());
        let mut b = World::new(42, "Test".into());
        tick(&mut a, 0.2);
        for _ in 0..4 {
            tick(&mut b, 0.05);
        }
        assert_eq!(a.time, b.time);
        assert_eq!(a.bots[0].pos.x, b.bots[0].pos.x);
        assert_eq!(a.bots.len(), 40);
    }

    #[test]
    fn crafting_consumes_ingredients() {
        let mut inventory = Inventory::default();
        inventory.add(ItemKind::Wood, 6);
        inventory.add(ItemKind::Stone, 4);
        assert!(craft(&mut inventory, ItemKind::StoneHatchet));
        assert!(inventory.has(ItemKind::StoneHatchet));
        assert_eq!(inventory.count(ItemKind::Wood), 0);
        assert!(!craft(&mut inventory, ItemKind::StoneHatchet));
    }

    #[test]
    fn chat_is_sanitized() {
        let mut world = World::new(7, "Test".into());
        world.send_chat("  hola\n\0  ");
        assert_eq!(world.chat.messages.last().unwrap().text, "hola");
        assert!(!world.chat.pending.is_empty());
        for _ in 0..70 {
            tick(&mut world, 0.05);
        }
        assert!(world.chat.pending.is_empty());
        assert!(world
            .chat
            .messages
            .iter()
            .any(|message| !message.is_player && message.text == "gg bro"));
    }
}
