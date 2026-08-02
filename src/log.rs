use sf_api::{gamestate::character::Class, session::SimpleSession};

fn get_class_name(class: Class) -> &'static str {
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
    if let Some(gs) = session.game_state() {
        let (name, level, class) = {
            let (name, level) = (gs.character.name.clone(), gs.character.level);

            (name, level, get_class_name(gs.character.class))
        };

        return println!("Level {level} {class} ({name}): {message}");
    }

    println!("User {} (state unpopulated): {message}", session.username());
}
