enum Clock {
    External, Internal
}

pub struct Serial {
    data: u8,
    start: bool,
    clock: Clock
}

impl Serial {
    pub fn new() -> Self {
        Serial {
            data: 0,
            start: false,
            clock: Clock::External
        }
    }
    pub fn get_transfer(&self) -> u8 {
        self.data
    }
    pub fn set_transfer(&mut self, value: u8) {
        self.data = value;
    }
    pub fn get_control(&self) -> u8 {
        let mut result = 0b01111110;
        if self.start { result |= 0x80; }
        if let Clock::Internal = self.clock { result |= 0x01; }
        result
    }
    pub fn set_control(&mut self, value: u8) {
        if value & 0x80 != 0 {
            //unimplemented!("Starting serial transfer");
            print!("{}", self.data as char);
        }
        self.clock = if value & 0x01 == 0 { Clock::External } else { Clock::Internal };
    }
}
