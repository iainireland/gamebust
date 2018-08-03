#[derive(Debug)]
enum MemoryController {
    None, MBC1, MBC2, //MBC3, MBC5
}

impl Default for MemoryController {
    fn default() -> Self { MemoryController::None }
}

#[derive(Default,Debug)]
struct CartridgeMode {
    mbc: MemoryController,
    has_ram: bool,
    has_batt: bool,
    // has_sram: bool,
    // has_rumble: bool
}

impl CartridgeMode {
    pub fn new(spec: u8) -> Option<Self> {
        println!("SPEC: {}", spec);
        match spec {
            0x0 => Some(Default::default()),
            0x1 => Some(CartridgeMode { mbc: MemoryController::MBC1,
                                        ..Default::default() }),
            0x2 => Some(CartridgeMode { mbc: MemoryController::MBC1,
                                        has_ram: true,
                                        ..Default::default() }),
            0x3 => Some(CartridgeMode { mbc: MemoryController::MBC1,
                                        has_ram: true,
                                        has_batt: true,
                                        ..Default::default() }),
            0x5 => Some(CartridgeMode { mbc: MemoryController::MBC2,
                                        ..Default::default() }),
            0x6 => Some(CartridgeMode { mbc: MemoryController::MBC2,
                                        has_batt: true,
                                        ..Default::default() }),
            0x8 => Some(CartridgeMode { has_ram: true,
                                        ..Default::default() }),
            0x9 => Some(CartridgeMode { has_ram: true,
                                        has_batt: true,
                                        ..Default::default() }),
            _ => None
        }
    }
}

pub struct Cartridge {
    data: Vec<u8>,
    mode: CartridgeMode
}

impl Cartridge {
    pub fn new(data: Vec<u8>) -> Self {
        let mode = CartridgeMode::new(data[0x147]).expect("Unknown cartridge type");
        let size = data[0x148];
        println!("Mode: {:?} ROM size: {} ({})", mode, data.len(), size);
        Cartridge {
            data: data,
            mode: mode
        }
    }
    pub fn r8(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    pub fn w8(&mut self, addr: u16, val: u8) {
        match self.mode.mbc {
            MemoryController::None => {},
            _ => { unimplemented!("Bank switching: {:#04x} <- {}", addr, val); }
        }
    }

}
