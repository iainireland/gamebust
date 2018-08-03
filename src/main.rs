#[macro_use]
extern crate clap;
extern crate sdl2;

use std::path::Path;
use std::time::{Duration, Instant};

mod bus;
mod cartridge;
mod gpu;
mod instructions;
mod registers;

use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use registers::{Registers,Reg8,Reg16,Indirect};
use bus::Bus;
use instructions::{Cond,Instr};

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

fn main() {
    let matches = clap_app!(gamebust =>
                            (version: "0.1")
                            (author: "Iain Ireland")
                            (about: "gameboy emulator")
                            (@arg DEBUG: -d --debug "Turns on debug mode")
                            (@arg INPUT: +required "Sets the input file to use")

    ).get_matches();

    let input_file = matches.value_of("INPUT").unwrap();
    let mut debug = matches.is_present("DEBUG");

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
    let mut line_texture = texture_creator.
        create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24,
                                 SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32).unwrap();
    canvas.set_draw_color(Color::RGB(0,0,0));
    canvas.clear();

    let mut events = sdl_context.event_pump().unwrap();
    let mut cpu = CPU::new(Path::new(&input_file));
    let mut pause = false;
    let mut frame_start = Instant::now();

    'eventloop: loop {

        for event in events.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { scancode: Some(Scancode::Escape), .. } =>
                    break 'eventloop,
                Event::KeyDown { scancode: Some(Scancode::D), .. } =>
                    debug = !debug,
                Event::KeyDown { scancode: Some(Scancode::P), .. } =>
                    pause = !pause,
                Event::KeyDown { scancode: scan, .. } =>
                    if let Some(button) = scancode_to_button(scan.unwrap()) {
                        cpu.bus.key_down(button)
                    },
                Event::KeyUp { scancode: scan, .. } =>
                    if let Some(button) = scancode_to_button(scan.unwrap()) {
                        cpu.bus.key_up(button)
                    },
                _ => {}
            }
        }
        if pause { continue; }

        let instr = cpu.fetch();
        let cycles = cpu.exec(instr);
        let redraw = cpu.bus.update(cycles);
        if redraw {
            const MICROS_PER_FRAME: u64 = 1_000_000 / 60;
            let data = cpu.bus.get_screen_buffer();
            line_texture.update(None, data, SCREEN_WIDTH * 3).unwrap();
            let line_rect = Rect::new(0, 0, SCREEN_WIDTH as u32 * scale, SCREEN_HEIGHT as u32 * scale);
            canvas.copy(&line_texture, None, line_rect).unwrap();
            canvas.present();

            let frame_time = frame_start.elapsed().subsec_micros() as u64;
            frame_start = Instant::now();
            if frame_time < MICROS_PER_FRAME {
                ::std::thread::sleep(Duration::from_micros(MICROS_PER_FRAME - frame_time));
            } else {
                println!("Frame over budget! {}", frame_time);
            }
        }
    }
}

pub enum Button {
    Up, Down, Left, Right,
    A, B, Select, Start,
}
impl Button {
    pub fn value(&self) -> u8 {
        match self {
            Button::Right => 1 << 0,
            Button::Left => 1 << 1,
            Button::Up => 1 << 2,
            Button::Down => 1 << 3,
            Button::A => 1 << 4,
            Button::B => 1 << 5,
            Button::Select => 1 << 6,
            Button::Start => 1 << 7,
        }
    }
}

fn scancode_to_button(scancode: sdl2::keyboard::Scancode) -> Option<Button> {
    match scancode {
        Scancode::Up => Some(Button::Up),
        Scancode::Down => Some(Button::Down),
        Scancode::Left => Some(Button::Left),
        Scancode::Right => Some(Button::Right),
        Scancode::Return => Some(Button::Start),
        Scancode::Backspace => Some(Button::Select),
        Scancode::Z => Some(Button::A),
        Scancode::X => Some(Button::B),
        _ => None
    }
}

pub struct CPU {
    reg: Registers,
    bus: Bus,
    interrupts: bool,
    interrupts_buffer: bool,
    halted: bool,
    stopped: bool
}

impl CPU {
    pub fn new(cartridge_path: &Path) -> Self {
        CPU {
            reg: Registers::new(),
            bus: Bus::new(cartridge_path).expect("File not found"),
            interrupts: false,
            interrupts_buffer: false,
            halted: false,
            stopped: false,
        }
    }
    pub fn fetch(&mut self) -> Instr {
        let opcode = self.imm8();

        let x = opcode >> 6;
        let y = (opcode >> 3) & 7;
        let z = opcode & 7;

        match (x,y,z) {
            (0,0,0) => Instr::Nop,
            (0,1,0) => Instr::StoreSP(self.imm16()),
            (0,2,0) => Instr::Stop,
            (0,3,0) => Instr::JumpRelative(self.imm8() as i8, Cond::Always),
            (0,4...7,0) => Instr::JumpRelative(self.imm8() as i8, Cond::from(y-4)),
            (0,_,1) if y % 2 == 0  => Instr::LoadImm16(Reg16::from(y, true), self.imm16()),
            (0,_,1) => Instr::AddHL(Reg16::from(y, true)),
            (0,_,2) if y % 2 == 0 => Instr::StoreA(Indirect::from(y)),
            (0,_,2) => Instr::LoadA(Indirect::from(y)),
            (0,_,3) if y % 2 == 0 => Instr::Inc16(Reg16::from(y, true)),
            (0,_,3) => Instr::Dec16(Reg16::from(y, true)),
            (0,_,4) => Instr::Inc8(Reg8::from(y)),
            (0,_,5) => Instr::Dec8(Reg8::from(y)),
            (0,_,6) => Instr::LoadImm8(Reg8::from(y), self.imm8()),
            (0,0,7) => Instr::RotateALeft,
            (0,1,7) => Instr::RotateARight,
            (0,2,7) => Instr::RotateALeftCarry,
            (0,3,7) => Instr::RotateARightCarry,
            (0,4,7) => Instr::DecimalAdjust,
            (0,5,7) => Instr::Complement,
            (0,6,7) => Instr::ComplementCarry,
            (0,7,7) => Instr::SetCarry,
            (1,6,6) => Instr::Halt,
            (1,_,_) => Instr::RegCopy(Reg8::from(y), Reg8::from(z)),
            (2,0,_) => Instr::Add(Reg8::from(z)),
            (2,1,_) => Instr::AddCarry(Reg8::from(z)),
            (2,2,_) => Instr::Sub(Reg8::from(z)),
            (2,3,_) => Instr::SubCarry(Reg8::from(z)),
            (2,4,_) => Instr::And(Reg8::from(z)),
            (2,5,_) => Instr::Xor(Reg8::from(z)),
            (2,6,_) => Instr::Or(Reg8::from(z)),
            (2,7,_) => Instr::Comp(Reg8::from(z)),
            (3,0...3,0) => Instr::Ret(Cond::from(y)),
            (3,4,0) => Instr::StoreIO(self.imm8()),
            (3,5,0) => Instr::StackAdjust(self.imm8() as i8),
            (3,6,0) => Instr::LoadIO(self.imm8()),
            (3,7,0) => Instr::LoadLocalAddr(self.imm8() as i8),
            (3,_,1) if y % 2 == 0 => Instr::Pop(Reg16::from(y, false)),
            (3,1,1) => Instr::Ret(Cond::Always),
            (3,3,1) => Instr::RetI,
            (3,5,1) => Instr::JumpHL,
            (3,7,1) => Instr::LoadStackHL,
            (3,0...3,2) => Instr::Jump(self.imm16(), Cond::from(y)),
            (3,4,2) => Instr::StoreIOC,
            (3,5,2) => Instr::StoreGlobal(self.imm16()),
            (3,6,2) => Instr::LoadIOC,
            (3,7,2) => Instr::LoadGlobal(self.imm16()),
            (3,0,3) => Instr::Jump(self.imm16(), Cond::Always),
            (3,1,3) => {
                let extended_opcode = self.imm8();
                let operation = extended_opcode >> 6;
                let y = (extended_opcode >> 3) & 7;
                let reg = Reg8::from(extended_opcode & 7);
                match operation {
                    0 => { // Rotations
                        match y {
                            0 => Instr::RotateLeft(reg),
                            1 => Instr::RotateRight(reg),
                            2 => Instr::RotateLeftCarry(reg),
                            3 => Instr::RotateRightCarry(reg),
                            4 => Instr::ShiftLeft(reg),
                            5 => Instr::ShiftRightLogical(reg),
                            6 => Instr::SwapBytes(reg),
                            7 => Instr::ShiftRightArith(reg),
                            _ => unreachable!("Invalid rotation"),
                        }
                    },
                    1 => Instr::Bit(reg, y),
                    2 => Instr::Reset(reg, y),
                    3 => Instr::Set(reg, y),
                    _ => unreachable!("Invalid extended instruction")
                }
            },
            (3,6,3) => Instr::DisableInterrupts,
            (3,7,3) => Instr::EnableInterrupts,
            (3,0...3,4) => Instr::Call(self.imm16(), Cond::from(y)),
            (3,_,5) if y % 2 == 0 => Instr::Push(Reg16::from(y, false)),
            (3,1,5) => Instr::Call(self.imm16(), Cond::Always),
            (3,0,6) => Instr::AddImm(self.imm8()),
            (3,1,6) => Instr::AddCarryImm(self.imm8()),
            (3,2,6) => Instr::SubImm(self.imm8()),
            (3,3,6) => Instr::SubCarryImm(self.imm8()),
            (3,4,6) => Instr::AndImm(self.imm8()),
            (3,5,6) => Instr::XorImm(self.imm8()),
            (3,6,6) => Instr::OrImm(self.imm8()),
            (3,7,6) => Instr::CompImm(self.imm8()),
            (3,_,7) => Instr::Restart(y),
            _ => unimplemented!("Unimplemented opcode: {:X} \nregs: {:?}", opcode, self.reg)
        }
    }
    fn imm8(&mut self) -> u8 {
        let value = self.bus.r8(self.reg.pc);
        self.reg.pc += 1;
        value
    }
    fn imm16(&mut self) -> u16 {
        let value = self.bus.r16(self.reg.pc);
        self.reg.pc += 2;
        value
    }

    pub fn exec(&mut self, instr: Instr) -> u32 {
        match instr {
            Instr::Nop => 4,
            Instr::Stop => {
                self.stopped = true;
                4
            },
            Instr::StoreSP(addr) => {
                self.bus.w16(addr, self.reg.sp);
                20
            },
            Instr::JumpRelative(offset, cond) => {
                if self.test_cc(cond) {
                    self.reg.pc = (self.reg.pc as i16).wrapping_add(offset as i16) as u16;
                    12
                } else {
                    8
                }
            },
            Instr::LoadImm16(reg, imm) => {
                self.set_reg16(reg, imm);
                12
            },
            Instr::AddHL(reg) => {
                let hl = self.get_reg16(Reg16::HL);
                let rr = self.get_reg16(reg);
                let result = self.add16(hl, rr);
                self.set_reg16(Reg16::HL, result);
                8
            },
            Instr::StoreA(reg) => {
                let addr = self.get_indirect(reg);
                self.bus.w8(addr, self.reg.r8(Reg8::A));
                8
            },
            Instr::LoadA(reg) => {
                let addr = self.get_indirect(reg);
                self.reg.w8(Reg8::A, self.bus.r8(addr));
                8
            },
            Instr::Inc16(reg) => {
                let result = self.get_reg16(reg).wrapping_add(1);
                self.set_reg16(reg, result);
                8
            },
            Instr::Dec16(reg) => {
                let result = self.get_reg16(reg).wrapping_sub(1);
                self.set_reg16(reg, result);
                8
            },
            Instr::Inc8(reg) => {
                let value = self.get_reg8(reg);
                let result = value.wrapping_add(1);
                self.set_reg8(reg, result);
                self.reg.f_z = result == 0;
                self.reg.f_n = false;
                self.reg.f_h = value & 0xf == 0xf;
                if reg == Reg8::HL { 12 } else { 4 }
            },
            Instr::Dec8(reg) => {
                let value = self.get_reg8(reg);
                let result = value.wrapping_sub(1);
                self.set_reg8(reg, result);
                self.reg.f_z = result == 0;
                self.reg.f_n = true;
                self.reg.f_h = value & 0xf == 0;
                if reg == Reg8::HL { 12 } else { 4 }
            },
            Instr::LoadImm8(reg, imm) => {
                self.set_reg8(reg, imm);
                if reg == Reg8::HL { 12 } else { 8 }
            },
            Instr::RotateALeft => {
                self.rotate(Reg8::A, |a,_c| (a.rotate_left(1), a & 0x80 != 0));
                self.reg.f_z = false;
                4
            },
            Instr::RotateALeftCarry => {
                self.rotate(Reg8::A, |a,c| (a << 1 | if c { 1 } else { 0 }, a & 0x80 == 0));
                self.reg.f_z = false;
                4
            },
            Instr::RotateARight => {
                self.rotate(Reg8::A, |a,_c| (a.rotate_right(1), a & 0x01 != 0));
                self.reg.f_z = false;
                4
            },
            Instr::RotateARightCarry => {
                self.rotate(Reg8::A, |a,c| (a >> 1 | if c { 0x80 } else { 0 }, a & 0x1 == 0));
                self.reg.f_z = false;
                4
            },
            Instr::DecimalAdjust => {
                let a = self.reg.r8(Reg8::A);

                let result = if self.reg.f_n {
                    let corr_lo = if self.reg.f_h { 0x6 } else { 0x0 };
                    let corr_hi = if self.reg.f_c { 0x60 } else { 0x0 };
                    a.wrapping_sub(corr_hi | corr_lo)
                } else {
                    let corr_lo = if self.reg.f_h || a & 0xf > 9 { 0x6 } else { 0x0 };
                    self.reg.f_c = self.reg.f_c || a > 0x99;
                    let corr_hi = if self.reg.f_c { 0x6 } else { 0x0 };
                    a.wrapping_add(corr_hi | corr_lo)
                };
                self.reg.f_z = result == 0;
                self.reg.f_h = false;
                self.reg.w8(Reg8::A, result);
                4
            },
            Instr::Complement => {
                let complemented = !self.reg.r8(Reg8::A);
                self.reg.w8(Reg8::A, complemented);
                4
            },
            Instr::ComplementCarry => {
                let complemented = !self.reg.f_c;
                self.reg.set_flags_nhc(false, false, complemented);
                4
            },
            Instr::SetCarry => {
                self.reg.set_flags_nhc(false, false, true);
                4
            },
            Instr::Halt => {
                self.halted = true;
                4
            },
            Instr::RegCopy(to,from) => {
                let val = self.get_reg8(from);
                self.set_reg8(to, val);
                if to == Reg8::HL || from == Reg8::HL { 8 } else { 4 }
            },
            Instr::Add(reg) | Instr::AddCarry(reg) => {
                let a = self.reg.r8(Reg8::A);
                let operand = self.get_reg8(reg);
                let is_carry = match instr { Instr::AddCarry(_) => true, _ => false };
                let result = self.add8(a, operand, is_carry);
                self.reg.w8(Reg8::A, result);
                if reg == Reg8::HL { 8 } else { 4 }
            },
            Instr::AddImm(imm) | Instr::AddCarryImm(imm) => {
                let a = self.reg.r8(Reg8::A);
                let is_carry = match instr {
                    Instr::AddCarryImm(_) => true,
                    _ => false
                };
                let result = self.add8(a, imm, is_carry);
                self.reg.w8(Reg8::A, result);
                4
            },
            Instr::Sub(reg) | Instr::SubCarry(reg) | Instr::Comp(reg) => {
                let a = self.reg.r8(Reg8::A);
                let operand = self.get_reg8(reg);
                let (is_carry, write_back) = match instr {
                    Instr::Sub(_) => (false, true),
                    Instr::SubCarry(_) => (true, true),
                    Instr::Comp(_) => (false, false),
                    _ => unreachable!()
                };
                let result = self.sub8(a, operand, is_carry);
                if write_back {
                    self.reg.w8(Reg8::A, result);
                }
                if reg == Reg8::HL { 8 } else { 4 }
            },
            Instr::SubImm(imm) | Instr::SubCarryImm(imm) | Instr::CompImm(imm) => {
                let a = self.reg.r8(Reg8::A);
                let (is_carry, write_back) = match instr {
                    Instr::SubImm(_) => (false, true),
                    Instr::SubCarryImm(_) => (true, true),
                    Instr::CompImm(_) => (false, false),
                    _ => unreachable!()
                };
                let result = self.sub8(a, imm, is_carry);
                if write_back {
                    self.reg.w8(Reg8::A, result);
                }
                4
            },
            Instr::And(reg) => {
                let value = self.get_reg8(reg);
                self.logical(value, |a,b| a & b, true);
                if reg == Reg8::HL { 8 } else { 4 }
            },
            Instr::Or(reg) => {
                let value = self.get_reg8(reg);
                self.logical(value, |a,b| a | b, true);
                if reg == Reg8::HL { 8 } else { 4 }
            },
            Instr::Xor(reg) => {
                let value = self.get_reg8(reg);
                self.logical(value, |a,b| a ^ b, true);
                if reg == Reg8::HL { 8 } else { 4 }
            },
            Instr::AndImm(imm) => {
                self.logical(imm, |a,b| a & b, true);
                4
            },
            Instr::OrImm(imm) => {
                self.logical(imm, |a,b| a | b, true);
                4
            },
            Instr::XorImm(imm) => {
                self.logical(imm, |a,b| a ^ b, true);
                4
            },
            Instr::Ret(cond) => {
                if self.test_cc(cond) {
                    self.ret();
                    20
                } else {
                    8
                }
            },
            Instr::RetI => {
                self.ret();
                self.interrupts = true;
                16
            },
            Instr::StoreIO(offset) => {
                let addr = 0xff00 + offset as u16;
                self.bus.w8(addr, self.reg.r8(Reg8::A));
                12
            },
            Instr::LoadIO(offset) => {
                let addr = 0xff00 + offset as u16;
                self.reg.w8(Reg8::A, self.bus.r8(addr));
                12
            },
            Instr::StackAdjust(disp) => {
                let sp = self.reg.sp;
                self.reg.sp = self.add16(sp, disp as u16);
                self.reg.f_z = false;
                16
            },
            Instr::LoadLocalAddr(disp) => {
                let sp = self.reg.sp;
                let result = self.add16(sp, disp as u16);
                self.reg.w16(Reg16::HL, result);
                self.reg.f_z = false;
                12
            },
            Instr::Pop(reg) => {
                let value = self.bus.r16(self.reg.sp);
                self.reg.sp += 2;
                self.reg.w16(reg, value);
                12
            },
            Instr::Push(reg) => {
                self.reg.sp -= 2;
                self.bus.w16(self.reg.sp, self.reg.r16(reg));
                12
            },
            Instr::Jump(dest, cond) => {
                if self.test_cc(cond) {
                    self.reg.pc = dest;
                    16
                } else {
                    12
                }
            },
            Instr::JumpHL => {
                let hl = self.reg.r16(Reg16::HL);
                self.reg.pc = hl;
                4
            },
            Instr::LoadStackHL => {
                let hl = self.reg.r16(Reg16::HL);
                self.reg.sp = hl;
                4
            },
            Instr::StoreIOC => {
                let addr = 0xff00 + self.reg.r8(Reg8::C) as u16;
                self.bus.w8(addr, self.reg.r8(Reg8::A));
                12
            },
            Instr::LoadIOC => {
                let addr = 0xff00 + self.reg.r8(Reg8::C) as u16;
                self.reg.w8(Reg8::A, self.bus.r8(addr));
                12
            },
            Instr::StoreGlobal(addr) => {
                self.bus.w8(addr, self.reg.r8(Reg8::A));
                16
            },
            Instr::LoadGlobal(addr) => {
                self.reg.w8(Reg8::A, self.bus.r8(addr));
                16
            },
            Instr::DisableInterrupts => {
                self.interrupts_buffer = false;
                4
            },
            Instr::EnableInterrupts => {
                self.interrupts_buffer = true;
                4
            },
            Instr::Call(dest, cond) => {
                if self.test_cc(cond) {
                    self.call(dest);
                    24
                } else {
                    12
                }
            },
            Instr::Restart(index) => {
                self.call(index as u16 * 8);
                16
            },
            Instr::Bit(reg, bit) => {
                let value = self.get_reg8(reg);
                self.reg.f_z = value & (1 << bit) == 0;
                self.reg.f_n = false;
                self.reg.f_h = true;
                if reg == Reg8::HL { 12 } else { 8 }
            },
            Instr::Set(reg, bit) => {
                let value = self.get_reg8(reg);
                let result = value | (1 << bit);
                self.set_reg8(reg, result);
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::Reset(reg, bit) => {
                let value = self.get_reg8(reg);
                let result = value & !(1 << bit);
                self.set_reg8(reg, result);
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::RotateLeft(reg) => {
                self.rotate(reg, |a,_c| (a.rotate_left(1), a & 0x80 != 0));
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::RotateRight(reg) => {
                self.rotate(reg, |a,_c| (a.rotate_right(1), a & 0x01 != 0));
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::RotateLeftCarry(reg) => {
                self.rotate(reg, |a,c| (a << 1 | if c { 1 } else { 0 }, a & 0x80 != 0));
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::RotateRightCarry(reg) => {
                self.rotate(reg, |a,c| (a >> 1 | if c { 0x80 } else { 0 }, a & 1 != 0));
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::ShiftLeft(reg) => {
                self.rotate(reg, |a,_c| (a << 1, a & 0x80 != 0));
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::ShiftRightLogical(reg) => {
                self.rotate(reg, |a,_c| (a >> 1, a & 1 != 0));
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::ShiftRightArith(reg) => {
                self.rotate(reg, |a,_c| (((a as i8) >> 1) as u8, a & 1 != 0));
                if reg == Reg8::HL { 16 } else { 8 }
            },
            Instr::SwapBytes(reg) => {
                self.rotate(reg, |a,_c| (a.rotate_left(4), false));
                if reg == Reg8::HL { 16 } else { 8 }
            }
        }
    }
    fn get_reg8(&self, reg: Reg8) -> u8 {
        match reg {
            Reg8::HL => self.bus.r8(self.reg.r16(Reg16::HL)),
            _ => self.reg.r8(reg)
        }
    }
    fn set_reg8(&mut self, reg: Reg8, value: u8) {
        match reg {
            Reg8::HL => self.bus.w8(self.reg.r16(Reg16::HL), value),
            _ => self.reg.w8(reg, value)
        }
    }
    fn get_reg16(&self, reg: Reg16) -> u16 {
        self.reg.r16(reg)
    }
    fn set_reg16(&mut self, reg: Reg16, value: u16) {
        self.reg.w16(reg, value)
    }

    fn test_cc(&self, cc: Cond) -> bool {
        match cc {
            Cond::Z => self.reg.f_z,
            Cond::NZ => !self.reg.f_z,
            Cond::C => self.reg.f_c,
            Cond::NC => !self.reg.f_c,
            Cond::Always => true
        }
    }
    fn get_indirect(&mut self, reg: Indirect) -> u16 {
        match reg {
            Indirect::BC => self.reg.r16(Reg16::BC),
            Indirect::DE => self.reg.r16(Reg16::DE),
            Indirect::HLPlus => {
                let result = self.reg.r16(Reg16::HL);
                self.reg.w16(Reg16::HL, result.wrapping_add(1));
                result
            },
            Indirect::HLMinus => {
                        let result = self.reg.r16(Reg16::HL);
                self.reg.w16(Reg16::HL, result.wrapping_sub(1));
                result
            },
        }
    }
    fn add8(&mut self, a: u8, b: u8, carry_in: bool) -> u8 {
        let b = if carry_in { b + 1 } else { b };
        let (result, carry_out) = a.overflowing_add(b);
        let half_carry = ((a & 0xf) + (b & 0xf)) & 0x10 != 0;
        self.reg.set_flags_nhc(false, half_carry, carry_out);
        self.reg.f_z = result == 0;
        result
    }
    fn sub8(&mut self, a: u8, b: u8, carry_in: bool) -> u8 {
        let b = if carry_in { b + 1 } else { b };
        let (result, carry_out) = a.overflowing_sub(b);
        let half_carry = a & 0xf < b & 0xf;
        self.reg.set_flags_nhc(true, half_carry, carry_out);
        self.reg.f_z = result == 0;
        result
    }
    fn add16(&mut self, a: u16, b: u16) -> u16 {
        let (result,carry) = a.overflowing_add(b);
        let half_carry = ((a & 0xff) + (b & 0xff)) & 0x100 != 0;
        self.reg.set_flags_nhc(false, half_carry, carry);
        result
    }
    fn rotate<F>(&mut self, reg: Reg8, f: F)
    where F: FnOnce(u8, bool) -> (u8, bool) {
        let value = self.get_reg8(reg);
        let (result, carry) = f(value, self.reg.f_c);
        self.reg.f_z = result != 0;
        self.reg.set_flags_nhc(false, false, carry);
        self.set_reg8(reg, result);
    }
    fn logical<F>(&mut self, value: u8, f: F, half_carry: bool)
    where F: FnOnce(u8,u8) -> u8 {
        let a = self.reg.r8(Reg8::A);
        let result = f(a,value);
        self.reg.f_z = result == 0;
        self.reg.set_flags_nhc(false, half_carry, false);
        self.reg.w8(Reg8::A, result);
    }
    fn ret(&mut self) {
        let addr = self.bus.r16(self.reg.sp);
        self.reg.sp += 2;
        self.reg.pc = addr;
    }
    fn call(&mut self, dest: u16) {
        self.reg.sp -= 2;
        self.bus.w16(self.reg.sp, self.reg.pc);
        self.reg.pc = dest;
    }
}
