#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate clap;
extern crate rustyline;
extern crate sdl2;

use std::path::Path;
use std::time::{Duration, Instant};

mod bus;
mod cartridge;
mod cpu;
mod debugger;
mod gpu;
mod instructions;
mod joypad;
mod registers;
mod serial;
mod timer;

use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use cpu::Cpu;
use debugger::Debugger;
use joypad::Button;

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

fn main() {
    let matches = clap_app!(gamebust =>
                            (version: "0.1")
                            (author: "Iain Ireland")
                            (about: "gameboy emulator")
                            (@arg INPUT: +required "Sets the input file to use")

    ).get_matches();

    let input_file = matches.value_of("INPUT").unwrap();

    let scale = 5;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("Gamebust",
                                        SCREEN_WIDTH as u32 * scale,
                                        SCREEN_HEIGHT as u32 * scale)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut screen_texture = texture_creator.
        create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24,
                                 SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32).unwrap();
    let screen_rect = Rect::new(0, 0, SCREEN_WIDTH as u32 * scale, SCREEN_HEIGHT as u32 * scale);
    canvas.set_draw_color(Color::RGB(0,0,0));
    canvas.clear();

    let mut events = sdl_context.event_pump().unwrap();
    let mut cpu = Cpu::new(Path::new(&input_file));
    let mut debugger = Debugger::new();
    let mut frame_start = Instant::now();

    'eventloop: loop {

        for event in events.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { scancode: Some(Scancode::Escape), .. } =>
                    break 'eventloop,
                Event::KeyDown { scancode: Some(Scancode::D), .. } =>
                    debugger.pause(),
                Event::KeyDown { scancode: scan, .. } =>
                    if let Some(button) = Button::from_scancode(scan.unwrap()) {
                        cpu.key_down(button)
                    },
                Event::KeyUp { scancode: scan, .. } =>
                    if let Some(button) = Button::from_scancode(scan.unwrap()) {
                        cpu.key_up(button)
                    },
                _ => {}
            }
        }

        if debugger.is_paused() {
            debugger.debug(&cpu);
        }

        cpu.step(debugger.get_state());
        if cpu.needs_redraw() {
            const MICROS_PER_FRAME: u64 = 1_000_000 / 60;
            let data = cpu.get_screen_buffer();
            screen_texture.update(None, data, SCREEN_WIDTH * 3).unwrap();
            canvas.copy(&screen_texture, None, screen_rect).unwrap();
            canvas.present();

            let frame_time = frame_start.elapsed().subsec_micros() as u64;
            frame_start = Instant::now();
            if frame_time < MICROS_PER_FRAME {
                ::std::thread::sleep(Duration::from_micros(MICROS_PER_FRAME - frame_time));
            }
        }
    }
}
