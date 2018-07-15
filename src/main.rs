extern crate sdl2;

mod registers;
mod memory;

use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
// use sdl2::rect::Rect;

use registers::{Registers,Reg8,Reg16};
use memory::Memory;

fn main() {
    let scale = 5;
    let width = 144;
    let height = 160;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("Gamebust",
                                        width * scale,
                                        height * scale)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_draw_color(Color::RGB(0,0,0));
    canvas.clear();

    let mut events = sdl_context.event_pump().unwrap();
    let mut cpu = CPU::new();
    cpu.load_boot_rom();

    'eventloop: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { scancode: Some(Scancode::Escape), .. } =>
                    break 'eventloop,
                _ => {}
            }
        }
        cpu.step();
    }
}

pub struct CPU {
    reg: Registers,
    mem: Memory,
}

macro_rules! op {
    ($description:expr, $cycles:expr, $code:block) =>
        ({$code; $cycles})
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            reg: Registers::new(),
            mem: Memory::new(),
        }
    }
    pub fn load_boot_rom(&mut self) {
        let boot_rom = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/boot.rom"));
        println!("boot_rom: {:?}", boot_rom.to_vec());
        self.mem.write(0, boot_rom);
    }
    pub fn step(&mut self) {
        let opcode = self.mem.r8(self.reg.pc);
        println!("{:x}: {:x}", self.reg.pc, opcode);

        self.reg.pc += 1;

        let x = opcode >> 6;
        let y = (opcode >> 3) & 7;
        let z = opcode & 7;

        println!("x: {},y: {}, z: {}, p: {}, q: {}",x,y,z,y >> 1,y % 2);

        let mut taken_cycles = 0;
        let cycles = match (x,y,z) {
            (0,0,0) => op!("NOP", 4, {}),
            (0,1,0) => op!("LD (addr), SP", 20, {
                let addr = self.imm16();
                self.mem.w16(addr, self.reg.sp);
            }),
            (0,2,0) => op!("STOP", 4, {
                unimplemented!();
            }),
            (0,3,0) => op!("JR disp", 12, {
                let disp = self.imm8() as i16;
                self.reg.pc = (self.reg.pc as i16).wrapping_add(disp) as u16;
            }),
            (0,4...7,0) => op!("JR <cond>,disp", 8, {
                let disp = self.imm8() as i16;
                if self.test_cc(y-4) {
                    self.reg.pc = (self.reg.pc as i16).wrapping_add(disp) as u16;
                    taken_cycles = 4;
                }
            }),
            (0,0,1)|(0,2,1)|(0,4,1)|(0,6,1) => op!("LD RR, imm", 12, {
                let imm = self.imm16();
                self.reg.w16(CPU::get_reg16(y, true), imm);
            }),
            (0,_,1) => op!("ADD HL, RR", 8, {
                let hl = self.reg.r16(Reg16::HL);
                let rr = self.reg.r16(CPU::get_reg16(y-1, true));
                let result = self.add16(hl, rr);
                self.reg.w16(Reg16::HL, result);
            }),
            (0,0,2)|(0,2,2)|(0,4,2)|(0,6,2) => op!("LD (RR),A", 8, {
                let reg = self.get_indirect_reg(y);
                self.mem.w8(self.reg.r16(reg), self.reg.r8(Reg8::A));
            }),
            (0,_,2) => op!("LD A,(RR)", 8, {
                let reg = self.get_indirect_reg(y-1);
                let val = self.mem.r8(self.reg.r16(reg));
                self.reg.w8(Reg8::A, val);
            }),
            (0,0,3)|(0,2,3)|(0,4,3)|(0,6,3) => op!("INC RR", 8,  {
                let reg = CPU::get_reg16(y, true);
                let val = self.reg.r16(reg).wrapping_add(1);
                self.reg.w16(reg, val);
            }),
            (0,_,3) => op!("DEC RR", 8, {
                let reg = CPU::get_reg16(y-1, true);
                let val = self.reg.r16(reg).wrapping_sub(1);
                self.reg.w16(reg, val);
            }),
            (0,6,4) => op!("INC (HL)", 12, {
                let addr = self.reg.r16(Reg16::HL);
                let val = self.mem.r8(addr);
                let result = self.inc8(val);
                self.mem.w8(addr, result);
            }),
            (0,_,4) => op!("INC R", 4, {
                let reg = CPU::get_reg8(y);
                let val = self.reg.r8(reg);
                let result = self.inc8(val);
                self.reg.w8(reg, result);
            }),
            (0,6,5) => op!("DEC (HL)", 12, {
                let addr = self.reg.r16(Reg16::HL);
                let val = self.mem.r8(addr);
                let result = self.dec8(val);
                self.mem.w8(addr, result);
            }),
            (0,_,5) => op!("DEC R", 4, {
                let reg = CPU::get_reg8(y);
                let val = self.reg.r8(reg);
                let result = self.dec8(val);
                self.reg.w8(reg, result);
            }),
            (0,6,6) => op!("LD (HL), imm", 12, {
                let addr = self.reg.r16(Reg16::HL);
                let imm = self.imm8();
                self.mem.w8(addr, imm);
            }),
            (0,_,6) => op!("LD R, imm", 8, {
                let reg = CPU::get_reg8(y);
                let imm = self.imm8();
                self.reg.w8(reg, imm);
            }),
            _ => unimplemented!("Unimplemented opcode: {:x} regs: {:?}", opcode, self.reg)
        };
        let _ = cycles + taken_cycles;

    }
    fn imm8(&mut self) -> u8 {
        let val = self.mem.r8(self.reg.pc);
        self.reg.pc += 1;
        val
    }
    fn imm16(&mut self) -> u16 {
        let val = self.mem.r16(self.reg.pc);
        self.reg.pc += 2;
        val
    }
    fn get_reg8(bits: u8) -> Reg8 {
        match bits {
            0 => Reg8::B,
            1 => Reg8::C,
            2 => Reg8::D,
            3 => Reg8::E,
            4 => Reg8::H,
            5 => Reg8::L,
            6 => panic!("Need special handling for (HL)"),
            7 => Reg8::A,
            _ => unreachable!("Invalid reg8 field")
        }
    }
    fn get_reg16(bits: u8, use_sp: bool) -> Reg16 {
        match bits {
            0 => Reg16::BC,
            2 => Reg16::DE,
            4 => Reg16::HL,
            6 => if use_sp { Reg16::SP } else { Reg16::AF },
            _ => unreachable!("Invalid reg16 field")
        }
    }
    fn test_cc(&self, cc: u8) -> bool {
        match cc {
            0 => self.reg.f_z,
            1 => !self.reg.f_z,
            2 => self.reg.f_c,
            3 => !self.reg.f_c,
            _ => unreachable!("Invalid condition code")
        }
    }
    fn add16(&mut self, a: u16, b: u16) -> u16 {
        let (result,carry) = a.overflowing_add(b);
        let half_carry = ((a & 0xff) + (b & 0xff)) & 0x100 != 0;
        self.reg.f_n = false;
        self.reg.f_c = carry;
        self.reg.f_h = half_carry;
        result
    }
    fn get_indirect_reg(&mut self, bits: u8) -> Reg16 {
        match bits {
            0 => Reg16::BC,
            1 => Reg16::DE,
            2 => {
                let val = self.reg.r16(Reg16::HL).wrapping_add(1);
                self.reg.w16(Reg16::HL, val);
                Reg16::HL
            },
            3 => {
                let val = self.reg.r16(Reg16::HL).wrapping_sub(1);
                self.reg.w16(Reg16::HL, val);
                Reg16::HL
            },
            _ => unreachable!("Invalid indirect reg field")
        }
    }
    fn inc8(&mut self, a: u8) -> u8 {
        let (result,carry) = a.overflowing_add(1);
        self.reg.f_z = result == 0;
        self.reg.f_n = false;
        self.reg.f_c = carry;
        result
    }
    fn dec8(&mut self, a: u8) -> u8 {
        let (result,carry) = a.overflowing_sub(1);
        self.reg.f_z = result == 0;
        self.reg.f_n = true;
        self.reg.f_c = carry;
        result
    }
}
