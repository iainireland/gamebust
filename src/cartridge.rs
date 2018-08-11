type Bank = u8;

#[derive(Debug)]
enum MemoryController {
    None,
    MBC1(Bank),
    MBC2, //MBC3, MBC5
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
        match spec {
            0x0 => Some(Default::default()),
            0x1 => Some(CartridgeMode { mbc: MemoryController::MBC1(1),
                                        ..Default::default() }),
            0x2 => Some(CartridgeMode { mbc: MemoryController::MBC1(1),
                                        has_ram: true,
                                        ..Default::default() }),
            0x3 => Some(CartridgeMode { mbc: MemoryController::MBC1(1),
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
        let rom_size = data[0x148];
        let ram_size = data[0x149];
        println!("Mode: {:?} ROM size: {} ({}) RAM size: ({})", mode, data.len(), rom_size, ram_size);
        Cartridge {
            data: data,
            mode: mode
        }
    }
    pub fn r8(&self, addr: u16) -> u8 {
        if addr < 0x4000 {
            self.data[addr as usize]
        } else {
            match self.mode.mbc {
                MemoryController::None => {
                    self.data[addr as usize]
                },
                MemoryController::MBC1(bank) => {
                    let bank_base_addr = (bank - 1) as usize * 0x4000;
                    self.data[bank_base_addr + addr as usize]
                },
                _ => unimplemented!("Memory controller: {:?}", self.mode.mbc)
            }
        }
    }
    pub fn w8(&mut self, addr: u16, val: u8) {
        match self.mode.mbc {
            MemoryController::None => {},
            MemoryController::MBC1(ref mut bank) => {
                match addr {
                    0x0000 ... 0x1fff => unimplemented!("RAM enable"),
                    0x2000 ... 0x3fff => {
                        let mut new_bank = addr & 0x1f;
                        if new_bank == 0 { new_bank = 1; }
                        *bank = new_bank as u8;
                    },
                    _ => unimplemented!("BLAH BLAH")
                }
            },
            _ => { unimplemented!("Bank switching: {:#04x} <- {}", addr, val); }
        }
    }

}
