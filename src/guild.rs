use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use sf_api::{command::Command, gamestate::guild::BattlesJoined, session::SimpleSession};

use crate::log::log;

fn guild_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let Some(guild) = &gs.guild else {
        return None;
    };

    let own_member = guild.members.iter().find(|m| m.name == gs.character.name);

    if let Some(defending) = &guild.defending {
        if Local::now() < defending.date {
            let joined_defense = own_member.as_ref().map_or(false, |m| {
                matches!(m.battles_joined, Some(BattlesJoined::Defense | BattlesJoined::Both))
            });

            if !joined_defense {
                return Some(Command::GuildJoinDefense);
            }
        }
    }

    if let Some(attacking) = &guild.attacking {
        if Local::now() < attacking.date {
            let joined_attack = own_member.as_ref().map_or(false, |m| {
                matches!(m.battles_joined, Some(BattlesJoined::Attack | BattlesJoined::Both))
            });

            if !joined_attack {
                return Some(Command::GuildJoinAttack);
            }
        }
    }

    if guild.hydra.remaining_fights > 0 && guild.hydra.current_life > 0 {
        let can_fight_hydra = guild.hydra.next_battle.map_or(true, |next| Local::now() >= next);

        if can_fight_hydra {
            return Some(Command::GuildPetBattle { use_mushroom: false });
        }
    }

    if gs.character.level >= 99 && guild.portal.life_percentage > 0 {
        let fought_today = own_member.as_ref().map_or(true, |m| {
            let date = Local::now().date_naive();

            m.portal_fought.map(|fought| fought.date_naive() >= date).unwrap_or(false)
        });

        if !fought_today {
            return Some(Command::GuildPortalBattle);
        }
    }

    None
}

pub async fn guild(session: &mut SimpleSession) {
    while let Some(cmd) = guild_next(session) {
        match &cmd {
            Command::GuildJoinDefense => {
                log(session, "JOINING GUILD DEFENSE");
            }

            Command::GuildJoinAttack => {
                log(session, "JOINING GUILD ATTACK");
            }

            Command::GuildPetBattle { .. } => {
                log(session, "FIGHTING GUILD HYDRA");
            }

            Command::GuildPortalBattle => {
                log(session, "FIGHTING GUILD PORTAL");
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("GUILD SEND COMMAND ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2000.0, 1000.0, 500.0, 3500.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
