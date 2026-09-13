use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use chrono::Local;
use strum::IntoEnumIterator;

use sf_api::{
    command::Command,
    gamestate::fortress::{FortressBuildingType, FortressResourceType, FortressUnitType},
    misc::EnumMapGet,
    session::SimpleSession,
};

use crate::config::CONFIG;
use crate::log::log;

static FS_COLLECTED_ON_STARTUP: Mutex<Option<HashSet<String>>> = Mutex::new(None);

static HARVEST_CHECK_MINS: LazyLock<i64> = LazyLock::new(|| CONFIG.fortress.harvest_check_mins);
static HARVEST_RATIO: LazyLock<f64> = LazyLock::new(|| CONFIG.fortress.harvest_ratio);

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
        let t1 = chrono::Duration::minutes(*HARVEST_CHECK_MINS);

        let (interval, window) = (t1, chrono::Duration::minutes(2));

        let time_since_update = Local::now() - lu;

        time_since_update >= interval || time_since_update < window
    });

    let ratio = *HARVEST_RATIO;

    if is_startup || is_collect_time {
        let wood = fortress.resources.get(FortressResourceType::Wood);

        let can_w = fortress.building_upgrade.target != Some(FortressBuildingType::WoodcuttersHut);

        let w_enough = (get_collectable(wood) as f64) >= (wood.production.limit as f64) * ratio;

        if w_enough && wood.production.limit > 0 && wood.current < wood.limit && can_w {
            return Some(Command::FortressGather { resource: FortressResourceType::Wood });
        }

        let stone = fortress.resources.get(FortressResourceType::Stone);

        let can_stone = fortress.building_upgrade.target != Some(FortressBuildingType::Quarry);

        let s_enough = (get_collectable(stone) as f64) >= (stone.production.limit as f64) * ratio;

        if s_enough && stone.production.limit > 0 && stone.current < stone.limit && can_stone {
            return Some(Command::FortressGather { resource: FortressResourceType::Stone });
        }

        let exp = fortress.resources.get(FortressResourceType::Experience);

        let can_exp = fortress.building_upgrade.target != Some(FortressBuildingType::Academy);

        let e_enough = (get_collectable(exp) as f64) >= (exp.production.limit as f64) * ratio;

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

    let fortress_level = fortress.buildings.get(FortressBuildingType::Fortress).level;

    let (hok_cost, hkl) = (fortress.hall_of_knights_upgrade_price, fortress.hall_of_knights_level);

    let cond = hkl < fortress.building_max_lvl as u16 && (hok_cost.wood > 0 || hok_cost.stone > 0);

    if gs.guild.is_some() && hkl < fortress_level && cond {
        let wood_c = fortress.resources.get(FortressResourceType::Wood);
        let stone = fortress.resources.get(FortressResourceType::Stone);

        let ew = hok_cost.wood <= wood_c.current;
        let es = hok_cost.stone <= stone.current;

        if ew && es && hok_cost.silver <= gs.character.silver {
            return Some(Command::FortressUpgradeHallOfKnights);
        }
    }

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

    if gem_mine.level > 0 && !fortress.gem_search.finish.is_some() && !is_gem_upgrading {
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
            log(session, "BUILDING FORTRESS FOR THE FIRST TIME");

            let cmd = Command::FortressBuild { f_type: FortressBuildingType::Fortress };

            if let Err(err) = session.send_command(cmd).await {
                log(session, &format!("FORTRESS INITIALIZATION ERROR ({:?})", err));
            }

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

            Command::FortressUpgradeHallOfKnights => {
                log(session, "UPGRADING HALL OF KNIGHTS IN FORTRESS");
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

        crate::wait_between_actions(3000.0, 1200.0, 1000.0, 7000.0).await;
    }
}
