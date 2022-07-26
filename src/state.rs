use crate::{
    animation::{self, AreaOfEffect, ScreenFade},
    color::Color,
    engine::Mouse,
    formula,
    graphic::Graphic,
    keys::{Key, Keys},
    monster,
    palette::Palette,
    pathfinding::Path,
    player::Player,
    point::Point,
    random::Random,
    stats::Stats,
    timer::Timer,
    util,
    window::Window,
    windows,
    world::{MonsterId, World},
};

use std::{
    collections::VecDeque,
    error::Error,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    time::Duration,
};

#[cfg(feature = "replay")]
use std::fs;

use serde::{Deserialize, Serialize};

const CHUNK_SIZE: i32 = 32;

// TODO: Rename this to `GameState` and the existing `GameState` to
// `Game`? It's no longer just who's side it is but also: did the
// player won? Lost?
#[derive(Copy, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Side {
    Player,
    Victory,
}

/// The status of the current game session. Whether it's not even
/// started (e.g. we just opened the app but didn't click "New Game"),
/// it's currently running or has been finished (by winning or
/// losing).
#[derive(Copy, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum GameSession {
    NotStarted,
    InProgress,
    Ended,
}

impl GameSession {
    pub fn started(&self) -> bool {
        use GameSession::*;
        match *self {
            NotStarted => false,
            InProgress => true,
            Ended => true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    N,
    E,
    S,
    W,
    NE,
    NW,
    SE,
    SW,
    UseFood,
    UseDose,
    UseCardinalDose,
    UseDiagonalDose,
    UseStrongDose,
    ShowMessageBox {
        ttl: Duration,
        title: String,
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub keys: Vec<Key>,
    pub mouse: Mouse,
    pub tick_id: i32,
}

pub fn generate_replay_path() -> Option<PathBuf> {
    #[cfg(feature = "replay")]
    {
        use chrono::prelude::*;
        let local_time = Local::now();

        // Timestamp in format: 2016-11-20T20-04-39.123. We can't use the
        // colons in the timestamp -- Windows don't allow them in a path.
        let timestamp = local_time.format("%FT%H-%M-%S%.3f");
        let replay_dir = &std::path::Path::new("replays");
        assert!(replay_dir.is_relative());
        if !replay_dir.exists() {
            let _ = fs::create_dir_all(replay_dir);
        }
        let replay_path = &replay_dir.join(format!("replay-{}", timestamp));
        Some(replay_path.into())
    }

    #[cfg(not(feature = "replay"))]
    {
        None
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub turn: i32,
    pub chunk_count: usize,
    pub player_pos: Point,
    pub monsters: Vec<(Point, Point, monster::Kind)>,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub player: Player,
    #[serde(skip_serializing, skip_deserializing)]
    pub explosion_animation: Option<Box<dyn AreaOfEffect>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub extra_animations: Vec<MotionAnimation>,

    /// The actual size of the game world in tiles. Could be infinite
    /// but we're limiting it for performance reasons for now.
    pub world_size: Point,
    pub world: World,

    /// The size of the game map inside the game window. We're keeping
    /// this square so this value represents both width and height.
    /// It's a window into the game world that is actually rendered.
    pub map_size: Point,

    /// The width of the in-game status panel.
    pub panel_width: i32,

    pub screen_position_in_world: Point,
    pub seed: u32,
    pub rng: Random,
    pub audio_rng: Random,
    // Keys pressed this turn (or loaded from the replay file)
    pub keys: Keys,
    // Mouse config read from the player this turn (or loaded from the replay file)
    pub mouse: Mouse,
    #[serde(skip_serializing, skip_deserializing)]
    pub inputs: VecDeque<Input>,
    pub commands: VecDeque<Command>,
    pub player_path: Path,
    #[serde(skip_serializing, skip_deserializing)]
    pub verifications: VecDeque<Verification>,
    #[serde(skip_serializing, skip_deserializing, default = "empty_command_logger")]
    pub input_logger: Box<dyn Write>,
    pub side: Side,
    pub turn: i32,
    pub tick_id: i32,
    pub cheating: bool,
    pub replay: bool,
    pub replay_full_speed: bool,
    pub exit_after: bool,
    pub clock: Duration,
    pub replay_step: Duration,
    #[serde(skip_serializing, skip_deserializing)]
    pub stats: Stats,
    pub pos_timer: Timer,
    pub path_walking_timer: Timer,
    pub paused: bool,
    pub old_screen_pos: Point,
    pub new_screen_pos: Point,
    pub screen_fading: Option<ScreenFade>,
    pub offset_px: Point,

    /// Whether the game has started, is currently running or is over
    /// (one way or another) and we should show the endgame screen --
    /// uncovered map, the score, etc.
    pub game_session: GameSession,
    pub victory_npc_id: Option<MonsterId>,

    pub window_stack: windows::Windows<Window>,

    pub show_keyboard_movement_hints: bool,
    pub show_anxiety_counter: bool,
    pub player_picked_up_a_dose: bool,
    pub player_bumped_into_a_monster: bool,
    pub current_help_window: windows::help::Page,
    /// Used for help contents pagination: which line should we start showing.
    pub help_starting_line: i32,

    /// Whether we should push the Endscreen window and uncover the
    /// map during the transition from screen fade out to fade in
    /// phase. This is purely a visual effect and the values here are
    /// a bit of a hack. If there's more instances of us wanting to do
    /// this, we should just have a list of screen fade transition
    /// effects here instead.
    pub show_endscreen_and_uncover_map_during_fadein: bool,
    pub uncovered_map: bool,

    pub challenge: Challenge,
    pub palette: Palette,
}

impl State {
    #[allow(clippy::too_many_arguments)]
    fn new<W: Write + 'static>(
        world_size: Point,
        map_size: Point,
        panel_width: i32,
        inputs: VecDeque<Input>,
        verifications: VecDeque<Verification>,
        log_writer: W,
        seed: u32,
        cheating: bool,
        invincible: bool,
        replay: bool,
        replay_full_speed: bool,
        exit_after: bool,
        challenge: Challenge,
        palette: Palette,
    ) -> State {
        let world_centre = (0, 0).into();
        assert_eq!(world_size.x, world_size.y);
        let seed = if cfg!(feature = "recording") {
            518_723_646
        } else {
            seed
        };
        log::info!("Using seed: {:?}", seed);
        let mut rng = Random::from_seed(seed);
        let audio_rng = rng.clone();
        let player_position = world_centre;
        let player = {
            let mut player = Player::new(player_position, invincible);
            if let Some(&graphic) =
                rng.choose(&[Graphic::CharacterSkirt, Graphic::CharacterTrousers])
            {
                player.graphic = graphic;
            }

            if let Some(&color_index) = rng.choose(&[0, 1, 2, 3, 4, 5]) {
                player.color_index = color_index;
            }

            player
        };

        let world = World::new(seed, world_size.x, CHUNK_SIZE, player.info(), challenge);

        // TODO: I think we'll want to create a Commands queue again here and then use that from everything

        State {
            player,
            explosion_animation: None,
            extra_animations: vec![],
            world_size,
            world,
            map_size,
            panel_width,
            screen_position_in_world: world_centre,
            seed,
            rng,
            audio_rng,
            keys: Keys::new(),
            mouse: Default::default(),
            inputs,
            commands: VecDeque::new(),
            player_path: Path::default(),
            verifications,
            input_logger: Box::new(log_writer),
            side: Side::Player,
            turn: 0,
            tick_id: 0,
            cheating,
            replay,
            replay_full_speed,
            exit_after,
            clock: Duration::new(0, 0),
            replay_step: Duration::new(0, 0),
            stats: Default::default(),
            pos_timer: Timer::new(Duration::from_millis(0)),
            path_walking_timer: Timer::new_elapsed(formula::PLAYER_PATH_WALKING_DELAY, 1.0),
            old_screen_pos: (0, 0).into(),
            new_screen_pos: (0, 0).into(),
            offset_px: Point::zero(),
            paused: false,
            screen_fading: None,
            game_session: GameSession::NotStarted,
            victory_npc_id: None,
            window_stack: windows::Windows::new(Window::Game),
            // NOTE: Since we've got the mouse support and the numpad
            // hints in the sidebar, let's see if we can just show
            // them never. We might even remove the whole thing at
            // some point.
            show_keyboard_movement_hints: false,
            show_anxiety_counter: false,
            player_picked_up_a_dose: false,
            player_bumped_into_a_monster: false,
            current_help_window: windows::help::Page::DoseResponse,
            help_starting_line: 0,
            show_endscreen_and_uncover_map_during_fadein: false,
            uncovered_map: false,

            challenge,
            palette,
        }
    }

    pub fn new_game(
        world_size: Point,
        map_size: Point,
        panel_width: i32,
        exit_after: bool,
        replay_path: Option<PathBuf>,
        challenge: Challenge,
        palette: Palette,
    ) -> State {
        let inputs = VecDeque::new();
        let verifications = VecDeque::new();
        let seed = util::random_seed();
        let mut writer: Box<dyn Write> = if let Some(replay_path) = replay_path {
            match File::create(&replay_path) {
                Ok(f) => {
                    log::info!("Recording the gameplay to '{}'", replay_path.display());
                    Box::new(f)
                }
                Err(msg) => {
                    log::error!(
                        "Failed to create the replay file at '{:?}'.
Reason: '{}'.",
                        replay_path.display(),
                        msg
                    );
                    Box::new(io::sink())
                }
            }
        } else {
            Box::new(io::sink())
        };

        log_header(&mut writer, seed);
        let cheating = false;
        let replay = false;
        let invincible = false;
        let replay_full_speed = false;
        State::new(
            world_size,
            map_size,
            panel_width,
            inputs,
            verifications,
            writer,
            seed,
            cheating,
            invincible,
            replay,
            replay_full_speed,
            exit_after,
            challenge,
            palette,
        )
    }

    #[cfg_attr(not(feature = "replay"), allow(dead_code, unused_variables))]
    pub fn replay_game(
        world_size: Point,
        map_size: Point,
        panel_width: i32,
        replay_path: &std::path::Path,
        cheating: bool,
        invincible: bool,
        replay_full_speed: bool,
        exit_after: bool,
        challenge: Challenge,
        palette: Palette,
    ) -> Result<State, Box<dyn Error>> {
        #[cfg(feature = "replay")]
        {
            use std::io::{BufRead, BufReader};
            let mut inputs = VecDeque::new();
            let mut verifications = VecDeque::new();
            let file = File::open(replay_path)?;
            let mut lines = BufReader::new(file).lines();
            let seed: u32 = match lines.next() {
                Some(seed_str) => seed_str?.parse()?,
                None => throw!("The replay file is empty."),
            };

            match lines.next() {
                Some(version) => {
                    let version = version?;
                    if version != crate::metadata::VERSION {
                        log::warn!(
                            "The replay file's version is: {}, but the program is: {}",
                            version,
                            crate::metadata::VERSION
                        );
                    }
                }
                None => throw!("The replay file is missing the version."),
            };

            match lines.next() {
                Some(commit) => {
                    let commit = commit?;
                    if commit != crate::metadata::GIT_HASH {
                        log::warn!(
                            "The replay file's commit is: {}, but the program is: {}.",
                            commit,
                            crate::metadata::GIT_HASH
                        );
                    }
                }
                None => throw!("The replay file is missing the commit hash."),
            };

            for line in lines {
                let line = line?;
                // Try parsing it as an `Input` first, otherwise it's a `Verification`
                if let Ok(input) = serde_json::from_str::<Input>(&line) {
                    assert!(input.tick_id > 0);
                    assert_eq!(inputs.len(), input.tick_id as usize - 1);

                    inputs.push_back(input);
                } else {
                    // Must be a verification, then
                    let verification = serde_json::from_str(&line)?;
                    verifications.push_back(verification);
                }
            }

            log::info!("Replaying game log: '{}'", replay_path.display());
            let cheating = cheating;
            let invincible = invincible;
            let replay = true;
            let mut state = State::new(
                world_size,
                map_size,
                panel_width,
                inputs,
                verifications,
                Box::new(io::sink()),
                seed,
                cheating,
                invincible,
                replay,
                replay_full_speed,
                exit_after,
                challenge,
                palette,
            );
            state.game_session = GameSession::InProgress;
            state.generate_world();
            Ok(state)
        }

        #[cfg(not(feature = "replay"))]
        {
            let mut state = Self::new_game(
                world_size,
                map_size,
                panel_width,
                exit_after,
                None,
                challenge,
                palette,
            );
            state.generate_world();
            Ok(state)
        }
    }

    pub fn generate_world(&mut self) {
        self.world = World::new(
            self.seed,
            self.world_size.x,
            CHUNK_SIZE,
            self.player.info(),
            self.challenge,
        );
    }

    pub fn verification(&self) -> Verification {
        // TODO: we can sort the chunks and compare directly at some point.
        let chunks = self.world.positions_of_all_chunks();
        let mut monsters = vec![];
        for &chunk_pos in &chunks {
            if let Some(chunk) = self.world.chunk(chunk_pos) {
                for monster in chunk.monsters() {
                    if !monster.dead {
                        monsters.push((monster.position, chunk_pos, monster.kind));
                    }
                }
            }
        }
        monsters
            .sort_by_key(|&(monster_pos, _chunk_pos, kind)| (monster_pos.x, monster_pos.y, kind));

        Verification {
            turn: self.turn,
            chunk_count: chunks.len(),
            player_pos: self.player.pos,
            monsters,
        }
    }

    pub fn save_to_file(&self) -> Result<(), Box<dyn Error>> {
        // TODO: select the filename dynamically!
        let filename = "SAVEDGAME.sav";
        let version_data = bincode::serialize(crate::metadata::VERSION)?;
        let commit_data = bincode::serialize(crate::metadata::GIT_HASH)?;
        let state_data = bincode::serialize(self)?;

        // TODO: this can be compressed nicely!

        let mut file = File::create(filename)?;
        file.write_all(&version_data)?;
        file.write_all(&commit_data)?;
        file.write_all(&state_data)?;
        file.flush()?;

        Ok(())
    }

    pub fn load_from_file() -> Result<State, Box<dyn Error>> {
        let filename = "SAVEDGAME.sav";
        let state = {
            let file = File::open(filename)?;
            let version: String = bincode::deserialize_from(&file)?;
            log::info!("Savefile version {}", version);
            if version != crate::metadata::VERSION {
                log::warn!("The game was saved in a different version: {}. This release has version: {}. The game might not load properly.",
                           version,
                           crate::metadata::VERSION);
            }
            let commit: String = bincode::deserialize_from(&file)?;
            log::info!("Savefile commit {}", commit);
            if commit != crate::metadata::GIT_HASH {
                log::warn!("The game was saved in a different commit: {}. This release has commit: {}. The game might not load properly.",
                           commit,
                crate::metadata::GIT_HASH);
            }
            bincode::deserialize_from(&file)?
        };

        if let Err(error) = ::std::fs::remove_file(filename) {
            log::error!(
                "Failed to delete the successfully loaded savegame. Error: {:?}",
                error
            );
        }

        Ok(state)
    }

    pub fn screen_left_top_corner(&self) -> Point {
        self.screen_position_in_world - (self.map_size / 2)
    }

    pub fn screen_pos_from_world_pos(&self, world_pos: Point) -> Point {
        world_pos - self.screen_left_top_corner()
    }

    pub fn mouse_world_position(&self) -> Point {
        self.screen_left_top_corner() + self.mouse.tile_pos
    }
}

#[derive(Clone, Debug)]
pub struct MotionAnimation {
    pub pos: Point,
    pub graphic: Graphic,
    pub color: Color,
    pub animation: animation::Move,
}

/// The various challenges that the player can take. Persisted via
/// settings, but available to the state for easier access within the
/// game code.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Challenge {
    pub hide_unseen_tiles: bool,
    pub fast_depression: bool,
    pub one_chance: bool,
}

impl Default for Challenge {
    fn default() -> Self {
        Self {
            hide_unseen_tiles: true,
            fast_depression: true,
            one_chance: true,
        }
    }
}

fn empty_command_logger() -> Box<dyn Write> {
    Box::new(io::sink())
}

pub fn log_header<W: Write>(writer: &mut W, seed: u32) {
    let _ = writeln!(writer, "{}", seed);
    let _ = writeln!(writer, "{}", crate::metadata::VERSION);
    let _ = writeln!(writer, "{}", crate::metadata::GIT_HASH);
}

pub fn log_input<W: Write>(writer: &mut W, input: Input) {
    match serde_json::to_string(&input) {
        Ok(json_input) => {
            let _ = writeln!(writer, "{}", json_input);
        }
        Err(err) => {
            log::error!("Could not serialise {:?} to JSON: {}", input, err);
        }
    }
}

pub fn log_verification<W: Write>(writer: &mut W, verification: &Verification) {
    match serde_json::to_string(&verification) {
        Ok(json) => {
            let _ = writeln!(writer, "{}", json);
        }
        Err(error) => {
            log::error!("Could not serialise {:?} to JSON: {}", verification, error);
        }
    }
}
