use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use crate::constant::*;
use serde::Deserialize;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub features: FeatureConfig,
    pub schedule: ScheduleConfig,
    pub inventory: InventoryConfig,
    pub skills: SkillConfig,
    pub fortress: FortressConfig,
    pub economy: EconomyConfig,
    pub daily: DailyConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FeatureConfig {
    pub enable_expedition: bool,
    pub enable_arena: bool,
    pub enable_dungeon: bool,
    pub enable_pets: bool,
    pub enable_fortress: bool,
    pub enable_underworld: bool,
    pub enable_idle: bool,
    pub enable_inventory: bool,
    pub enable_shop: bool,
    pub enable_skill: bool,
    pub enable_mount: bool,
    pub enable_witch: bool,
    pub enable_guild: bool,
    pub enable_mail: bool,
    pub enable_daily: bool,
    pub enable_guard: bool,
    pub enable_unlock: bool,
    pub enable_toilet: bool,
    pub enable_blacksmith: bool,
    pub enable_hellevator: bool,
    pub enable_wheel: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            enable_expedition: ENABLE_EXPEDITION,
            enable_arena: ENABLE_ARENA,
            enable_dungeon: ENABLE_DUNGEON,
            enable_pets: ENABLE_PETS,
            enable_fortress: ENABLE_FORTRESS,
            enable_underworld: ENABLE_UNDERWORLD,
            enable_idle: ENABLE_IDLE,
            enable_inventory: ENABLE_INVENTORY,
            enable_shop: ENABLE_SHOP,
            enable_skill: ENABLE_SKILL,
            enable_mount: ENABLE_MOUNT,
            enable_witch: ENABLE_WITCH,
            enable_guild: ENABLE_GUILD,
            enable_mail: ENABLE_MAIL,
            enable_daily: ENABLE_DAILY,
            enable_guard: ENABLE_GUARD,
            enable_unlock: ENABLE_UNLOCK,
            enable_toilet: ENABLE_TOILET,
            enable_blacksmith: ENABLE_BLACKSMITH,
            enable_hellevator: ENABLE_HELLEVATOR,
            enable_wheel: ENABLE_WHEEL,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ScheduleConfig {
    pub expedition_hour: u32,
    pub status_table_secs: u64,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            expedition_hour: EXPEDITION_START_HOUR,
            status_table_secs: STATUS_TABLE_INTERVAL_SECS,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InventoryConfig {
    pub min_free_slots: usize,
    pub epic_multiplier: f64,
    pub min_arcane: u64,
    pub min_arcane_cleanup: u64,
}

impl Default for InventoryConfig {
    fn default() -> Self {
        Self {
            min_free_slots: INVENTORY_MIN_FREE_SLOTS,
            epic_multiplier: EPIC_LEGENDARY_MULTIPLIER,
            min_arcane: BLACKSMITH_MIN_ARCANE_DISMANTLE,
            min_arcane_cleanup: BLACKSMITH_MIN_ARCANE_CLEANUP,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SkillConfig {
    pub gold_safety_mult: u64,
    pub weight_main_attribute: f64,
    pub weight_constitution: f64,
    pub weight_luck: f64,
    pub weight_secondary: f64,
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            gold_safety_mult: SKILL_GOLD_SAFETY_MULTIPLIER,
            weight_main_attribute: SKILL_WEIGHT_MAIN_ATTRIBUTE,
            weight_constitution: SKILL_WEIGHT_CONSTITUTION,
            weight_luck: SKILL_WEIGHT_LUCK,
            weight_secondary: SKILL_WEIGHT_SECONDARY_ATTRIBUTE,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FortressConfig {
    pub harvest_check_mins: i64,
    pub harvest_ratio: f64,
    pub goblin_hero_ratio: f64,
}

impl Default for FortressConfig {
    fn default() -> Self {
        Self {
            harvest_check_mins: HARVEST_CHECK_INTERVAL_MINS,
            harvest_ratio: FORTRESS_HARVEST_STORAGE_RATIO,
            goblin_hero_ratio: GOBLIN_LEVEL_HERO_RATIO,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EconomyConfig {
    pub min_mushroom_reserve: u32,
    pub guild_mushroom_ratio: f64,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            min_mushroom_reserve: MIN_MUSHROOM_RESERVE,
            guild_mushroom_ratio: GUILD_UPGRADE_MAX_MUSHROOM_RATIO,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DailyConfig {
    pub wheel_max_daily_spins: u8,
    pub wheel_spins_lucky_day: u8,
}

impl Default for DailyConfig {
    fn default() -> Self {
        Self {
            wheel_max_daily_spins: WHEEL_MAX_DAILY_SPINS,
            wheel_spins_lucky_day: WHEEL_MAX_DAILY_SPINS_LUCKY_DAY_EVENT,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            features: FeatureConfig::default(),
            schedule: ScheduleConfig::default(),
            inventory: InventoryConfig::default(),
            skills: SkillConfig::default(),
            fortress: FortressConfig::default(),
            economy: EconomyConfig::default(),
            daily: DailyConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let candidates = [Path::new("rsfb.toml"), Path::new("config.toml")];

        for path in candidates {
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(content) => match toml::from_str(&content) {
                        Ok(cfg) => {
                            println!("LOADED CONFIGURATION FROM '{}'", path.display());

                            return cfg;
                        }

                        Err(err) => {
                            eprintln!("FAILED TO PARSE '{}' ({err})", path.display());
                        }
                    },

                    Err(err) => {
                        eprintln!("FAILED TO READ '{}' ({err})", path.display());
                    }
                }
            }
        }

        Config::default()
    }
}
