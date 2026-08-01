use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{
    command::Command,
    error::SFError,
    gamestate::tavern::{AvailableTasks, CurrentAction, ExpeditionStage},
    session::SimpleSession,
};

use crate::log::log;

fn expedition_next(session: &mut SimpleSession) -> Result<Option<Command>, SFError> {
    let gs = session.game_state().ok_or_else(|| SFError::InvalidRequest("GAME STATE IS NOT POPULATED"))?;

    let exp = &gs.tavern.expeditions;

    if let Some(stage) = exp.active().map(|a| a.current_stage()) {
        match stage {
            ExpeditionStage::Boss(_) => {
                log(session, "FIGHTING BOSS IN EXPEDITION")?;

                return Ok(Some(Command::ExpeditionContinue));
            }

            ExpeditionStage::Rewards(_) => {
                log(session, "PICKING REWARD IN EXPEDITION")?;

                return Ok(Some(Command::ExpeditionPickReward { pos: 0 }));
            }

            ExpeditionStage::Encounters(encounters) if !encounters.is_empty() => {
                log(session, "PICKING ENCOUNTER IN EXPEDITION")?;

                return Ok(Some(Command::ExpeditionPickEncounter { pos: 0 }));
            }

            ExpeditionStage::Waiting { busy_until, .. } => {
                let now = chrono::Local::now();

                let total_secs = busy_until.signed_duration_since(now).num_seconds().max(0);

                let mins = total_secs / 60;
                let secs = total_secs % 60;

                log(session, &format!("EXPEDITION IN PROGRESS, BUSY FOR {mins}M {secs}S"))?;

                return Ok(None);
            }

            _ => return Err(SFError::InvalidRequest("UNHANDLED EXPEDITION STAGE"))
        }
    }

    match gs.tavern.current_action {
        CurrentAction::Expedition => {
            log(session, "FINISHING EXPEDITION")?;

            return Ok(Some(Command::ExpeditionContinue));
        }

        CurrentAction::Idle => {
            log(session, "READY TO START NEW EXPEDITION")?;
        }

        _ => return Ok(None)
    }

    let AvailableTasks::Expeditions(tasks) = gs.tavern.available_tasks() else {
        log(session, "NO EXPEDITIONS AVAILABLE")?;

        return Ok(None);
    };

    let Some(task) = tasks.first() else {
        log(session, "EXPEDITION LIST IS EMPTY")?;

        return Ok(None);
    };

    let (cost, current_thirst) = (task.thirst_for_adventure_sec, gs.tavern.thirst_for_adventure_sec);

    if cost <= current_thirst {
        let mins = cost / 60;

        log(session, &format!("STARTING NEW EXPEDITION FOR {mins} THIRST FOR ADVENTURE"))?;

        return Ok(Some(Command::ExpeditionStart { pos: 0 }))
    }

    log(session, "NOT ENOUGH THIRST FOR ADVENTURE")?;

    return Ok(None)
}

pub async fn expedition(session: &mut SimpleSession) -> Result<(), SFError> {
    while let Some(cmd) = expedition_next(session)? {
        session.send_command(cmd).await?;

        wait_between_actions().await;
    }

    Ok(())
}

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2000.0, 1000.0, 500.0, 3500.0);

    let wait_time = Normal::new(mean, std).unwrap().sample(&mut thread_rng()).clamp(min, max) as u64;

    tokio::time::sleep(std::time::Duration::from_millis(wait_time)).await;
}
