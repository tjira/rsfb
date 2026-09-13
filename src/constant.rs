// =============================================================================
// Feature Toggles (Enable / Disable)
// =============================================================================

// Combat & Exploration
/// Enable or disable tavern expeditions / quests.
pub const ENABLE_EXPEDITION: bool = true;

/// Enable or disable PvP Arena battles.
pub const ENABLE_ARENA: bool = true;

/// Enable or disable Dungeon fights (dungeons, tower, portal, shadow world).
pub const ENABLE_DUNGEON: bool = true;

/// Enable or disable Pet activities (habitat fights, pet battles, feeding).
pub const ENABLE_PETS: bool = true;

// Base Management
/// Enable or disable Fortress management (resource harvesting, buildings, gem mine, units, attacks).
pub const ENABLE_FORTRESS: bool = true;

/// Enable or disable Underworld management (resource harvesting, buildings, luring hero fights, units).
pub const ENABLE_UNDERWORLD: bool = true;

/// Enable or disable the Idle game (Arena Manager) building upgrades and rune sacrifices.
pub const ENABLE_IDLE: bool = true;

// Character & Economy
/// Enable or disable inventory management (equipping upgrades, dismantling, selling).
pub const ENABLE_INVENTORY: bool = true;

/// Enable or disable shop interactions (buying items, scrapbooks, dice).
pub const ENABLE_SHOP: bool = true;

/// Enable or disable automatic skill attribute upgrades.
pub const ENABLE_SKILL: bool = true;

/// Enable or disable automatic mount rental and renewals.
pub const ENABLE_MOUNT: bool = true;

/// Enable or disable witch interactions (cauldron donations, enchantments).
pub const ENABLE_WITCH: bool = true;

// Social & Guild
/// Enable or disable Guild activities (battles, portal, hydra, donations).
pub const ENABLE_GUILD: bool = true;

/// Enable or disable reading inbox mail and player news.
pub const ENABLE_MAIL: bool = true;

// Routine & Automation
/// Enable or disable daily rewards (calendar, daily & event quests, Hellevator, Wheel of Fortune, dice).
pub const ENABLE_DAILY: bool = true;

/// Enable or disable city guard work when out of thirst or before daytime start.
pub const ENABLE_GUARD: bool = true;

/// Enable or disable automatic unlocking of pending feature unlocks.
pub const ENABLE_UNLOCK: bool = true;

// =============================================================================
// Bot Schedule & Status
// =============================================================================

/// The hour (in 24-hour local time) when the active botting day begins (5:00 AM).
/// Before this hour, daytime activities (expeditions, shops, dungeons) are skipped
/// and city guard duty is prioritized until this target time.
pub const EXPEDITION_START_HOUR: u32 = 5;

/// Interval (in seconds) between periodic refreshes of the multi-account status summary table.
pub const STATUS_TABLE_INTERVAL_SECS: u64 = 300;

// =============================================================================
// Inventory & Equipment
// =============================================================================

/// Minimum number of empty slots kept in the inventory before triggering item cleanup.
/// When free slots drop below this number, the bot begins selling or sacrificing surplus items.
pub const INVENTORY_MIN_FREE_SLOTS: usize = 5;

/// Weighting multiplier applied when comparing Epic/Legendary equipment against normal gear.
/// Normal items must have more than 2x the main attribute of an epic to replace it,
/// while an epic can replace a normal item even if its main attribute is lower.
pub const EPIC_LEGENDARY_MULTIPLIER: f64 = 2.0;

/// Minimum Arcane Splinters an item must yield for proactive blacksmith dismantling.
/// Items yielding more than this threshold are dismantled immediately while daily dismantles
/// remain, even if the inventory still has plenty of free space.
pub const BLACKSMITH_MIN_ARCANE_DISMANTLE: u64 = 1000;

// =============================================================================
// Skills & Attribute Weights
// =============================================================================

/// Multiplier on the highest shop item price kept as a silver buffer.
/// Skill attributes will only be upgraded if current silver exceeds this multiple of the max shop item price.
pub const SKILL_GOLD_SAFETY_MULTIPLIER: u64 = 10;

/// Target basis ratio weight for the character's main class attribute.
pub const SKILL_WEIGHT_MAIN_ATTRIBUTE: f64 = 100.0;

/// Target basis ratio weight for Constitution.
pub const SKILL_WEIGHT_CONSTITUTION: f64 = 80.0;

/// Target basis ratio weight for Luck.
pub const SKILL_WEIGHT_LUCK: f64 = 40.0;

/// Target basis ratio weight for secondary (non-main) attributes.
pub const SKILL_WEIGHT_SECONDARY_ATTRIBUTE: f64 = 10.0;

// =============================================================================
// Fortress & Underworld
// =============================================================================

/// Minimum interval (in minutes) between checks for harvesting produced Fortress and Underworld resources.
pub const HARVEST_CHECK_INTERVAL_MINS: i64 = 30;

/// Capacity ratio required before harvesting Fortress and Underworld resources (e.g. 0.50 for 50% capacity).
pub const FORTRESS_HARVEST_STORAGE_RATIO: f64 = 0.50;

/// Required ratio of Underworld Goblin level to opponent hero level before attacking.
/// The goblin's level must be at least 1.5x the hero's level to safely initiate combat.
pub const GOBLIN_LEVEL_HERO_RATIO: f64 = 1.5;

// =============================================================================
// Currency & Guild Limits
// =============================================================================

/// Minimum number of mushrooms kept in reserve when spending mushrooms on optional upgrades.
/// Upgrades costing mushrooms will only be performed if the remaining mushrooms after purchase
/// are at least this amount.
pub const MIN_MUSHROOM_RESERVE: u32 = 30;

/// Maximum proportion of total mushrooms allowed for a single guild upgrade.
/// The mushroom price must be strictly less than this ratio (e.g. 0.10 for 10%).
pub const GUILD_UPGRADE_MAX_MUSHROOM_RATIO: f64 = 0.10;

// =============================================================================
// Daily & Mini-Games
// =============================================================================

/// Maximum number of total spins allowed per day on the Wheel of Fortune (Dr. Abawuwu).
pub const WHEEL_MAX_DAILY_SPINS: u8 = 20;

/// Maximum number of total spins allowed per day on the Wheel of Fortune (Dr. Abawuwu) during the Lucky Day event.
pub const WHEEL_MAX_DAILY_SPINS_LUCKY_DAY_EVENT: u8 = 40;
