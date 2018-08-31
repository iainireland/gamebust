use sdl2::keyboard::Scancode;

pub struct Joypad {
    button_state: u8,
    input_lines: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            button_state: 0xff,
            input_lines: 0
        }
    }
    pub fn key_down(&mut self, button: Button) {
        self.button_state &= !button.value();
    }
    pub fn key_up(&mut self, button: Button) {
        self.button_state |= button.value();
    }
    pub fn read(&self) -> u8 {
        let low_bits =
            if self.input_lines & 0x10 == 0 {
                self.button_state & 0x0f
            } else if self.input_lines & 0x20 == 0 {
                self.button_state >> 4
            } else {
                0x0f
            };
        let high_bits = 0xc0 | (self.input_lines & 0x30);
        high_bits | low_bits
    }
    pub fn write(&mut self, val: u8) {
        self.input_lines = val;
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
    pub fn from_scancode(scancode: Scancode) -> Option<Self> {
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
}
