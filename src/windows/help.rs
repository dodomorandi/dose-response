use crate::{
    color,
    engine::{Display, DrawResult, TextMetrics, TextOptions},
    point::Point,
    rect::Rectangle,
    state::State,
    ui::{self, Button},
};

use std::{
    convert::TryFrom,
    fmt::{Display as FmtDisplay, Error, Formatter},
};

use serde::{Deserialize, Serialize};

pub enum Action {
    NextPage,
    PrevPage,
    LineUp,
    LineDown,
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Page {
    DoseResponse,
    NumpadControls,
    ArrowControls,
    ViKeys,
    HowToPlay,
    Legend,
    Credits,
    About,
}

impl Page {
    pub fn prev(self) -> Option<Self> {
        use self::Page::*;
        match self {
            DoseResponse => None,
            NumpadControls => Some(DoseResponse),
            ArrowControls => Some(NumpadControls),
            ViKeys => Some(ArrowControls),
            HowToPlay => Some(ViKeys),
            Legend => Some(HowToPlay),
            Credits => Some(Legend),
            About => Some(Credits),
        }
    }

    pub fn next(self) -> Option<Self> {
        use self::Page::*;
        match self {
            DoseResponse => Some(NumpadControls),
            NumpadControls => Some(ArrowControls),
            ArrowControls => Some(ViKeys),
            ViKeys => Some(HowToPlay),
            HowToPlay => Some(Legend),
            Legend => Some(Credits),
            Credits => Some(About),
            About => None,
        }
    }
}

impl FmtDisplay for Page {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use self::Page::*;
        let s = match *self {
            DoseResponse => "Dose Response",
            NumpadControls => "Controls: numpad",
            ArrowControls => "Controls: arrow keys",
            ViKeys => "Controls: Vi keys",
            HowToPlay => "How to play",
            Legend => "Legend",
            Credits => "Credits",
            About => "About Dose Response",
        };
        f.write_str(s)
    }
}

struct Layout {
    next_page_button: Option<Button>,
    prev_page_button: Option<Button>,
    close_button: Button,
    scroll_up_button: Button,
    scroll_down_button: Button,
    action_under_mouse: Option<Action>,
    rect_under_mouse: Option<Rectangle>,
    window_rect: Rectangle,
    help_text_rect: Rectangle,
}

pub struct Window;

impl Window {
    fn layout(
        &self,
        state: &State,
        metrics: &dyn TextMetrics,
        display: &Display,
        top_level: bool,
    ) -> Layout {
        let screen_padding = Point::from_i32(2);
        let window_rect = Rectangle::from_point_and_size(
            screen_padding,
            display.size_without_padding() - (screen_padding * 2),
        );

        let contents_rect = Rectangle::new(
            window_rect.top_left() + (2, 1),
            window_rect.bottom_right() - (2, 1),
        );

        let help_text_rect = Rectangle::new(
            contents_rect.top_left() + (0, 1),
            contents_rect.bottom_right() - (0, 1),
        );

        let mut action_under_mouse = None;
        let mut rect_under_mouse = None;

        let next_page_button = state.current_help_window.next().map(|text| {
            let text = format!("[->] {}", text);
            let button = Button::new(contents_rect.bottom_right(), &text).align_right();
            let button_rect = metrics.button_rect(&button);
            if button_rect.contains(state.mouse.tile_pos) {
                action_under_mouse = Some(Action::NextPage);
                rect_under_mouse = Some(button_rect);
            }
            button
        });

        let prev_page_button = state.current_help_window.prev().map(|text| {
            let text = format!("{} [<-]", text);
            let button = Button::new(contents_rect.bottom_left(), &text);
            let button_rect = metrics.button_rect(&button);
            if button_rect.contains(state.mouse.tile_pos) {
                action_under_mouse = Some(Action::PrevPage);
                rect_under_mouse = Some(button_rect);
            }
            button
        });

        let scroll_up_button = {
            let mut button = Button::new(window_rect.top_right() + (0, 2), " ");
            button.text_options.height = 2;
            let button_rect = metrics.button_rect(&button);
            if button_rect.contains(state.mouse.tile_pos) {
                action_under_mouse = Some(Action::LineUp);
                rect_under_mouse = Some(button_rect);
            }
            button
        };

        let scroll_down_button = {
            let mut button = Button::new(window_rect.bottom_right() - (0, 3), " ");
            button.text_options.height = 2;
            let button_rect = metrics.button_rect(&button);
            if button_rect.contains(state.mouse.tile_pos) {
                action_under_mouse = Some(Action::LineDown);
                rect_under_mouse = Some(button_rect);
            }
            button
        };

        let close_button = {
            let text = format!("[Esc] Close");
            let mut button = Button::new(window_rect.top_right() - (1, 0), &text);
            button.text_options = TextOptions::align_right();
            let button_rect = metrics.button_rect(&button);
            if button_rect.contains(state.mouse.tile_pos) {
                action_under_mouse = Some(Action::Close);
                rect_under_mouse = Some(button_rect);
            }
            button
        };

        if !top_level {
            action_under_mouse = None;
            rect_under_mouse = None;
        }

        Layout {
            window_rect,
            help_text_rect,
            next_page_button,
            prev_page_button,
            scroll_up_button,
            scroll_down_button,
            close_button,
            action_under_mouse,
            rect_under_mouse,
        }
    }

    pub fn render(
        &self,
        state: &State,
        metrics: &dyn TextMetrics,
        display: &mut Display,
        top_level: bool,
    ) {
        use crate::ui::Text::*;

        let layout = self.layout(state, metrics, display, top_level);

        display.draw_rectangle(layout.window_rect, color::window_edge);

        display.draw_rectangle(
            Rectangle::new(
                layout.window_rect.top_left() + (1, 1),
                layout.window_rect.bottom_right() - (1, 1),
            ),
            color::window_background,
        );

        let header = format!("{}", state.current_help_window);
        let version = &format!(
            "{} version: {}",
            crate::metadata::TITLE,
            crate::metadata::VERSION
        );

        let copyright = format!("Copyright 2013-2020 {}", crate::metadata::AUTHORS);
        let homepage = &format!("Homepage: {}", crate::metadata::HOMEPAGE);
        let git_msg = &format!("Git commit: {}", crate::metadata::GIT_HASH);

        display.draw_text_in_tile_coordinates(
            // TODO: this needs to be pixel
            layout.window_rect.top_left(),
            &header,
            color::gui_text,
            TextOptions::align_center(layout.window_rect.width()),
            display.tile_size,
        );

        let mut lines = vec![];

        match state.current_help_window {
            Page::DoseResponse => {
                lines.push(Paragraph("Dose Response is a roguelike: every time you start a game, the map will be different. The items and monsters will be in new places. And when you lose, that's it -- you can't reload and try again. You start from the beginning, with a brand new map. Every life matters."));
                lines.push(Empty);
                lines.push(Paragraph("You can't learn the map (because it changes), but you can learn the world. How do the monsters work? What happens when you take two doses at the same time? What's that glowing thing around a dose? What is food good for?"));
                lines.push(Empty);
                lines.push(Paragraph("You will lose quickly and often. That's normal. Learn from it! What went wrong? Is there anything you could have done better? Were you saving an item for later that could have helped you?"));
                lines.push(Empty);
                lines.push(Paragraph(
                    "Each run takes 3 - 10 minutes so you won't lose that much anyway. Experiment!",
                ));
            }

            Page::NumpadControls => {
                lines.push(Paragraph("You control the @ character. It moves just like the king in Chess: one step in any direction. That means up, down, left, right, but also diagonally."));
                lines.push(Empty);
                lines.push(Paragraph("You can use the numpad. Imagine your @ is in the middle (where [5] is) and you just pick a direction."));
                lines.push(EmptySpace(1));

                lines.push(SquareTiles(r"7 8 9"));
                lines.push(SquareTiles(r" \|/ "));
                lines.push(SquareTiles(r"4-@-6"));
                lines.push(SquareTiles(r" /|\ "));
                lines.push(SquareTiles(r"1 2 3"));

                lines.push(EmptySpace(1));
                lines.push(Paragraph("Using items: you can use an item you're carrying (food and later on, doses) by clicking on it in the sidebar or pressing its number on the keyboard (not numpad -- that's for movement)."));
            }

            Page::ArrowControls => {
                lines.push(Paragraph("You control the @ character. It moves just like the king in Chess: one step in any direction. That means up, down, left, right, but also diagonally."));
                lines.push(Empty);
                lines.push(Paragraph("If you don't have a numpad, you can use the arrow keys. You will need [Shift] and [Ctrl] for diagonal movement. [Shift] means up and [Ctrl] means down. You combine them with the [Left] and [Right] keys."));

                lines.push(EmptySpace(1));

                lines.push(SquareTiles(r"Shift+Left  Up  Shift+Right"));
                lines.push(SquareTiles(r"         \  |  /           "));
                lines.push(SquareTiles(r"       Left-@-Right        "));
                lines.push(SquareTiles(r"         /  |  \           "));
                lines.push(SquareTiles(r"Ctrl+Left  Down Ctrl+Right "));

                lines.push(EmptySpace(1));
                lines.push(Paragraph("Using items: you can use an item you're carrying (food and later on, doses) by clicking on it in the sidebar or pressing its number on the keyboard (not numpad -- that's for movement)."));
            }

            Page::ViKeys => {
                lines.push(Paragraph("You control the @ character. It moves just like the king in Chess: one step in any direction. That means up, down, left, right, but also diagonally."));
                lines.push(Empty);
                lines.push(Paragraph("You can also move using the \"Vi keys\". Those map to the letters on your keyboard. This makes more sense if you've ever used the Vi text editor."));
                lines.push(EmptySpace(1));

                lines.push(SquareTiles(r"y k u"));
                lines.push(SquareTiles(r" \|/ "));
                lines.push(SquareTiles(r"h-@-l"));
                lines.push(SquareTiles(r" /|\ "));
                lines.push(SquareTiles(r"b j n"));

                lines.push(EmptySpace(1));
                lines.push(Paragraph("Using items: you can use an item you're carrying (food and later on, doses) by clicking on it in the sidebar or pressing its number on the keyboard (not numpad -- that's for movement)."));
            }

            Page::HowToPlay => {
                lines.push(Paragraph("Your character ('@') is an addict. Stay long without using a Dose ('i'), and the game is over. Eat food ('%') to remain sober for longer. Using a Dose or eating Food will also defeat nearby enemies."));
                lines.push(Empty);
                lines.push(Paragraph("If you step into the glow around a Dose, you can't resist even if it means Overdosing yourself. At the beginning, you will also Overdose by using a Dose when you're still High or using a Dose that's too strong ('+', 'x' or 'I'). By using Doses you build up tolerance. You'll need stronger Doses later on."));
                lines.push(Empty);
                lines.push(Paragraph("The letters ('h', 'v', 'S', 'a' and 'D') are enemies. Each has their own way of harming you. The Depression ('D') moves twice as fast. The Anxiety ('a') will reduce your Will on each hit. When it reaches zero, you will lose."));
                lines.push(Empty);
                lines.push(Paragraph("To progress, your Will needs to get stronger. Defeat enough Anxieties ('a') to make it go up. The Dose or Food \"explosions\" don't count though! Higher Will shrinks the irresistible area around Doses. It also lets you pick them up!"));
                lines.push(Empty);
                lines.push(Paragraph("If you see another '@' characters, they are friendly. They will give you a bonus and follow you around, but only while you're Sober. You can have only one bonus active at a time."));
            }

            Page::Legend => {
                lines.push(Paragraph("Monsters:"));
                lines.push(Paragraph(
                    "'a' (anxiety): takes Will away when it hits you. Defeat them to win the game.",
                ));
                lines.push(Paragraph(
                    "'D' (depression): moves twice as fast. You lose immediately when it hits you.",
                ));
                lines.push(Paragraph(
                    "'h' (hunger): summons other Hungers nearby. Reduces your mind state.",
                ));
                lines.push(Paragraph(
                    "'v' (hearing voices): paralyzes you for three turns.",
                ));
                lines.push(Paragraph(
                    "'S' (seeing shadows): makes you move randomly for three turns.",
                ));
                lines.push(Paragraph(
                    "'@' (friendly): ignores you when High. Bump into them Sober for a bonus.",
                ));
                lines.push(Empty);

                lines.push(Paragraph("Items:"));
                lines.push(Paragraph("'%' (food): prolongs being Sober or in a Withdrawal. Kills monsters around you."));
                lines.push(Paragraph(
                    "'i' (dose): makes you High. When you're High already, you'll likely Overdose.",
                ));
                lines.push(Paragraph(
                    "'+' (cardinal dose): Destroys trees in the horizontal and vertical lines.",
                ));
                lines.push(Paragraph(
                    "'x' (diagonal dose): Destroys trees in the diagonal lines.",
                ));
                lines.push(Paragraph(
                    "'I' (strong dose): very strong Dose. Don't walk into it by accident.",
                ));
                lines.push(Empty);

                lines.push(Paragraph("Each Dose has a faint glow around it. If you step into it, you will not be able to resist."));
                lines.push(Empty);
                lines.push(Paragraph("When the glow disappears completely, you can pick the dose up and use it later. Don't lose Will if you're carrying doses though!"));
            }

            Page::Credits => {
                lines.push(Paragraph(
                    "Design and development by Tomas Sedovic at https://tomas.sedovic.cz/",
                ));
                lines.push(Paragraph("Copyright (C) 2013-2020 Tomas Sedovic"));
                lines.push(Paragraph(
                    "licensed under GNU General Public License 3 or later",
                ));
                lines.push(Empty);
                lines.push(Paragraph("Tiles by VEXED at https://vexed.zone/"));
                lines.push(Paragraph("licensed under Creative Commons 0"));
                lines.push(Empty);
                lines.push(Paragraph(
                    "Mononoki typeface by Matthias Tellen at https://github.com/madmalik",
                ));
                lines.push(Paragraph(
                    "Copyright (c) 2013, Matthias Tellen matthias.tellen@googlemail.com",
                ));
                lines.push(Paragraph(
                    "licensed under the SIL Open Font License, Version 1.1",
                ));
            }

            Page::About => {
                lines.push(Paragraph(version));
                lines.push(Paragraph(homepage));
                lines.push(Empty);

                if !crate::metadata::GIT_HASH.trim().is_empty() {
                    lines.push(Paragraph(git_msg));
                    lines.push(Empty);
                }

                lines.push(Paragraph("Dose Response is a Free and Open Source software provided under the terms of GNU General Public License version 3 or later. If you did not receieve the license text with the program, you can read it here:"));
                lines.push(Paragraph("https://www.gnu.org/licenses/gpl-3.0.en.html"));
                lines.push(Empty);
                lines.push(Paragraph(&copyright));
            }
        }

        let res = ui::render_text_flow(
            &lines,
            layout.help_text_rect,
            state.help_starting_line,
            metrics,
            display,
        );
        if let DrawResult::Overflow = res {
            let options = TextOptions::align_center(layout.window_rect.width());
            display.draw_text_in_tile_coordinates(
                // TODO: this needs to be fixel
                layout.window_rect.bottom_left() - (0, 1),
                "(more)",
                color::gui_text,
                options,
                display.tile_size,
            );
        }

        if let Some(highlighted_rect) = layout.rect_under_mouse {
            display.draw_rectangle(highlighted_rect, color::menu_highlight);
        }

        {
            // Render the "up" portion of the scollbar
            let glyph = char::try_from(710u32).unwrap_or('^');
            let button = &layout.scroll_up_button;
            let tilesize = metrics.tile_width_px();
            let x_offset_px = (tilesize - metrics.advance_width_px(glyph)) / 2;
            display.draw_glyph_abs_px(
                button.pos.x * tilesize + x_offset_px,
                button.pos.y * tilesize,
                glyph,
                button.color,
            );
            let pos_px = button.pos + (0, 1);
            display.draw_glyph_abs_px(
                pos_px.x * tilesize + x_offset_px,
                pos_px.y * tilesize,
                glyph,
                button.color,
            );
            display.draw_glyph_abs_px(
                pos_px.x * tilesize + x_offset_px,
                pos_px.y * tilesize - (tilesize / 2),
                glyph,
                button.color,
            );
        }

        {
            // Render the "down" portion of the scollbar
            let glyph = char::try_from(711u32).unwrap_or('v');
            let button = &layout.scroll_down_button;
            let tilesize = metrics.tile_width_px();
            let x_offset_px = (tilesize - metrics.advance_width_px(glyph)) / 2;
            display.draw_glyph_abs_px(
                button.pos.x * tilesize + x_offset_px,
                button.pos.y * tilesize,
                glyph,
                button.color,
            );
            let pos_px = button.pos + (0, 1);
            display.draw_glyph_abs_px(
                pos_px.x * tilesize + x_offset_px,
                pos_px.y * tilesize,
                glyph,
                button.color,
            );
            display.draw_glyph_abs_px(
                pos_px.x * tilesize + x_offset_px,
                pos_px.y * tilesize - (tilesize / 2),
                glyph,
                button.color,
            );
        }

        display.draw_button(&layout.close_button);

        if let Some(button) = layout.next_page_button {
            display.draw_button(&button)
        }

        if let Some(button) = layout.prev_page_button {
            display.draw_button(&button)
        }
    }

    pub fn hovered(
        &self,
        state: &State,
        metrics: &dyn TextMetrics,
        display: &Display,
        top_level: bool,
    ) -> Option<Action> {
        self.layout(state, metrics, display, top_level)
            .action_under_mouse
    }
}
