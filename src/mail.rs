use sf_api::{command::Command, gamestate::social::MessageType, session::SimpleSession};

use crate::log::log;

pub async fn mail(session: &mut SimpleSession) {
    let Some(gs) = session.game_state() else {
        return;
    };

    let mut unread_messages = Vec::new();

    for msg in &gs.mail.inbox {
        if !msg.read && matches!(msg.msg_typ, MessageType::Normal) {
            unread_messages.push((msg.msg_id, msg.from.clone(), msg.title.clone()));
        }
    }

    for (msg_id, from, title) in unread_messages {
        let display_from = if crate::log::is_hidden() { "****" } else { &from };
        let display_tit = if crate::log::is_hidden() { "****" } else { &title };

        log(session, &format!("READING MESSAGE FROM '{display_from}' ({display_tit})"));

        if let Err(err) = session.send_command(Command::MessageOpen { msg_id }).await {
            log(session, &format!("FAILED TO READ MESSAGE ({:?})", err));
        }

        crate::wait_between_actions(2800.0, 1000.0, 1000.0, 6000.0).await;
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

        crate::wait_between_actions(2800.0, 1000.0, 1000.0, 6000.0).await;
    }
}
