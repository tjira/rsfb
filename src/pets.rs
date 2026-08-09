use std::time::Duration;

use chrono::Local;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    gamestate::unlockables::{HabitatExploration, HabitatType, Pet},
    session::SimpleSession,
};

use crate::log::log;

fn pets_next(session: &SimpleSession) -> Option<(Command, String)> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let Some(pets) = gs.pets.as_ref() else {
        return None;
    };

    let total_fruit_count = pets.habitats.values().map(|h| h.fruits as u32).sum::<u32>();

    if total_fruit_count > 0 {
        let mut best_pet: Option<&Pet> = None;

        for (_, habitat) in &pets.habitats {
            if habitat.fruits == 0 {
                continue;
            }

            for pet in &habitat.pets {
                if pet.level > 0 && pet.level < 100 && pet.fruits_today < 3 {
                    match best_pet {
                        Some(b) => {
                            if pet.level > b.level {
                                best_pet = Some(pet);
                            }
                        }

                        None => best_pet = Some(pet),
                    }
                }
            }
        }

        if let Some(pet_to_feed) = best_pet {
            let msg = format!("FEEDING PETS");

            let cmd = Command::PetFeed { pet_id: pet_to_feed.id, total_fruit_count };

            return Some((cmd, msg));
        }
    }

    let mut can_attack = true;

    if let Some(next_free) = pets.next_free_exploration {
        if Local::now() < next_free + chrono::Duration::seconds(5) {
            can_attack = false;
        }
    }

    if can_attack {
        let mut best: Option<(u16, u32, HabitatType, &Pet)> = None;

        for (habitat_type, habitat) in &pets.habitats {
            let (exp, p) = (&habitat.exploration, &habitat.pets);

            if let HabitatExploration::Exploring { fights_won, next_fight_lvl } = exp {
                if let Some(pet) = p.iter().filter(|p| p.level > 0).max_by_key(|p| p.level) {
                    if best.map_or(true, |(best_lvl, _, _, _)| *next_fight_lvl < best_lvl) {
                        best = Some((*next_fight_lvl, *fights_won + 1, habitat_type, pet));
                    }
                }
            }
        }

        if let Some((_, enemy_pos, habitat_type, pet)) = best {
            let msg = format!("ATTACKING '{:?}' HABITAT", habitat_type);

            let (use_mush, habitat, player_pet_id) = (false, habitat_type, pet.id);

            let cmd = Command::FightPetDungeon { use_mush, habitat, enemy_pos, player_pet_id };

            return Some((cmd, msg));
        }
    }

    None
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2800.0, 1000.0, 1000.0, 6000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}

pub async fn pets(session: &mut SimpleSession) {
    loop {
        let Some((cmd, msg)) = pets_next(session) else {
            break;
        };

        log(session, &msg);

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("PETS SEND COMMAND ERROR ({:?})", err));

            break;
        }

        wait_between_actions().await;
    }
}
