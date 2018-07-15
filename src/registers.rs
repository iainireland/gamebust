#[derive(Copy,Clone,Debug)]
pub struct Registers {
    a: u8,
    pub f_z: bool,
    pub f_n: bool,
    pub f_h: bool,
    pub f_c: bool,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pub sp: u16,
    pub pc: u16
}

#[derive(Copy,Clone,Debug)]
pub enum Reg8 {
    A, B, C, D, E, H, L
}

#[derive(Copy,Clone,Debug)]
pub enum Reg16 {
    AF, BC, DE, HL, SP
}

impl Registers {
    pub fn new() -> Self {
        Registers { a: 0,
                    f_z: false,
                    f_n: false,
                    f_h: false,
                    f_c: false,
                    b: 0,
                    c: 0,
                    d: 0,
                    e: 0,
                    h: 0,
                    l: 0,
                    sp: 0,
                    pc: 0 }
    }
    pub fn r8(&self, reg: Reg8) -> u8 {
        match reg {
            Reg8::A => self.a,
            Reg8::B => self.b,
            Reg8::C => self.c,
            Reg8::D => self.d,
            Reg8::E => self.e,
            Reg8::H => self.h,
            Reg8::L => self.l,
        }
    }
    pub fn w8(&mut self, reg: Reg8, value: u8) {
        match reg {
            Reg8::A => self.a = value,
            Reg8::B => self.b = value,
            Reg8::C => self.c = value,
            Reg8::D => self.d = value,
            Reg8::E => self.e = value,
            Reg8::H => self.h = value,
            Reg8::L => self.l = value,
        }
    }
    pub fn r16(&self, reg: Reg16) -> u16 {
        match reg {
            Reg16::AF => (self.a as u16) << 8 | self.get_f() as u16,
            Reg16::BC => (self.b as u16) << 8 | self.c as u16,
            Reg16::DE => (self.d as u16) << 8 | self.e as u16,
            Reg16::HL => (self.h as u16) << 8 | self.l as u16,
            Reg16::SP => self.sp
        }
    }
    pub fn w16(&mut self, reg: Reg16, value: u16) {
        let hi = ((value & 0xff00) >> 8) as u8;
        let lo = (value & 0xff00) as u8;
        match reg {
            Reg16::AF => { self.a = hi;
                           self.f_z = lo & 0x80 != 0;
                           self.f_n = lo & 0x40 != 0;
                           self.f_h = lo & 0x20 != 0;
                           self.f_c = lo & 0x10 != 0;
            },
            Reg16::BC => { self.b = hi; self.c = lo; },
            Reg16::DE => { self.d = hi; self.e = lo; },
            Reg16::HL => { self.h = hi; self.l = lo; },
            Reg16::SP => { self.sp = value; },
        }
    }
    fn get_f(&self) -> u8 {
        let mut result = 0;
        if self.f_z { result &= 0x80; }
        if self.f_n { result &= 0x40; }
        if self.f_h { result &= 0x20; }
        if self.f_c { result &= 0x10; }
        result
    }
}
