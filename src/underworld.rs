use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use chrono::Local;
use strum::IntoEnumIterator;

use sf_api::{
    command::Command,
    gamestate::underworld::{LureSuggestion, UnderworldBuildingType},
    gamestate::underworld::{UnderworldResourceType, UnderworldUnitType},
    misc::EnumMapGet,
    session::SimpleSession,
};

use crate::config::CONFIG;
use crate::log::log;

static UW_COLLECTED_ON_STARTUP: Mutex<Option<HashSet<String>>> = Mutex::new(None);

static LAST_LURE_CHECK: Mutex<Option<HashMap<String, (LureSuggestion, u32)>>> = Mutex::new(None);

static HARVEST_CHECK_MINS: LazyLock<i64> = LazyLock::new(|| CONFIG.fortress.harvest_check_mins);
static HARVEST_RATIO: LazyLock<f64> = LazyLock::new(|| CONFIG.fortress.harvest_ratio);
static GOBLIN_HERO_RATIO: LazyLock<f64> = LazyLock::new(|| CONFIG.fortress.goblin_hero_ratio);

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
        let t1 = chrono::Duration::minutes(*HARVEST_CHECK_MINS);

        let (interval, window) = (t1, chrono::Duration::minutes(2));

        let time_since_update = Local::now() - lu;

        time_since_update >= interval || time_since_update < window
    });

    let ratio = *HARVEST_RATIO;

    let (sl, th) = (UnderworldResourceType::Silver, UnderworldResourceType::ThirstForAdventure);

    if is_startup || is_collect_time {
        let souls = underworld.production.get(UnderworldResourceType::Souls);

        let can_s = underworld.upgrade_building != Some(UnderworldBuildingType::SoulExtractor);

        let se = (get_col(UnderworldResourceType::Souls) as f64) >= (souls.limit as f64) * ratio;

        if se && souls.limit > 0 && underworld.souls_current < underworld.souls_limit && can_s {
            return Some(Command::UnderworldCollect { resource: UnderworldResourceType::Souls });
        }

        let silver = underworld.production.get(UnderworldResourceType::Silver);

        let can_silver = underworld.upgrade_building != Some(UnderworldBuildingType::GoldPit);

        let silver_enough = (get_col(sl) as f64) >= (silver.limit as f64) * ratio;

        if silver_enough && silver.limit > 0 && can_silver {
            return Some(Command::UnderworldCollect { resource: UnderworldResourceType::Silver });
        }

        let thirst = underworld.production.get(UnderworldResourceType::ThirstForAdventure);

        let can_t = underworld.upgrade_building != Some(UnderworldBuildingType::Adventuromatic);

        let thirst_enough = (get_col(th) as f64) >= (thirst.limit as f64) * ratio;

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
            let already_checked = {
                let last_check = LAST_LURE_CHECK.lock().unwrap();

                let mut is_valid = false;

                if let Some(m) = &*last_check {
                    if let Some(&(id, lvl)) = m.get(&gs.character.name) {
                        is_valid = (id == sugg) && (lvl == goblin_level as u32);
                    }
                }

                is_valid
            };

            if already_checked {
                return None;
            }

            if let Some(hof_player) = gs.hall_of_fames.players.first() {
                if let Some(other_player) = gs.lookup.lookup_name(&hof_player.name) {
                    let level = other_player.level as f64;

                    if goblin_level >= level * *GOBLIN_HERO_RATIO {
                        let player_id = other_player.player_id;

                        return Some(Command::UnderworldAttack { player_id });
                    }

                    if goblin_level < level * *GOBLIN_HERO_RATIO {
                        let mut last_check = LAST_LURE_CHECK.lock().unwrap();

                        let (name, pair) = (gs.character.name.clone(), (sugg, goblin_level as u32));

                        last_check.get_or_insert_with(HashMap::new).insert(name, pair);

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
        let is_attack = matches!(&cmd, Command::UnderworldAttack { .. });

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

        if is_attack {
            if let Some(gs) = session.game_state_mut() {
                gs.hall_of_fames.players.clear();
            }
        }

        crate::wait_between_actions(3000.0, 1200.0, 1000.0, 7000.0).await;
    }
}
