/// The hour (in 24-hour local time) when the active botting day begins (5:00 AM).
/// Before this hour, daytime activities (expeditions, shops, dungeons) are skipped
/// and city guard duty is prioritized until this target time.
pub const EXPEDITION_START_HOUR: u32 = 5;

/// Weighting multiplier applied when comparing Epic/Legendary equipment against normal gear.
/// Normal items must have more than 2x the main attribute of an epic to replace it,
/// while an epic can replace a normal item even if its main attribute is lower.
pub const EPIC_LEGENDARY_MULTIPLIER: f64 = 2.0;

/// Required ratio of Underworld Goblin level to opponent hero level before attacking.
/// The goblin's level must be at least 1.5x the hero's level to safely initiate combat.
pub const GOBLIN_LEVEL_HERO_RATIO: f64 = 1.5;

/// Minimum number of empty slots kept in the inventory before triggering item cleanup.
/// When free slots drop below this number, the bot begins selling or sacrificing surplus items.
pub const INVENTORY_MIN_FREE_SLOTS: usize = 5;

/// Minimum Arcane Splinters an item must yield for proactive blacksmith dismantling.
/// Items yielding more than this threshold are dismantled immediately while daily dismantles
/// remain, even if the inventory still has plenty of free space.
pub const BLACKSMITH_MIN_ARCANE_DISMANTLE: u64 = 1000;

/// Multiplier on the highest shop item price kept as a silver buffer.
/// Skill attributes will only be upgraded if current silver exceeds this multiple of the max shop item price.
pub const SKILL_GOLD_SAFETY_MULTIPLIER: u64 = 10;

/// Interval (in seconds) between periodic refreshes of the multi-account status summary table.
pub const STATUS_TABLE_INTERVAL_SECS: u64 = 300;

/// Minimum interval (in minutes) between checks for harvesting produced Fortress and Underworld resources.
pub const HARVEST_CHECK_INTERVAL_MINS: i64 = 30;

/// Minimum number of mushrooms kept in reserve when spending mushrooms on optional upgrades.
/// Upgrades costing mushrooms will only be performed if the remaining mushrooms after purchase
/// are at least this amount.
pub const MIN_MUSHROOM_RESERVE: u32 = 30;

/// Maximum proportion of total mushrooms allowed for a single guild upgrade.
/// The mushroom price must be strictly less than this ratio (e.g. 0.10 for 10%).
pub const GUILD_UPGRADE_MAX_MUSHROOM_RATIO: f64 = 0.10;
