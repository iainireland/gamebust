use cpu::Interrupt;

pub struct Timer {
    divider: u16,
    counter: u8,
    modulo: u8,
    tac_enabled: bool,
    frequency: u8,
    cycles: u32
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            divider: 0,
            counter: 0,
            modulo: 0,
            tac_enabled: false,
            frequency: 0,
            cycles: 0,
        }
    }
    pub fn update(&mut self, cycles: u32, irq: &mut Interrupt) {
        self.divider = self.divider.wrapping_add(cycles as u16);

        if self.tac_enabled {
            self.cycles += cycles;
            let target = match self.frequency {
                0 => 1024, 1 => 16, 2 => 64, 3 => 256,
                _ => unreachable!("Invalid timer frequency")
            };
            if self.cycles >= target {
                let (c, carry) = self.counter.overflowing_add(1);
                self.counter = c;
                if carry {
                    self.counter = self.modulo;
                    irq.insert(Interrupt::TIMER);
                }
            }
        }
    }
    #[inline(always)]
    pub fn get_divider(&self) -> u8 {
        (self.divider >> 8) as u8
    }
    #[inline(always)]
    pub fn reset_divider(&mut self) {
        self.divider = 0;
    }
    #[inline(always)]
    pub fn get_counter(&self) -> u8 {
        self.counter
    }
    #[inline(always)]
    pub fn set_counter(&mut self, value: u8) {
        self.counter = value;
    }
    #[inline(always)]
    pub fn get_modulo(&self) -> u8 {
        self.modulo
    }
    #[inline(always)]
    pub fn set_modulo(&mut self, value: u8) {
        self.modulo = value;
    }
    #[inline(always)]
    pub fn get_control(&self) -> u8 {
        let mut result = 0b11111000 | self.frequency;
        if self.tac_enabled { result |= 1 << 2; }
        result
    }
    #[inline(always)]
    pub fn set_control(&mut self, value: u8) {
        self.tac_enabled = value & (1 << 2) != 0;
        self.frequency = value & 0x3;
    }
}
