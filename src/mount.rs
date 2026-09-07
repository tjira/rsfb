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

    let current_tier = active_mount.map(|m| m as u8).unwrap_or(0);

    let candidates = [Mount::Dragon, Mount::Tiger, Mount::Horse, Mount::Cow];

    for mount in candidates {
        if (mount as u8) > current_tier {
            let cost = mount.cost();

            if gs.character.mushrooms >= cost.mushrooms as u32 && gs.character.silver >= cost.silver
            {
                return Some(Command::BuyMount { mount });
            }
        }
    }

    None
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

        crate::wait_between_actions(2000.0, 500.0, 1000.0, 4000.0).await;
    }
}
