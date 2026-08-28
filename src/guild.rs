use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    gamestate::NormalCost,
    gamestate::guild::{BattlesJoined, GuildSkill},
    session::SimpleSession,
};

use crate::log::log;

fn can_afford_guild_skill(cost: NormalCost, silver: u64, mushrooms: u32) -> bool {
    if cost.silver > silver {
        return false;
    }

    let mm = crate::constant::MIN_MUSHROOM_RESERVE;

    if cost.mushrooms > 0 {
        let cost_mushrooms = cost.mushrooms as u32;

        if mushrooms.saturating_sub(cost_mushrooms) < mm {
            return false;
        }

        let gum = crate::constant::GUILD_UPGRADE_MAX_MUSHROOM_RATIO;

        if (cost.mushrooms as f64) >= (mushrooms as f64) * gum {
            return false;
        }
    }

    true
}

fn can_afford_both_guild_skills(c1: NormalCost, c2: NormalCost, silver: u64, mush: u32) -> bool {
    if !can_afford_guild_skill(c1, silver, mush) {
        return false;
    }

    let remaining_silver = silver.saturating_sub(c1.silver);
    let rema_mus = mush.saturating_sub(c1.mushrooms as u32);

    can_afford_guild_skill(c2, remaining_silver, rema_mus)
}

fn guild_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let Some(guild) = &gs.guild else {
        return None;
    };

    let (ins, trs, pet) = (GuildSkill::Instructor, GuildSkill::Treasure, GuildSkill::Pet);

    let candidates = match guild.own_treasure_skill.cmp(&guild.own_instructor_skill) {
        std::cmp::Ordering::Less => vec![trs, pet],
        std::cmp::Ordering::Greater => vec![ins, pet],
        std::cmp::Ordering::Equal => {
            let (ci, ct) = (guild.upgrade_price[ins], guild.upgrade_price[trs]);
            let (silver, mushr) = (gs.character.silver, gs.character.mushrooms);

            if can_afford_both_guild_skills(ci, ct, silver, mushr) {
                vec![ins, pet]
            } else {
                vec![pet]
            }
        }
    };

    for skill in candidates {
        let can_pet = guild.pet_max_lvl > 0 && guild.own_pet_lvl < guild.pet_max_lvl;

        if skill == GuildSkill::Pet && !can_pet {
            continue;
        }

        let cost = guild.upgrade_price[skill];

        if can_afford_guild_skill(cost, gs.character.silver, gs.character.mushrooms) {
            let current = match skill {
                GuildSkill::Treasure => guild.own_treasure_skill,
                GuildSkill::Instructor => guild.own_instructor_skill,
                GuildSkill::Pet => guild.own_pet_lvl,
            };

            return Some(Command::GuildIncreaseSkill { skill, current });
        }
    }

    if let Some(joined) = guild.joined {
        if Local::now() - joined <= chrono::Duration::hours(24) {
            return None;
        }
    }

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

            Command::GuildIncreaseSkill { skill, .. } => {
                log(session, &format!("UPGRADING '{:?}' GUILD SKILL", skill));
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
    let (mean, std, min, max): (f64, f64, f64, f64) = (3400.0, 1400.0, 1200.0, 7500.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
