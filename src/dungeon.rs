use std::sync::LazyLock;

use chrono::Local;
use strum::IntoEnumIterator;

use sf_api::{
    command::Command,
    gamestate::dungeons::{Dungeon, DungeonProgress, LightDungeon, ShadowDungeon},
    gamestate::unlockables::HellevatorStatus,
    session::SimpleSession,
};

use crate::config::CONFIG;
use crate::log::log;

static ENABLE_DUNGEON: LazyLock<bool> = LazyLock::new(|| CONFIG.features.enable_dungeon);
static ENABLE_HELLEVATOR: LazyLock<bool> = LazyLock::new(|| CONFIG.features.enable_hellevator);

fn dungeon_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    if *ENABLE_DUNGEON && gs.dungeons.portal.as_ref().is_some_and(|p| p.can_fight) {
        return Some(Command::FightPortal);
    }

    if *ENABLE_HELLEVATOR && gs.character.level >= 10 {
        match gs.hellevator.status() {
            HellevatorStatus::NotEntered => return Some(Command::HellevatorEnter),

            HellevatorStatus::Active(hellevator) if hellevator.key_cards > 0 => {
                return Some(Command::HellevatorFight { use_mushroom: false });
            }

            _ => {}
        }
    }

    if !*ENABLE_DUNGEON {
        return None;
    }

    if let Some(next_fight) = gs.dungeons.next_free_fight {
        if Local::now() < next_fight + chrono::Duration::seconds(5) {
            return None;
        }
    }

    let mut best: Option<(Dungeon, u16)> = None;

    for l in LightDungeon::iter() {
        if let Some(current) = gs.dungeons.current_enemy(l) {
            match best {
                Some((_, best_level)) => {
                    if current.level < best_level {
                        best = Some((Dungeon::Light(l), current.level));
                    }
                }

                None => best = Some((Dungeon::Light(l), current.level)),
            }
        }
    }

    for s in ShadowDungeon::iter() {
        if let Some(current) = gs.dungeons.current_enemy(s) {
            match best {
                Some((_, best_level)) => {
                    if current.level < best_level {
                        best = Some((Dungeon::Shadow(s), current.level));
                    }
                }

                None => best = Some((Dungeon::Shadow(s), current.level)),
            }
        }
    }

    if let Some((target_dungeon, _level)) = best {
        let cmd = match target_dungeon {
            Dungeon::Light(LightDungeon::Tower) => {
                let current_level = match gs.dungeons.progress(LightDungeon::Tower) {
                    DungeonProgress::Open { finished } => finished as u8 + 1,

                    _ => 1,
                };

                Command::FightTower { current_level, use_mush: false }
            }

            d => Command::FightDungeon { dungeon: d, use_mushroom: false },
        };

        return Some(cmd);
    }

    return None;
}

pub async fn dungeon(session: &mut SimpleSession) {
    if let Some(gs) = session.game_state() {
        let pcf = *ENABLE_DUNGEON && gs.dungeons.portal.as_ref().is_some_and(|p| p.can_fight);

        let hcf = *ENABLE_HELLEVATOR
            && gs.character.level >= 10
            && match gs.hellevator.status() {
                HellevatorStatus::NotEntered => true,

                HellevatorStatus::Active(h) => h.key_cards > 0,

                _ => false,
            };

        if !*ENABLE_DUNGEON && !hcf {
            return;
        }

        if let Some(next_fight) = gs.dungeons.next_free_fight {
            let not_portal_or_hell = !pcf && !hcf;

            let is_time = Local::now() < next_fight + chrono::Duration::seconds(5);

            if *ENABLE_DUNGEON && is_time && not_portal_or_hell {
                return;
            }
        }
    }

    if *ENABLE_DUNGEON {
        if let Err(err) = session.send_command(Command::UpdateDungeons).await {
            log(session, &format!("FAILED TO UPDATE DUNGEONS ({:?})", err));

            return;
        }
    }

    while let Some(cmd) = dungeon_next(session) {
        match &cmd {
            Command::FightPortal => {
                log(session, "FIGHTING PORTAL");
            }

            Command::FightTower { current_level, .. } => {
                log(session, &format!("FIGHTING TOWER LEVEL {current_level}"));
            }

            Command::FightDungeon { dungeon, .. } => {
                log(session, &format!("FIGHTING IN '{dungeon:?}' DUNGEON"));
            }

            Command::HellevatorEnter => {
                log(session, "ENTERING HELLEVATOR");
            }

            Command::HellevatorFight { .. } => {
                log(session, "FIGHTING IN HELLEVATOR");
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("DUNGEON SEND COMMAND ERROR ({:?})", err));

            break;
        }

        crate::wait_between_actions(3200.0, 1200.0, 1200.0, 7000.0).await;

        if *ENABLE_DUNGEON {
            if let Err(err) = session.send_command(Command::UpdateDungeons).await {
                log(session, &format!("FAILED TO UPDATE DUNGEONS ({:?})", err));

                break;
            }
        }
    }
}
