use std::io::Read;
use std::path::Path;

use cartridge::Cartridge;
use cpu::Interrupt;
use debugger::DebugState;
use gpu::{BgMap,Gpu};
use joypad::{Joypad,Button};
use serial::Serial;
use timer::Timer;

const BOOT_ROM_SIZE: usize = 0x100;
const INTERNAL_RAM_SIZE: usize = 0x2000;
const HIGH_RAM_SIZE: usize = 0x7f;

struct WatchInfo {
    enabled: bool,
    write_buffer: Vec<u16>
}

impl WatchInfo {
    pub fn new() -> Self {
        WatchInfo {
            enabled: false,
            write_buffer: vec![],
        }
    }
}

pub struct Bus {
    bootrom: [u8; BOOT_ROM_SIZE],
    bootrom_active: bool,
    cartridge: Cartridge,
    dma: Dma,
    gpu: Gpu,
    joypad: Joypad,
    serial: Serial,
    timer: Timer,
    internal_ram: [u8; INTERNAL_RAM_SIZE],
    high_ram: [u8; HIGH_RAM_SIZE],
    interrupts_flag: Interrupt,
    interrupts_enable: u8,
    watch_info: WatchInfo
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
            dma: Dma::new(),
            gpu: Gpu::new(),
            joypad: Joypad::new(),
            serial: Serial::new(),
            timer: Timer::new(),
            internal_ram: [0; INTERNAL_RAM_SIZE],
            high_ram: [0; HIGH_RAM_SIZE],
            interrupts_flag: Interrupt::empty(),
            interrupts_enable: 0,
            watch_info: WatchInfo::new(),
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
            0xff0f            => self.interrupts_flag.bits(),
            0xff10 ... 0xff14 |
            0xff16 ... 0xff1e |
            0xff20 ... 0xff26 |
            0xff30 ... 0xff3f => { // unimplemented!("Read IO reg (sound): {:04x}", addr);
                                   0},
            0xff40            => self.gpu.get_control(),
            0xff41            => self.gpu.get_stat(),
            0xff42            => self.gpu.get_scroll_y(),
            0xff43            => self.gpu.get_scroll_x(),
            0xff44            => self.gpu.get_ly(),
            0xff45            => self.gpu.get_ly_compare(),
            0xff46            => self.dma.get_address(),
            0xff47            => self.gpu.get_bg_palette(),
            0xff48            => self.gpu.get_obj_palette(0),
            0xff49            => self.gpu.get_obj_palette(1),
            0xff4a            => self.gpu.get_window_y(),
            0xff4b            => self.gpu.get_window_x(),
            0xff00 ... 0xff7f => 0xff,
            0xff80 ... 0xfffe => self.high_ram[addr as usize - 0xff80],
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
            0xff0f            => self.interrupts_flag = Interrupt::from_bits_truncate(val),
            0xff04            => self.timer.reset_divider(),
            0xff05            => self.timer.set_counter(val),
            0xff06            => self.timer.set_modulo(val),
            0xff07            => self.timer.set_control(val),
            0xff10 ... 0xff14 |
            0xff16 ... 0xff1e |
            0xff20 ... 0xff26 |
            0xff30 ... 0xff3f => {},//unimplemented!("Write IO reg (sound): {:04x} = {}", addr, val),
            0xff40            => self.gpu.set_control(val),
            0xff41            => self.gpu.set_stat(val),
            0xff42            => self.gpu.set_scroll_y(val),
            0xff43            => self.gpu.set_scroll_x(val),
            0xff44            => self.gpu.reset_ly(),
            0xff45            => self.gpu.set_ly_compare(val),
            0xff46            => self.dma.set_address(val),
            0xff47            => self.gpu.set_bg_palette(val),
            0xff48            => self.gpu.set_obj_palette(0, val),
            0xff49            => self.gpu.set_obj_palette(1, val),
            0xff4a            => self.gpu.set_window_y(val),
            0xff4b            => self.gpu.set_window_x(val),
            0xff50            => self.bootrom_active = false,
            0xff00 ... 0xff7f => {},
            0xff80 ... 0xfffe => self.high_ram[addr as usize - 0xff80] = val,
            0xffff            => self.interrupts_enable = val,
            _ => unimplemented!("Unknown address: {:04x}", addr)
        }
        if self.watch_info.enabled {
            self.watch_info.write_buffer.push(addr);
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
        self.timer.update(cycles, &mut self.interrupts_flag);
        self.update_dma(cycles);
        let redraw = self.gpu.update(cycles, &mut self.interrupts_flag);
        redraw
    }
    fn update_dma(&mut self, cycles: u32) {
        for _ in 0..cycles / 4 {
            if let Some(offset) = self.dma.progress {
                let base = (self.dma.address as u16) << 8;
                let data = self.r8(base);
                self.gpu.write_sprite_ram(offset, data);
                self.dma.progress = if offset < 0x9f {
                    Some(offset + 1)
                } else {
                    None
                }
            } else {
                return;
            }
        }
    }
    pub fn get_screen_buffer(&self) -> &[u8] {
        self.gpu.get_screen_buffer()
    }
    pub fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        let enabled = Interrupt::from_bits_truncate(self.interrupts_enable);
        let valid_interrupts = (self.interrupts_flag & enabled).bits();

        // Isolate rightmost bit of x: x & ((!x)+1)
        let rightmost_bit = valid_interrupts & (!valid_interrupts).wrapping_add(1);
        let priority_interrupt = Interrupt::from_bits_truncate(rightmost_bit);

        if !priority_interrupt.is_empty() {
            Some(priority_interrupt)
        } else {
            None
        }
    }
    pub fn clear_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupts_flag.remove(interrupt);
    }
    pub fn set_watching(&mut self, value: bool) {
        self.watch_info.enabled = value;
    }
    pub fn check_watch_buffer(&mut self, debug: &mut DebugState) {
        if debug.watchpoints.is_empty() {
            return;
        }
        for addr in self.watch_info.write_buffer.drain(..) {
            if debug.watchpoints.contains(&addr) {
                debug.paused = true;
            }
        }
    }
}



pub struct Dma {
    address: u8,
    progress: Option<u16>
}

impl Dma {
    pub fn new() -> Self {
        Dma {
            address: 0,
            progress: None
        }
    }
    pub fn get_address(&self) -> u8 {
        self.address
    }
    pub fn set_address(&mut self, value: u8) {
        self.address = value;
        self.progress = Some(0);
    }
}
