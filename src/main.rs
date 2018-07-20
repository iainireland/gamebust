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
    interrupts: bool,
    interrupts_buffer: bool,
    halted: bool,
    stopped: bool
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
            interrupts: false,
            interrupts_buffer: false,
            halted: false,
            stopped: false,
        }
    }
    pub fn load_boot_rom(&mut self) {
        let boot_rom = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/boot.rom"));
        self.mem.write(0, boot_rom);
    }
    pub fn step(&mut self) {
        let opcode = self.imm8();
        println!("{:x}: {:x}", self.reg.pc, opcode);

        let x = opcode >> 6;
        let y = (opcode >> 3) & 7;
        let z = opcode & 7;

        let mut extra_cycles = 0;
        let cycles = match (x,y,z) {
            (0,0,0) => op!("NOP", 4, {}),
            (0,1,0) => op!("LD (addr), SP", 20, {
                let addr = self.imm16();
                self.mem.w16(addr, self.reg.sp);
            }),
            (0,2,0) => op!("STOP", 4, {
                self.stopped = true;
            }),
            (0,3,0) => op!("JR disp", 12, {
                let disp = self.imm8() as i8 as i16;
                self.reg.pc = (self.reg.pc as i16).wrapping_add(disp) as u16;
            }),
            (0,4...7,0) => op!("JR <cond>,disp", 8, {
                let disp = self.imm8() as i8 as i16;
                if self.test_cc(y-4) {
                    self.reg.pc = (self.reg.pc as i16).wrapping_add(disp) as u16;
                    extra_cycles = 4;
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
            (0,0...3,7) => op!("R[RL](C)A", 4, {
                let a = self.reg.r8(Reg8::A);
                let (result, carry) = match y {
                    0 => (a.rotate_left(1), a & 0x80 != 0),
                    1 => (a << 1 | if self.reg.f_c { 1 } else { 0 }, a & 0x80 != 0),
                    2 => (a.rotate_right(1), a & 0x1 != 0),
                    3 => (a >> 1 | if self.reg.f_c { 0x80 } else { 0 }, a & 0x1 != 0),
                    _ => unreachable!("Invalid rotation op")
                };
                self.reg.f_z = false;
                self.reg.set_flags_nhc(false, false, carry);
                self.reg.w8(Reg8::A, result);
            }),
            (0,4,7) => op!("DAA", 4, {
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
            }),
            (0,5,7) => op!("CPL", 4, {
                let complemented = !self.reg.r8(Reg8::A);
                self.reg.w8(Reg8::A, complemented);
            }),
            (0,6,7) => op!("SCF", 4, {
                self.reg.set_flags_nhc(false, false, true);
            }),
            (0,7,7) => op!("CCF", 4, {
                let complemented = !self.reg.f_c;
                self.reg.set_flags_nhc(false, false, complemented);
            }),
            (1,6,6) => op!("HALT", 4, {
                self.halted = true;
            }),
            (1,6,_) => op!("LD (HL),R", 8, {
                let addr = self.reg.r16(Reg16::HL);
                let val = self.reg.r8(CPU::get_reg8(z));
                self.mem.w8(addr, val);
            }),
            (1,_,6) => op!("LD R,(HL)", 8, {
                let val = self.mem.r8(self.reg.r16(Reg16::HL));
                self.reg.w8(CPU::get_reg8(y), val);
            }),
            (1,_,_) => op!("LD R1,R2", 4, {
                let val = self.reg.r8(CPU::get_reg8(z));
                self.reg.w8(CPU::get_reg8(y), val);
            }),
            (2,_,_) => op!("<op> A, (HL)/R", 8, {
                let operand = if z == 6 {
                    extra_cycles = 4;
                    self.mem.r8(self.reg.r16(Reg16::HL))
                } else {
                    self.reg.r8(CPU::get_reg8(z))
                };
                self.alu(y, operand);
            }),
            (3,0...3,0) => op!("RET <cond>", 8, {
                if self.test_cc(y) {
                    extra_cycles = 12;
                    self.ret();
                }
            }),
            (3,4,0) => op!("LD (0xFF00 + nn), A", 12, {
                let addr = 0xff00 + self.imm8() as u16;
                self.mem.w8(addr, self.reg.r8(Reg8::A));
            }),
            (3,5,0) => op!("ADD SP, disp", 16, {
                let sp = self.reg.sp;
                let disp = self.imm8() as i8 as i16 as u16;
                self.reg.sp = self.add16(sp, disp);
                self.reg.f_z = false;
            }),
            (3,6,0) => op!("LD A, (0xFF00 + nn)", 12, {
                let addr = 0xff00 + self.imm8() as u16;
                self.reg.w8(Reg8::A, self.mem.r8(addr));
            }),
            (3,7,0) => op!("LD HL, SP+disp", 12, {
                let sp = self.reg.sp;
                let disp = self.imm8() as i8 as i16 as u16;
                let result = self.add16(sp, disp);
                self.reg.w16(Reg16::HL, result);
                self.reg.f_z = false;
            }),
            (3,0,1)|(3,2,1)|(3,4,1)|(3,6,1) => op!("POP RR", 12, {
                let reg = CPU::get_reg16(y, false);
                let val = self.mem.r16(self.reg.sp);
                self.reg.sp += 2;
                self.reg.w16(reg,val);
            }),
            (3,1,1) => op!("RET", 16, {
                self.ret();
            }),
            (3,3,1) => op!("RETI", 16, {
                self.ret();
                self.interrupts = true;
            }),
            (3,5,1) => op!("JP HL", 4, {
                let hl = self.reg.r16(Reg16::HL);
                self.reg.pc = hl;
            }),
            (3,7,1) => op!("LD SP, HL", 8, {
                let hl = self.reg.r16(Reg16::HL);
                self.reg.sp = hl;
            }),
            (3,0...3,2) => op!("JP <cond>", 12, {
                let dest = self.imm16();
                if self.test_cc(y) {
                    extra_cycles = 4;
                    self.reg.pc = dest;
                }
            }),
            (3,4,2) => op!("LD (0xFF00 + C), A", 12, {
                let addr = 0xff00 + self.reg.r8(Reg8::C) as u16;
                self.mem.w8(addr, self.reg.r8(Reg8::A));
            }),
            (3,5,2) => op!("LD (nn), A", 16, {
                let addr = self.imm16();
                self.mem.w8(addr, self.reg.r8(Reg8::A));
            }),
            (3,6,2) => op!("LD A, (0xFF00 + C)", 12, {
                let addr = 0xff00 + self.reg.r8(Reg8::C) as u16;
                self.reg.w8(Reg8::A, self.mem.r8(addr));
            }),
            (3,7,2) => op!("LD A, (nn)", 16, {
                let addr = self.imm16();
                self.reg.w8(Reg8::A, self.mem.r8(addr));
            }),
            (3,0,3) => op!("JP", 16, {
                let dest = self.imm16();
                self.reg.pc = dest;
            }),
            (3,1,3) => op!("CB+", 0, {
                extra_cycles = self.cb_prefix();
            }),
            (3,6,3) => op!("DI", 4, {
                self.interrupts_buffer = false;
            }),
            (3,7,3) => op!("EI", 4, {
                self.interrupts_buffer = true;
            }),
            (3,0...3,4) => op!("CALL <cond>", 12, {
                let dest = self.imm16();
                if self.test_cc(y) {
                    extra_cycles = 12;
                    self.call(dest);
                }
            }),
            (3,0,5)|(3,2,5)|(3,4,5)|(3,6,5) => op!("PUSH RR", 16, {
                let reg = CPU::get_reg16(y, false);
                self.reg.sp -= 2;
                self.mem.w16(self.reg.sp, self.reg.r16(reg));
            }),
            (3,1,5) => op!("CALL", 24, {
                let dest = self.imm16();
                self.call(dest);
            }),
            (3,_,6) => op!("<op> A, (HL)/R", 8, {
                let operand = self.imm8();
                self.alu(y, operand);
            }),
            (3,_,7) => op!("RST", 16, {
                self.call(y as u16 * 8);
            }),
            _ => unimplemented!("Unimplemented opcode: {:X} \nregs: {:?}", opcode, self.reg)
        };
        let _ = cycles + extra_cycles;

    }
    fn cb_prefix(&mut self) -> u32 {
        let opcode = self.imm8();
        let op = opcode >> 6;
        let reg = opcode & 7;

        let (value,cycles) = if reg == 6 {
            (self.mem.r8(self.reg.r16(Reg16::HL)), 16)
        } else {
            (self.reg.r8(CPU::get_reg8(reg)), 8)
        };
        let result = match op {
            0 => { // Rotations
                let rot_op = (opcode >> 3) & 7;
                let (result, carry) = match rot_op {
                    0 => /*RLC*/ (value.rotate_left(1), value & 0x80 != 0),
                    1 => /*RRC*/ (value.rotate_right(1), value & 0x1 != 0),
                    2 => /*RL */ (value << 1 | if self.reg.f_c { 1 } else { 0 }, value & 0x80 != 0),
                    3 => /*RR */ (value >> 1 | if self.reg.f_c { 0x80 } else { 0 }, value & 0x1 != 0),
                    4 => /*SLA*/ (value << 1, value & 0x80 != 0),
                    5 => /*SRA*/ (((value as i8) >> 1) as u8, value & 0x01 != 0),
                    6 => /*SWP*/ (value.rotate_right(4), false),
                    7 => /*SRL*/ (value >> 1, value & 0x1 != 0),
                    _ => unreachable!("Invalid rotation op")
                };
                self.reg.f_c = carry;
                self.reg.set_flags_nhc(false, false, rot_op != 6);
                result
            },
            1 => { // BIT
                let bit = (opcode >> 3) & 7;
                self.reg.f_z = value & (1<<bit) == 0;
                self.reg.f_n = false;
                self.reg.f_h = true;
                return cycles; // early return; don't need to do writeback
            },
            2 => { // RES
                let bit = (opcode >> 3) & 7;
                value & !(1 << bit)
            },
            3 => { // SET
                let bit = (opcode >> 3) & 7;
                value & (1 << bit)
            },
            _ => unreachable!("Invalid CB instruction")
        };
        if reg == 6 {
            self.mem.w8(self.reg.r16(Reg16::HL), result);
        } else {
            self.reg.w8(CPU::get_reg8(reg), result);
        }
        cycles
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
    fn alu(&mut self, opcode: u8, operand: u8) {
        let a = self.reg.r8(Reg8::A);
        let operand = if (opcode == 1 || opcode == 3) && self.reg.f_c {
            operand.wrapping_add(1)
        } else {
            operand
        };
        let result = match opcode {
            0|1 => self.add8(a, operand),
            2|3|7 => self.sub8(a, operand),
            4 => { self.reg.set_flags_nhc(false,true,false); a & operand },
            5 => { self.reg.set_flags_nhc(false,false,false); a ^ operand },
            6 => { self.reg.set_flags_nhc(false,false,false); a | operand },
            _ => unreachable!("Invalid ops field")
                };
        self.reg.f_z = result == 0;
        if opcode != 7 {
            self.reg.w8(Reg8::A, result);
        }
    }
    fn add8(&mut self, a: u8, b: u8) -> u8 {
        let (result, carry) = a.overflowing_add(b);
        let half_carry = ((a & 0xf) + (b & 0xf)) & 0x10 != 0;
        self.reg.set_flags_nhc(false, half_carry, carry);
        result
    }
    fn sub8(&mut self, a: u8, b: u8) -> u8 {
        let (result, carry) = a.overflowing_sub(b);
        let half_carry = a & 0xf < b & 0xf;
        self.reg.set_flags_nhc(true, half_carry, carry);
        result
    }
    fn add16(&mut self, a: u16, b: u16) -> u16 {
        let (result,carry) = a.overflowing_add(b);
        let half_carry = ((a & 0xff) + (b & 0xff)) & 0x100 != 0;
        self.reg.set_flags_nhc(false, half_carry, carry);
        result
    }
    fn get_indirect_reg(&mut self, bits: u8) -> Reg16 {
        match bits {
            0 => Reg16::BC,
            2 => Reg16::DE,
            4 => {
                let val = self.reg.r16(Reg16::HL).wrapping_add(1);
                self.reg.w16(Reg16::HL, val);
                Reg16::HL
            },
            6 => {
                let val = self.reg.r16(Reg16::HL).wrapping_sub(1);
                self.reg.w16(Reg16::HL, val);
                Reg16::HL
            },
            _ => unreachable!("Invalid indirect reg field")
        }
    }
    fn inc8(&mut self, a: u8) -> u8 {
        let result = a.wrapping_add(1);
        self.reg.f_z = result == 0;
        self.reg.f_n = false;
        self.reg.f_h = a & 0xf == 0xf;
        result
    }
    fn dec8(&mut self, a: u8) -> u8 {
        let result = a.wrapping_sub(1);
        self.reg.f_z = result == 0;
        self.reg.f_n = true;
        self.reg.f_h = a & 0xf == 0;
        result
    }
    fn ret(&mut self) {
        let addr = self.mem.r16(self.reg.sp);
        self.reg.sp += 2;
        self.reg.pc = addr;
    }
    fn call(&mut self, dest: u16) {
        self.reg.sp -= 2;
        self.mem.w16(self.reg.sp, self.reg.pc);
        self.reg.pc = dest;
    }
}
