use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use strum::IntoEnumIterator;

use sf_api::{
    command::Command,
    gamestate::fortress::{FortressBuildingType, FortressResourceType},
    misc::EnumMapGet,
    session::SimpleSession,
};

use crate::log::log;

static COLLECTED_ON_STARTUP: Mutex<Option<HashSet<String>>> = Mutex::new(None);

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
        let mut startup_set = COLLECTED_ON_STARTUP.lock().unwrap();

        startup_set.get_or_insert_with(HashSet::new).insert(gs.character.name.clone())
    };

    let is_collect_time = fortress.last_collectable_updated.map_or(true, |lu| {
        let (interval, window) = (chrono::Duration::minutes(30), chrono::Duration::minutes(2));

        let time_since_update = Local::now() - lu;

        time_since_update >= interval || time_since_update < window
    });

    if is_startup || is_collect_time {
        let wood = fortress.resources.get(FortressResourceType::Wood);

        if get_collectable(wood) > 0 && wood.current < wood.limit {
            return Some(Command::FortressGather { resource: FortressResourceType::Wood });
        }

        let stone = fortress.resources.get(FortressResourceType::Stone);

        if get_collectable(stone) > 0 && stone.current < stone.limit {
            return Some(Command::FortressGather { resource: FortressResourceType::Stone });
        }

        let exp = fortress.resources.get(FortressResourceType::Experience);

        if get_collectable(exp) > 0 {
            return Some(Command::FortressGather { resource: FortressResourceType::Experience });
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
            Command::FortressBuildFinish { f_type, .. } => {
                log(session, &format!("FINISHING '{:?}' BUILDING IN FORTRESS", f_type));
            }

            Command::FortressGather { resource } => {
                log(session, &format!("GATHERING '{:?}' FROM FORTRESS", resource));
            }

            Command::FortressBuild { f_type } => {
                log(session, &format!("STARTING BUILDING '{:?}' IN FORTRESS", f_type));
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

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (3000.0, 1200.0, 1000.0, 7000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
