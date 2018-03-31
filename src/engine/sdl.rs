use color::Color;
use engine::{self, Draw, Settings, TextMetrics, UpdateFn};
use game::RunningState;
use point::Point;
use state::State;

use std::time::{Duration, Instant};
use std::thread;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator};
use sdl2::surface::Surface;
use image;


const DESIRED_FPS: u64 = 60;


fn load_texture<T>(texture_creator: &TextureCreator<T>) -> Result<Texture, String> {
    let data = &include_bytes!(concat!(env!("OUT_DIR"), "/font.png"))[..];
    let image = image::load_from_memory(data)
        .map_err(|err| format!("Error loading image: {}", err))?.to_rgba();
    let (width, height) = image.dimensions();
    // Pitch is the length of the row in bytes. We have 4 bytes (RGBA, each is a u8):
    let pitch = width * 4;
    // NOTE: I think `SDL2` and `Image` differ in endianness and
    // that's why we have to say ABGR instead of RGBA here
    let format = PixelFormatEnum::ABGR8888;

    let raw_image = &mut image.into_raw();
    let temp_surface = Surface::from_data(raw_image, width, height, pitch, format)?;

    texture_creator.create_texture_from_surface(&temp_surface)
        .map_err(|err| format!("Could not create texture from surface: {}", err))
}


pub fn main_loop(
    display_size: Point,
    default_background: Color,
    window_title: &str,
    mut state: State,
    update: UpdateFn,
) {
    let tilesize = super::TILESIZE;
    let (desired_window_width, desired_window_height) = (
        display_size.x as u32 * tilesize as u32,
        display_size.y as u32 * tilesize as u32,
    );

    let sdl_context = sdl2::init()
        .expect("SDL context creation failed.");
    let video_subsystem = sdl_context.video()
        .expect("SDL video subsystem creation failed.");

    // NOTE: add `.fullscreen_desktop()` to start in fullscreen.
    let window = video_subsystem.window(window_title, desired_window_width, desired_window_height)
        .position_centered()
        .build()
        .expect("SDL window creation failed.");

    // NOTE: use `.software()` instead of `.accelerated()` to use software rendering
    let mut canvas = window.into_canvas()
        .accelerated()
        .build()
        .expect("SDL canvas creation failed.");

    let mut event_pump = sdl_context.event_pump()
        .expect("SDL event pump creation failed.");

    let texture_creator = canvas.texture_creator();
    let texture = load_texture(&texture_creator)
        .expect("Loading texture failed.");

    let expected_frame_length = Duration::from_millis(1000 / DESIRED_FPS);

    let mut running = true;
    while running {
        let clock = Instant::now();
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 128, 128));

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
                    running = false;
                },
                Event::KeyDown {keycode: Some(Keycode::F), ..} => {
                    use sdl2::video::FullscreenType::*;
                    println!("Toggling fullscreen");
                    let fullscreen_state = canvas.window().fullscreen_state();
                    println!("Current state: {:?}", fullscreen_state);
                    let result = match fullscreen_state {
                        Off => {
                            println!("Switching to (desktop-type) fullscreen");
                            canvas.window_mut().set_fullscreen(Desktop)
                        }
                        True => {
                            println!("Switching fullscreen OFF");
                            canvas.window_mut().set_fullscreen(Off)
                        }
                        Desktop => {
                            println!("Switching fullscreen OFF");
                            canvas.window_mut().set_fullscreen(Off)
                        }
                    };
                    println!("Fullscreen result: {:?}", result);
                }
                _ => {}
            }
        }

        canvas.clear();
        canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 0, 255));

        let rects = &[];
        for &(src, dst) in rects {
            // Highlight the sprite's target boundaries
            if let Err(err) = canvas.fill_rect(dst) {
                println!("WARNING: drawing rectangle {:?} failed:", dst);
                println!("{}", err);
            }

            // Draw the sprite
            // NOTE: use `copy_ex` to rotate or flip the image
            if let Err(err) = canvas.copy(&texture, Some(src), Some(dst)) {
                println!("WARNING: blitting {:?} to {:?} failed:", src, dst);
                println!("{}", err);
            }
        }

        canvas.present();

        // println!("Code duration: {:?}ms",
        //          clock.elapsed().subsec_nanos() as f32 / 1_000_000.0);
        if let Some(sleep_duration) = expected_frame_length.checked_sub(clock.elapsed()) {
            thread::sleep(sleep_duration);
        };
        // println!("Total frame duration: {:?}ms",
        //          clock.elapsed().subsec_nanos() as f32 / 1_000_000.0);
    }
}
