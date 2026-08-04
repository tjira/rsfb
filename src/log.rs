use sf_api::{gamestate::character::Class, session::SimpleSession};

pub(crate) fn get_class_name(class: Class) -> &'static str {
    match class {
        Class::Assassin => "Assassin",
        Class::Bard => "Bard",
        Class::BattleMage => "Battle Mage",
        Class::Berserker => "Berserker",
        Class::DemonHunter => "Demon Hunter",
        Class::Druid => "Druid",
        Class::Mage => "Mage",
        Class::Necromancer => "Necromancer",
        Class::Paladin => "Paladin",
        Class::PlagueDoctor => "Plague Doctor",
        Class::Scout => "Scout",
        Class::Warrior => "Warrior",
    }
}

pub(crate) fn log(session: &SimpleSession, message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    if let Some(gs) = session.game_state() {
        let (name, level, class) = {
            let (name, level) = (gs.character.name.clone(), gs.character.level);

            (name, level, get_class_name(gs.character.class))
        };

        return println!("[{timestamp}] Level {level} {class} ({name}): {message}");
    }

    println!("[{timestamp}] USER '{}' (STATE UNPOPULATED): {message}", session.username());
}
