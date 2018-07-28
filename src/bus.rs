use std::io::Read;
use std::path::Path;

const BOOT_ROM_SIZE: usize = 0x100;
const ZERO_PAGE_SIZE: usize = 0x7f;

struct Cartridge {
    data: Vec<u8>,
}

impl Cartridge {
    pub fn new(data: Vec<u8>) -> Self {
        Cartridge {
            data: data
        }
    }
    pub fn r8(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    pub fn w8(&mut self, addr: u16, val: u8) {

    }
}

struct GPU {

}

impl GPU {
    pub fn new() -> Self {
        GPU {}
    }
    pub fn r8(&self, addr: u16) -> u8 {
        0
    }
    pub fn w8(&mut self, addr: u16, val: u8) {

    }
}

pub struct Bus {
    bootrom: [u8; BOOT_ROM_SIZE],
    bootrom_active: bool,
    cartridge: Cartridge,
    gpu: GPU,
    zero_page: [u8; ZERO_PAGE_SIZE],
}

impl Bus {
    pub fn new(cartridge_file: &Path) -> ::std::io::Result<Self> {
        let mut file = ::std::fs::File::open(cartridge_file)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(Bus {
            bootrom_active: true,
            bootrom: *include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/boot.rom")),
            cartridge: Cartridge::new(buffer),
            gpu: GPU::new(),
            zero_page: [0; ZERO_PAGE_SIZE],
        })
    }
    pub fn r8(&self, addr: u16) -> u8 {
        match addr {
            0x0000 ... 0x00ff if self.bootrom_active => self.bootrom[addr as usize],
            0x0000 ... 0x7fff => self.cartridge.r8(addr),
            0x8000 ... 0x9fff => self.gpu.r8(addr),
            0xff00 ... 0xff7f => 0, //{println!("Read IO reg: {:04x}", addr); 0},
            0xff80 ... 0xfffe => self.zero_page[addr as usize - 0xff80],
            _ => unimplemented!("Unknown address: {:04x}", addr)
        }
    }
    pub fn w8(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000 ... 0x00FF if self.bootrom_active => panic!("Writing to boot rom"),
            0x0000 ... 0x7fff => self.cartridge.w8(addr, val),
            0x8000 ... 0x9fff => self.gpu.w8(addr, val),
            0xff00 ... 0xff7f => {}, //println!("Write IO reg: {:04x} = {}", addr, val),
            0xff80 ... 0xfffe => self.zero_page[addr as usize - 0xff80] = val,
            _ => unimplemented!("Unknown address: {:04x}", addr)
        }
        //self.data[addr as usize] = val;
    }

    pub fn r16(&self, addr: u16) -> u16 {
        let lo = self.r8(addr) as u16;
        let hi = self.r8(addr+1) as u16;
        hi << 8 | lo
    }
    pub fn w16(&mut self, addr: u16, val: u16) {
        let lo = val & 0x00ff;
        let hi = (val & 0xff00) >> 8;
        self.w8(addr, lo as u8);
        self.w8(addr+1, hi as u8);
    }
    // pub fn write(&mut self, addr: usize, data: &[u8]) {
    //     assert!(addr + data.len() < MEM_SIZE);
    //     self.data[addr..addr + data.len()].copy_from_slice(data);
    // }
}
