use std::fmt;

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Reg8 {
    A, B, C, D, E, H, L, HL
}
impl Reg8 {
    pub fn from(bits: u8) -> Self {
        match bits {
            0 => Reg8::B,
            1 => Reg8::C,
            2 => Reg8::D,
            3 => Reg8::E,
            4 => Reg8::H,
            5 => Reg8::L,
            6 => Reg8::HL,
            7 => Reg8::A,
            _ => unreachable!("Invalid reg8 field")
        }
    }
}
impl fmt::Display for Reg8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Reg8::A => write!(f, "A"),
            Reg8::B => write!(f, "B"),
            Reg8::C => write!(f, "C"),
            Reg8::D => write!(f, "D"),
            Reg8::E => write!(f, "E"),
            Reg8::H => write!(f, "H"),
            Reg8::L => write!(f, "L"),
            Reg8::HL => write!(f, "(HL)"),
        }
    }
}

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Reg16 {
    AF, BC, DE, HL, SP
}

impl Reg16 {
    pub fn from(bits: u8, use_sp: bool) -> Reg16 {
        match bits {
            0|1 => Reg16::BC,
            2|3 => Reg16::DE,
            4|5 => Reg16::HL,
            6|7 => if use_sp { Reg16::SP } else { Reg16::AF },
            _ => unreachable!("Invalid reg16 field")
        }
    }
}
impl fmt::Display for Reg16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Reg16::BC => write!(f, "BC"),
            Reg16::DE => write!(f, "DE"),
            Reg16::HL => write!(f, "HL"),
            Reg16::SP => write!(f, "SP"),
            Reg16::AF => write!(f, "AF"),
        }
    }
}

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Indirect {
    BC, DE, HLPlus, HLMinus
}

impl Indirect {
    pub fn from(bits: u8) -> Indirect {
        match bits {
            0|1 => Indirect::BC,
            2|3 => Indirect::DE,
            4|5 => Indirect::HLPlus,
            6|7 => Indirect::HLMinus,
            _ => unreachable!("Invalid indirect reg field")
        }
    }
}
impl fmt::Display for Indirect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Indirect::BC => write!(f, "(BC)"),
            Indirect::DE => write!(f, "(DE)"),
            Indirect::HLPlus => write!(f, "(HL+)"),
            Indirect::HLMinus => write!(f, "(HL-)"),
        }
    }
}

#[derive(Debug)]
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
            Reg8::HL => panic!("HL needs special handling")
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
            Reg8::HL => panic!("HL needs special handling")
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
        let lo = (value & 0x00ff) as u8;
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
    pub fn set_flags_nhc(&mut self, n: bool, h: bool, c: bool) {
        self.f_n = n;
        self.f_h = h;
        self.f_c = c;
    }
    fn get_f(&self) -> u8 {
        let mut result = 0;
        if self.f_z { result |= 0x80; }
        if self.f_n { result |= 0x40; }
        if self.f_h { result |= 0x20; }
        if self.f_c { result |= 0x10; }
        result
    }
}
impl fmt::Display for Registers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02x}|{:02x} {:02x}|{:02x} {:02x}|{:02x} {:02x}|{:02x} {:04x}|{:04x}",
               self.a, self.get_f(),
               self.b, self.c,
               self.d, self.e,
               self.h, self.l,
               self.pc, self.sp)
    }
}
