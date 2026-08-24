use std::time::Duration;

use rand::thread_rng;
use rand_distr::{Distribution, Normal};

use sf_api::{command::Command, session::SimpleSession};

use crate::log::log;

async fn wait_between_actions() {
    let (mean, std, min, max): (f64, f64, f64, f64) = (2800.0, 1000.0, 1000.0, 6000.0);

    let number = Normal::new(mean, std).unwrap().sample(&mut thread_rng());

    tokio::time::sleep(Duration::from_millis(number.clamp(min, max) as u64)).await;
}

pub async fn mail(session: &mut SimpleSession) {
    let Some(gs) = session.game_state() else {
        return;
    };

    let mut unread_messages = Vec::new();

    for msg in &gs.mail.inbox {
        if !msg.read {
            unread_messages.push((msg.msg_id, msg.from.clone(), msg.title.clone()));
        }
    }

    for (msg_id, from, title) in unread_messages {
        let display_from = if crate::log::is_hidden() { "****" } else { &from };
        let display_tit = if crate::log::is_hidden() { "****" } else { &title };

        log(session, &format!("READING MESSAGE FROM '{display_from}' ({display_tit})"));

        let cmd = Command::Custom {
            cmd_name: "PlayerMessageView".to_string(),
            arguments: vec![msg_id.to_string()],
        };

        if let Err(err) = session.send_command(cmd).await {
            log(session, &format!("FAILED TO READ MESSAGE ({:?})", err));
        }

        wait_between_actions().await;
    }

    let Some(gs) = session.game_state() else {
        return;
    };

    let mut unread_news = Vec::new();

    for news in &gs.mail.news_inbox {
        if !news.read {
            unread_news.push((news.news_id, news.title.clone()));
        }
    }

    for (news_id, title) in unread_news {
        let display_title = if crate::log::is_hidden() { "****" } else { &title };

        log(session, &format!("READING NEWS '{display_title}'"));

        if let Err(err) = session.send_command(Command::PlayerNewsView { news_id }).await {
            log(session, &format!("FAILED TO READ NEWS ({:?})", err));
        }

        wait_between_actions().await;
    }
}
