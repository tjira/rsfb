use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{command::Command, gamestate::character::Mount, session::SimpleSession};

use crate::log::log;

fn mount_next(session: &SimpleSession) -> Option<Command> {
    let Some(gs) = session.game_state() else {
        return None;
    };

    let now = chrono::Local::now();

    let active_mount = gs.character.mount.and_then(|m| match gs.character.mount_end {
        Some(end) if end <= now => None,

        _ => Some(m),
    });

    match active_mount {
        None => {
            if gs.character.mushrooms >= 25 {
                return Some(Command::BuyMount { mount: Mount::Dragon });
            }

            if gs.character.mushrooms >= 1 && gs.character.silver >= 1000 {
                return Some(Command::BuyMount { mount: Mount::Tiger });
            }

            None
        }

        Some(Mount::Tiger) => {
            if gs.character.mushrooms >= 25 {
                return Some(Command::BuyMount { mount: Mount::Dragon });
            }

            None
        }
        _ => None,
    }
}

pub async fn mount(session: &mut SimpleSession) {
    while let Some(cmd) = mount_next(session) {
        match &cmd {
            Command::BuyMount { mount } => {
                log(session, &format!("BUYING '{:?}' MOUNT", mount));
            }

            _ => {}
        }

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("MOUNT SEND COMMAND ERROR: {:?}", err));

            break;
        }

        wait_between_actions().await;
    }
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2000.0, 500.0, 1000.0, 4000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}
