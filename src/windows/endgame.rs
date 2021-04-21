use crate::{
    engine::{Display, TextMetrics},
    formula,
    game::{self, RunningState},
    keys::KeyCode,
    player::CauseOfDeath,
    settings::Settings,
    state::{Side, State},
    ui, window,
};

use egui::{self, Ui};

pub enum Action {
    NewGame,
    Help,
    Menu,
    Close,
}

pub fn process(
    state: &mut State,
    ui: &mut Ui,
    settings: &Settings,
    _metrics: &dyn TextMetrics,
    display: &Display,
    active: bool,
) -> RunningState {
    use CauseOfDeath::*;
    let cause_of_death = formula::cause_of_death(&state.player);
    let perpetrator = state.player.perpetrator.as_ref();
    let endgame_description = match (cause_of_death, perpetrator) {
        (Some(Exhausted), None) => "Exhausted".into(),
        (Some(Exhausted), Some(monster)) => format!("Exhausted because of {}", monster.name(),),
        (Some(Overdosed), _) => "Overdosed".into(),
        (Some(LostWill), Some(monster)) => format!("Lost all Will due to {}", monster.name(),),
        (Some(LostWill), None) => {
            log::error!("Lost all will without any apparent cause.");
            format!("Lost all will")
        }
        (Some(Killed), Some(monster)) => format!("Defeated by {}", monster.name()),
        (Some(Killed), None) => {
            // NOTE: this happens when the player kills itself using a cheat command.
            format!("Lost")
        }
        (None, _) => "".into(), // Victory
    };
    let endgame_reason_text = if state.side == Side::Victory {
        if !state.player.alive() {
            log::warn!("The player appears to be dead on victory screen.");
        }
        if cause_of_death.is_some() {
            log::warn!("The player has active cause of dead on victory screen.");
        }
        "You won!".into()
    } else {
        format!("You lost: {}", endgame_description)
    };

    let mut action = None;
    let mut window_is_open = true;

    let expected_window_width: f32 = 600.0;
    let expected_window_height: f32 = 400.0;
    let padding = 50.0;
    let max_size = [
        display.screen_size_px.x as f32 - padding,
        display.screen_size_px.y as f32 - padding,
    ];
    let window_size = [
        expected_window_width.min(max_size[0]),
        expected_window_height.min(max_size[1]),
    ];
    let window_pos_px = [
        (display.screen_size_px.x as f32 - window_size[0]) / 2.0,
        (display.screen_size_px.y as f32 - window_size[1]) / 2.0,
    ];

    egui::Window::new(&endgame_reason_text)
        .open(&mut window_is_open)
        .collapsible(false)
        .fixed_pos(window_pos_px)
        .fixed_size(window_size)
        .show(ui.ctx(), |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(format!("Turns: {}", state.turn));
                ui.label("");
                ui.label(format!(
                    "Longest High streak: {} turns",
                    state.player.longest_high_streak
                ));
                ui.label("");
                let carrying_doses_text = if state.player_picked_up_a_dose {
                    let doses_in_inventory = state
                        .player
                        .inventory
                        .iter()
                        .filter(|item| item.is_dose())
                        .count();
                    format!("Carrying {} doses", doses_in_inventory)
                } else {
                    "You've never managed to save a dose for a later fix.".to_string()
                };
                ui.label(carrying_doses_text);
                // Show some game tip, but not if the player just won
                if state.side != Side::Victory {
                    ui.label("");
                    ui.label(format!("Tip: {}", endgame_tip(state)));
                }

                ui.separator();
                ui.columns(3, |c| {
                    c[0].with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        if ui
                            .add(ui::button("[N]ew Game", active, &state.palette))
                            .clicked()
                        {
                            action = Some(Action::NewGame);
                        };
                    });
                    c[1].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if ui
                            .add(ui::button("[?] Help", active, &state.palette))
                            .clicked()
                        {
                            action = Some(Action::Help);
                        };
                    });
                    c[2].with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                        if ui
                            .add(ui::button("[Esc] Main Menu", active, &state.palette))
                            .clicked()
                        {
                            action = Some(Action::Menu);
                        };
                    });
                });
            });
        });

    if !window_is_open {
        action = Some(Action::Close)
    };

    if action.is_none() {
        if state.keys.matches_code(KeyCode::N) {
            action = Some(Action::NewGame);
        } else if state.keys.matches_code(KeyCode::Esc) {
            action = Some(Action::Menu);
        } else if state.keys.matches_code(KeyCode::QuestionMark)
            || state.keys.matches_code(KeyCode::H)
        {
            action = Some(Action::Help);
        }
    }

    if !active {
        action = None;
    }

    match action {
        Some(Action::NewGame) => RunningState::NewGame(Box::new(game::create_new_game_state(
            state,
            settings.challenge(),
        ))),
        Some(Action::Menu) => {
            state.window_stack.pop();
            state.window_stack.push(window::Window::MainMenu);
            RunningState::Running
        }
        Some(Action::Help) => {
            state.window_stack.push(window::Window::Help);
            RunningState::Running
        }
        Some(Action::Close) => {
            state.window_stack.pop();
            RunningState::Running
        }
        None => {
            if state.keys.get().is_some() || state.mouse.right_clicked {
                state.window_stack.pop();
            }
            RunningState::Running
        }
    }
}

fn endgame_tip(state: &State) -> String {
    use self::CauseOfDeath::*;

    if state.player_picked_up_a_dose == false {
        return String::from("Attack monsters by bumping (moving) into them!");
    }

    let throwavay_rng = &mut state.rng.clone();

    let overdosed_tips = &[
        "Using another dose when High will likely cause overdose early on.",
        "When you get too close to a Dose, it will be impossible to resist.",
        "The Cardinal, Diagonal and Strong Doses are much stronger. Early on, you'll likely overdose on them.",
    ];

    let food_tips = &["Eat Food (by pressing [1]) or use a Dose to stave off withdrawal."];

    let hunger_tips = &[
        "Being hit by Hunger will quickly get you into a withdrawal.",
        "The Hunger monsters can swarm you.",
    ];

    let anxiety_tips =
        &["Being hit by an Anxiety reduces your Will. You lose when it reaches zero."];

    let unsorted_tips = &[
        "As you use doses, you slowly build up tolerance.",
        "Even the doses of the same kind can have different strength. Their purity varies.",
        "Directly confronting Anxiety will slowly increase your Will.",
        "The other characters won't talk to you while you're High.",
        "Talking to another person sober will give you a bonus.",
        "The Depression monsters move twice as fast as you. Be careful.",
    ];

    let all_tips = overdosed_tips
        .iter()
        .chain(food_tips)
        .chain(hunger_tips)
        .chain(anxiety_tips)
        .chain(unsorted_tips)
        .collect::<Vec<_>>();

    let fallback = &"Losing a game is normal. Think about what happened and try again!";
    let cause_of_death = formula::cause_of_death(&state.player);
    let perpetrator = state.player.perpetrator.as_ref();
    let selected_tip = match (cause_of_death, perpetrator) {
        (Some(Overdosed), _) => *throwavay_rng.choose_with_fallback(overdosed_tips, &fallback),
        (Some(Exhausted), Some(_monster)) => {
            *throwavay_rng.choose_with_fallback(hunger_tips, &fallback)
        }
        (Some(Exhausted), None) => *throwavay_rng.choose_with_fallback(food_tips, &fallback),
        (Some(LostWill), Some(_monster)) => {
            *throwavay_rng.choose_with_fallback(anxiety_tips, &fallback)
        }
        _ => *throwavay_rng.choose_with_fallback(&all_tips, &fallback),
    };

    String::from(selected_tip)
}
