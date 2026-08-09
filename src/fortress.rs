use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use strum::IntoEnumIterator;

use sf_api::{
    command::Command,
    gamestate::fortress::{FortressBuildingType, FortressResourceType, FortressUnitType},
    gamestate::underworld::{UnderworldBuildingType, UnderworldResourceType, UnderworldUnitType},
    misc::EnumMapGet,
    session::SimpleSession,
};

use crate::log::log;

static FS_COLLECTED_ON_STARTUP: Mutex<Option<HashSet<String>>> = Mutex::new(None);
static UW_COLLECTED_ON_STARTUP: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn fortress_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let Some(ref fortress) = gs.fortress else {
        return None;
    };

    if let Some(target) = fortress.building_upgrade.target {
        if let Some(finish) = fortress.building_upgrade.finish {
            if Local::now() >= finish {
                return Some(Command::FortressBuildFinish { f_type: target, mushrooms: 0 });
            }
        }
    }

    let gem_mine = fortress.buildings.get(FortressBuildingType::GemMine);

    if gem_mine.level > 0 {
        if let Some(finish) = fortress.gem_search.finish {
            if Local::now() >= finish {
                if gs.character.inventory.count_free_slots() > 0 {
                    return Some(Command::FortressGemStoneSearchFinish { mushrooms: 0 });
                }
            }
        }
    }

    let get_collectable = |resource: &sf_api::gamestate::fortress::FortressResource| {
        let last_collectable = resource.production.last_collectable;

        let Some(lu) = fortress.last_collectable_updated else {
            return last_collectable;
        };

        let seconds = (Local::now() - lu).num_seconds().max(0) as u64;
        let produce = (seconds * resource.production.per_hour) / 3600;

        if resource.production.limit > 0 {
            return (last_collectable + produce).min(resource.production.limit);
        }

        return last_collectable + produce;
    };

    let is_startup = {
        let mut startup_set = FS_COLLECTED_ON_STARTUP.lock().unwrap();

        startup_set.get_or_insert_with(HashSet::new).insert(gs.character.name.clone())
    };

    let is_collect_time = fortress.last_collectable_updated.map_or(true, |lu| {
        let (interval, window) = (chrono::Duration::minutes(30), chrono::Duration::minutes(2));

        let time_since_update = Local::now() - lu;

        time_since_update >= interval || time_since_update < window
    });

    if is_startup || is_collect_time {
        let wood = fortress.resources.get(FortressResourceType::Wood);

        let can_w = fortress.building_upgrade.target != Some(FortressBuildingType::WoodcuttersHut);

        let w_enough = get_collectable(wood) >= wood.production.limit / 2;

        if w_enough && wood.production.limit > 0 && wood.current < wood.limit && can_w {
            return Some(Command::FortressGather { resource: FortressResourceType::Wood });
        }

        let stone = fortress.resources.get(FortressResourceType::Stone);

        let can_stone = fortress.building_upgrade.target != Some(FortressBuildingType::Quarry);

        let s_enough = get_collectable(stone) >= stone.production.limit / 2;

        if s_enough && stone.production.limit > 0 && stone.current < stone.limit && can_stone {
            return Some(Command::FortressGather { resource: FortressResourceType::Stone });
        }

        let exp = fortress.resources.get(FortressResourceType::Experience);

        let can_exp = fortress.building_upgrade.target != Some(FortressBuildingType::Academy);

        let e_enough = get_collectable(exp) >= exp.production.limit / 2;

        if e_enough && exp.production.limit > 0 && can_exp {
            return Some(Command::FortressGather { resource: FortressResourceType::Experience });
        }
    }

    for unit_type in FortressUnitType::iter() {
        if fortress.building_upgrade.target == Some(unit_type.training_building()) {
            continue;
        }

        let unit = fortress.units.get(unit_type);

        let current_total = unit.count + unit.in_training;

        if current_total < unit.limit {
            let cost = unit.training.cost;

            let wood_s = fortress.resources.get(FortressResourceType::Wood);
            let stone = fortress.resources.get(FortressResourceType::Stone);

            let mut to_train = (unit.limit - current_total) as u32;

            if cost.wood > 0 {
                to_train = to_train.min((wood_s.current / cost.wood) as u32);
            }

            if cost.stone > 0 {
                to_train = to_train.min((stone.current / cost.stone) as u32);
            }

            if cost.silver > 0 {
                to_train = to_train.min((gs.character.silver / cost.silver) as u32);
            }

            if to_train > 0 {
                return Some(Command::FortressBuildUnit { unit: unit_type, count: to_train });
            }
        }
    }

    let smithy = fortress.buildings.get(FortressBuildingType::Smithy);

    if smithy.level > 0 {
        for unit_type in FortressUnitType::iter() {
            let building_level = fortress.buildings.get(unit_type.training_building()).level;

            if building_level > 0 {
                let unit = fortress.units.get(unit_type);

                let cost = &unit.upgrade_cost;

                if cost.wood > 0 && cost.stone > 0 {
                    let max_level = match smithy.level {
                        00 => 000,
                        01 => 028,
                        02 => 030,
                        03 => 035,
                        04 => 040,
                        05 => 045,
                        06 => 050,
                        07 => 055,
                        08 => 062,
                        09 => 070,
                        10 => 077,

                        lvl => 5 * lvl + 30,
                    };

                    if unit.upgrade_next_lvl <= max_level as u64 {
                        let wood_c = fortress.resources.get(FortressResourceType::Wood);
                        let stone = fortress.resources.get(FortressResourceType::Stone);

                        let ew = cost.wood <= wood_c.current;
                        let es = cost.stone <= stone.current;

                        if ew && es && cost.silver <= gs.character.silver {
                            return Some(Command::FortressUpgradeUnit { unit: unit_type });
                        }
                    }
                }
            }
        }
    }

    let is_gem_searching = fortress.gem_search.finish.is_some();

    if fortress.building_upgrade.target.is_none() {
        let mut buildable = Vec::new();

        for building_type in FortressBuildingType::iter() {
            if fortress.can_build(building_type, gs.character.silver) {
                let building = fortress.buildings.get(building_type);

                if building.level < fortress.building_max_lvl as u16 {
                    buildable.push((building_type, building.level));
                }
            }
        }

        if let Some((best_building, _)) = buildable.into_iter().min_by_key(|&(_, lvl)| lvl) {
            return Some(Command::FortressBuild { f_type: best_building });
        }
    }

    let is_gem_upgrading = fortress.building_upgrade.target == Some(FortressBuildingType::GemMine);

    if gem_mine.level > 0 && !is_gem_searching && !is_gem_upgrading {
        let cost = fortress.gem_search.cost;

        let wood_c = fortress.resources.get(FortressResourceType::Wood);
        let stone = fortress.resources.get(FortressResourceType::Stone);

        let ew = cost.wood <= wood_c.current;
        let es = cost.stone <= stone.current;

        let eg = cost.silver <= gs.character.silver;

        if ew && es && eg {
            return Some(Command::FortressGemStoneSearch);
        }
    }

    None
}

pub async fn fortress(session: &mut SimpleSession) {
    if let Some(gs) = session.game_state() {
        if gs.character.level < 25 {
            return;
        }

        let needs_initialization = match &gs.fortress {
            Some(fortress) => fortress.building_max_lvl == 0,

            None => true,
        };

        if needs_initialization {
            log(session, "FORTRESS UNLOCKED - INITIALIZING STATE");

            let cmd = Command::FortressBuild { f_type: FortressBuildingType::Fortress };

            let _ = session.send_command(cmd).await;

            return;
        }
    }

    while let Some(cmd) = fortress_next(session) {
        match &cmd {
            Command::FortressGather { resource } => {
                log(session, &format!("GATHERING '{:?}' FROM FORTRESS", resource));
            }

            Command::FortressBuild { f_type } => {
                log(session, &format!("STARTING BUILDING '{:?}' IN FORTRESS", f_type));
            }

            Command::FortressBuildUnit { unit, count } => {
                log(session, &format!("TRAINING {} '{:?}' IN FORTRESS", count, unit));
            }

            Command::FortressUpgradeUnit { unit } => {
                log(session, &format!("UPGRADING UNIT '{:?}' IN SMITHY", unit));
            }

            Command::FortressGemStoneSearch => {
                log(session, "STARTING GEM SEARCH IN GEM MINE");
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("FORTRESS SEND COMMAND ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}

fn underworld_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let Some(ref underworld) = gs.underworld else {
        return None;
    };

    if let Some(target) = underworld.upgrade_building {
        if let Some(finish) = underworld.upgrade_finish {
            if Local::now() >= finish {
                return Some(Command::UnderworldUpgradeFinish { building: target, mushrooms: 0 });
            }
        }
    }

    let get_col = |resource_type: UnderworldResourceType| {
        let resource = underworld.production.get(resource_type);

        let Some(lu) = underworld.last_collectable_update else {
            return resource.last_collectable;
        };

        let seconds = (Local::now() - lu).num_seconds().max(0) as u64;

        let mut produce = (seconds * resource.per_hour) / 3600;

        if resource_type == UnderworldResourceType::ThirstForAdventure {
            produce /= 24;
        }

        if resource.limit > 0 {
            return (resource.last_collectable + produce).min(resource.limit);
        }

        resource.last_collectable + produce
    };

    let is_startup = {
        let mut startup_set = UW_COLLECTED_ON_STARTUP.lock().unwrap();

        startup_set.get_or_insert_with(HashSet::new).insert(gs.character.name.clone())
    };

    let is_collect_time = underworld.last_collectable_update.map_or(true, |lu| {
        let (interval, window) = (chrono::Duration::minutes(30), chrono::Duration::minutes(2));

        let time_since_update = Local::now() - lu;

        time_since_update >= interval || time_since_update < window
    });

    if is_startup || is_collect_time {
        let souls = underworld.production.get(UnderworldResourceType::Souls);

        let can_s = underworld.upgrade_building != Some(UnderworldBuildingType::SoulExtractor);

        let se = get_col(UnderworldResourceType::Souls) >= souls.limit / 2;

        if se && souls.limit > 0 && underworld.souls_current < underworld.souls_limit && can_s {
            return Some(Command::UnderworldCollect { resource: UnderworldResourceType::Souls });
        }

        let silver = underworld.production.get(UnderworldResourceType::Silver);

        let can_silver = underworld.upgrade_building != Some(UnderworldBuildingType::GoldPit);

        let silver_enough = get_col(UnderworldResourceType::Silver) >= silver.limit / 2;

        if silver_enough && silver.limit > 0 && can_silver {
            return Some(Command::UnderworldCollect { resource: UnderworldResourceType::Silver });
        }

        let thirst = underworld.production.get(UnderworldResourceType::ThirstForAdventure);

        let can_t = underworld.upgrade_building != Some(UnderworldBuildingType::Adventuromatic);

        let thirst_enough = get_col(UnderworldResourceType::ThirstForAdventure) >= thirst.limit / 2;

        if thirst_enough && thirst.limit > 0 && can_t {
            let toa = UnderworldResourceType::ThirstForAdventure;

            return Some(Command::UnderworldCollect { resource: toa });
        }
    }

    if underworld.upgrade_building.is_none() {
        let mut buildable = Vec::new();

        let hod_level = underworld.buildings.get(UnderworldBuildingType::HeartOfDarkness).level;

        for building_type in UnderworldBuildingType::iter() {
            let building = underworld.buildings.get(building_type);

            let is_hod = building_type == UnderworldBuildingType::HeartOfDarkness;

            let mut can_upgrade = building.level < hod_level && building.level < 15;

            if is_hod {
                can_upgrade = hod_level < 15;
            }

            let cost = building.upgrade_cost;

            if can_upgrade && (cost.silver > 0 || cost.souls > 0) {
                if cost.silver <= gs.character.silver && cost.souls <= underworld.souls_current {
                    buildable.push((building_type, building.level));
                }
            }
        }

        if let Some((best_building, _)) = buildable.into_iter().min_by_key(|&(_, lvl)| lvl) {
            return Some(Command::UnderworldUpgradeStart { building: best_building, mushrooms: 0 });
        }
    }

    if underworld.lured_today < 5 {
        let goblin_level = underworld.units.get(UnderworldUnitType::Goblin).level as f64;

        if let Some(sugg) = underworld.lure_suggestion {
            if let Some(hof_player) = gs.hall_of_fames.players.first() {
                if let Some(other_player) = gs.lookup.lookup_name(&hof_player.name) {
                    let level = other_player.level as f64;

                    if goblin_level >= level * crate::constant::GOBLIN_LEVEL_HERO_RATIO {
                        let player_id = other_player.player_id;

                        return Some(Command::UnderworldAttack { player_id });
                    }

                    if goblin_level < level * crate::constant::GOBLIN_LEVEL_HERO_RATIO {
                        return None;
                    }
                }

                return Some(Command::ViewPlayer { ident: hof_player.name.clone() });
            }

            return Some(Command::ViewLureSuggestion { suggestion: sugg });
        }

        return Some(Command::UpdateLureSuggestion);
    }

    None
}

pub async fn underworld(session: &mut SimpleSession) {
    if let Some(gs) = session.game_state_mut() {
        if gs.underworld.is_none() {
            return;
        }

        gs.hall_of_fames.players.clear();
    }

    while let Some(cmd) = underworld_next(session) {
        match &cmd {
            Command::UnderworldCollect { resource } => {
                log(session, &format!("GATHERING '{:?}' FROM UNDERWORLD", resource));
            }

            Command::UnderworldUpgradeStart { building, .. } => {
                log(session, &format!("STARTING BUILDING '{:?}' IN UNDERWORLD", building));
            }

            Command::UnderworldAttack { .. } => {
                log(session, &format!("LURING HERO INTO UNDERWORLD"));
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("UNDERWORLD SEND COMMAND ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (3000.0, 1200.0, 1000.0, 7000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
