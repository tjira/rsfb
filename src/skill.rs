use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::{AttributeType, Command},
    session::SimpleSession,
};

use crate::log::log;

fn skill_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let character = &gs.character;

    let (main_attr, mut others) = (gs.character.class.main_attribute(), Vec::new());
    let (strength, dexterity) = (AttributeType::Strength, AttributeType::Dexterity);

    let (constitut, luck) = (AttributeType::Constitution, AttributeType::Luck);

    for attr in [strength, dexterity, AttributeType::Intelligence, constitut, luck] {
        if attr != main_attr && attr != constitut && attr != luck {
            others.push(attr);
        }
    }

    let other1 = others[0];
    let other2 = others[1];

    let scores = [
        (main_attr, character.attribute_basis[main_attr] as f64 / 100.0),
        (constitut, character.attribute_basis[constitut] as f64 / 080.0),

        (other1, character.attribute_basis[other1] as f64 / 10.0),
        (other2, character.attribute_basis[other2] as f64 / 10.0),

        (luck, character.attribute_basis[luck] as f64 / 40.0),
    ];

    let (mut best_attr, mut min_score) = (main_attr, f64::MAX);

    for (attr, score) in scores {
        if score < min_score {
            (best_attr, min_score) = (attr, score);
        }
    }

    let mut max_shop_price = 0;

    for shop in gs.shops.values() {
        for item in &shop.items {
            if item.price != u32::MAX && (item.price as u64) > max_shop_price {
                max_shop_price = item.price as u64;
            }
        }
    }

    if character.silver > 10 * max_shop_price {
        let next_value = character.attribute_basis[best_attr] + 1;

        return Some(Command::UpgradeSkill { attribute: best_attr, next_attribute: next_value });
    }

    None
}

pub async fn skill(session: &mut SimpleSession) {
    while let Some(cmd) = skill_next(session) {
        if let Command::UpgradeSkill { attribute, next_attribute } = &cmd {
            log(session, &format!("UPGRADING ATTRIBUTE '{:?}' TO {}", attribute, next_attribute));
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("SKILL SEND COMMAND ERROR ({:?})", err));
            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (500.0, 100.0, 250.0, 750.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
