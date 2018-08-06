use std::io::Read;
use std::path::Path;

use cartridge::Cartridge;
use gpu::{BgMap,Gpu};
use joypad::{Joypad,Button};
use serial::Serial;
use timer::Timer;

const BOOT_ROM_SIZE: usize = 0x100;
const INTERNAL_RAM_SIZE: usize = 0x2000;
const ZERO_PAGE_SIZE: usize = 0x7f;



pub struct Bus {
    bootrom: [u8; BOOT_ROM_SIZE],
    bootrom_active: bool,
    cartridge: Cartridge,
    gpu: Gpu,
    joypad: Joypad,
    serial: Serial,
    timer: Timer,
    internal_ram: [u8; INTERNAL_RAM_SIZE],
    zero_page: [u8; ZERO_PAGE_SIZE],
    interrupts_flag: u8,
    interrupts_enable: u8
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
            gpu: Gpu::new(),
            joypad: Joypad::new(),
            serial: Serial::new(),
            timer: Timer::new(),
            internal_ram: [0; INTERNAL_RAM_SIZE],
            zero_page: [0; ZERO_PAGE_SIZE],
            interrupts_flag: 0,
            interrupts_enable: 0,
        })
    }
    pub fn r8(&self, addr: u16) -> u8 {
        match addr {
            0x0000 ... 0x00ff if self.bootrom_active => self.bootrom[addr as usize],
            0x0000 ... 0x7fff => self.cartridge.r8(addr),
            0x8000 ... 0x97ff => self.gpu.read_tile_ram(addr - 0x8000),
            0x9800 ... 0x9bff => self.gpu.read_bg_map(addr - 0x9800, BgMap::Map1),
            0x9c00 ... 0x9fff => self.gpu.read_bg_map(addr - 0x9c00, BgMap::Map2),
            0xc000 ... 0xdfff => self.internal_ram[addr as usize - 0xc000],
            0xe000 ... 0xfdff => self.internal_ram[addr as usize - 0xe000],
            0xfe00 ... 0xfe9f => self.gpu.read_sprite_ram(addr - 0xfe00),
            0xfea0 ... 0xfeff => 0,
            0xff00            => self.joypad.read(),
            0xff01            => self.serial.get_transfer(),
            0xff02            => self.serial.get_control(),
            0xff04            => self.timer.get_divider(),
            0xff05            => self.timer.get_counter(),
            0xff06            => self.timer.get_modulo(),
            0xff07            => self.timer.get_control(),
            0xff0f            => self.interrupts_flag,
            0xff10 ... 0xff14 |
            0xff16 ... 0xff1e |
            0xff20 ... 0xff26 |
            0xff30 ... 0xff3f => { println!("Read IO reg (sound): {:04x}", addr); 0},
            0xff40            => self.gpu.get_control(),
            0xff41            => self.gpu.get_stat(),
            0xff42            => self.gpu.get_scroll_y(),
            0xff43            => self.gpu.get_scroll_x(),
            0xff44            => self.gpu.get_ly(),
            0xff45            => self.gpu.get_ly_compare(),
            0xff46            => unimplemented!("DMA"),
            0xff47            => self.gpu.get_bg_palette(),
            0xff48            => self.gpu.get_obj0_palette(),
            0xff49            => self.gpu.get_obj1_palette(),
            0xff4a            => self.gpu.get_window_y(),
            0xff4b            => self.gpu.get_window_x(),
            0xff00 ... 0xff7f => { println!("Read IO reg: {:04x}", addr); 0},
            0xff80 ... 0xfffe => self.zero_page[addr as usize - 0xff80],
            0xffff            => self.interrupts_enable,
            _ => unimplemented!("Unknown address: {:04x}", addr)
        }
    }
    pub fn w8(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000 ... 0x00FF if self.bootrom_active => panic!("Writing to boot rom"),
            0x0000 ... 0x7fff => self.cartridge.w8(addr, val),
            0x8000 ... 0x97ff => self.gpu.write_tile_ram(addr - 0x8000, val),
            0x9800 ... 0x9bff => self.gpu.write_bg_map(addr - 0x9800, BgMap::Map1, val),
            0x9c00 ... 0x9fff => self.gpu.write_bg_map(addr - 0x9c00, BgMap::Map2, val),
            0xc000 ... 0xdfff => self.internal_ram[addr as usize - 0xc000] = val,
            0xe000 ... 0xfdff => self.internal_ram[addr as usize - 0xe000] = val,
            0xfe00 ... 0xfe9f => self.gpu.write_sprite_ram(addr - 0xfe00, val),
            0xfea0 ... 0xfeff => {},
            0xff00            => self.joypad.write(val),
            0xff01            => self.serial.set_transfer(val),
            0xff02            => self.serial.set_control(val),
            0xff04            => self.timer.reset_divider(),
            0xff05            => self.timer.set_counter(val),
            0xff06            => self.timer.set_modulo(val),
            0xff07            => self.timer.set_control(val),
            0xff10 ... 0xff14 |
            0xff16 ... 0xff1e |
            0xff20 ... 0xff26 |
            0xff30 ... 0xff3f => println!("Write IO reg (sound): {:04x} = {}", addr, val),
            0xff40            => self.gpu.set_control(val),
            0xff41            => self.gpu.set_stat(val),
            0xff42            => self.gpu.set_scroll_y(val),
            0xff43            => self.gpu.set_scroll_x(val),
            0xff44            => self.gpu.reset_ly(),
            0xff45            => self.gpu.set_ly_compare(val),
            0xff46            => unimplemented!("DMA"),
            0xff47            => self.gpu.set_bg_palette(val),
            0xff48            => self.gpu.set_obj0_palette(val),
            0xff49            => self.gpu.set_obj1_palette(val),
            0xff4a            => self.gpu.set_window_y(val),
            0xff4b            => self.gpu.set_window_x(val),
            0xff50            => self.bootrom_active = false,
            0xff00 ... 0xff7f => println!("Write IO reg: {:04x} = {}", addr, val),
            0xff80 ... 0xfffe => self.zero_page[addr as usize - 0xff80] = val,
            0xffff            => self.interrupts_enable = val,
            _ => unimplemented!("Unknown address: {:04x}", addr)
        }
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
    pub fn key_down(&mut self, button: Button) {
        self.joypad.key_down(button);
    }
    pub fn key_up(&mut self, button: Button) {
        self.joypad.key_up(button);
    }
    pub fn update(&mut self, cycles: u32) -> bool {
        self.timer.update(cycles);
        let redraw = self.gpu.update(cycles);
        redraw
    }
    pub fn get_screen_buffer(&self) -> &[u8] {
        self.gpu.get_screen_buffer()
    }
}
