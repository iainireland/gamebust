use {SCREEN_WIDTH,SCREEN_HEIGHT};

const TILE_LINES_SIZE: usize = 0x1800 / 2;
const BG_MAP_SIZE: usize = 0x400;
const SPRITE_RAM_SIZE: usize = 0xa0;
const SCREEN_BUFFER_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT * 3;

const HBLANK_CYCLES: i32 = 200;
const VBLANK_CYCLES: i32 = 456;
const OAM_ACCESS_CYCLES: i32 = 84;
const VRAM_ACCESS_CYCLES: i32 = 172;

#[derive(Copy,Clone,Debug)]
pub enum BgMap {
    Map1, Map2
}

struct Palette {
    colours: [u8; 4]
}

impl Palette {
    #[inline(always)]
    pub fn get(&self, index: usize) -> u8 {
        self.colours[index]
    }
    #[inline(always)]
    pub fn from_u8(raw: u8) -> Self {
        const BRIGHTNESS: [u8; 4] = [0xff, 0xcc, 0x77, 0x00];
        let mut colours = [0; 4];
        for i in 0..4 {
            let bits = (raw >> (2 * i)) & 0x3;
            colours[i] = BRIGHTNESS[bits as usize];
        }
        Palette { colours: colours }
    }
    #[inline(always)]
    pub fn to_u8(&self) -> u8 {
        self.colours[0]      | self.colours[1] << 2 |
        self.colours[2] << 4 | self.colours[3] << 6
    }
}

enum Mode {
    HBlank, VBlank, OamAccess, VramAccess,
}

impl Mode {
    #[inline(always)]
    pub fn get_bits(&self) -> u8 {
        match self {
            Mode::HBlank => 0,
            Mode::VBlank => 1,
            Mode::OamAccess => 2,
            Mode::VramAccess => 3,
        }
    }
}

pub struct Gpu {
    tile_lines: [u16; TILE_LINES_SIZE],
    bg_map: [[u8; BG_MAP_SIZE]; 2],
    sprite_ram: [u8; SPRITE_RAM_SIZE],
    bg_enabled: bool,
    sprites_enabled: bool,
    large_sprites_enabled: bool,
    active_bg_map: BgMap,
    bg_and_sprite_tiles_overlap: bool,
    window_enabled: bool,
    active_window_map: BgMap,
    lcd_enabled: bool,

    scroll_x: u8,
    scroll_y: u8,
    ly: u8,
    ly_compare: u8,
    bg_palette: Palette,
    obj0_palette: Palette,
    obj1_palette: Palette,
    window_x: u8,
    window_y: u8,

    ly_check_enabled: bool,
    oam_check_enabled: bool,
    vblank_check_enabled: bool,
    hblank_check_enabled: bool,

    screen_buffer: [u8; SCREEN_BUFFER_SIZE],
    mode: Mode,
    cycles_left: i32,
}

impl Gpu {

    pub fn new() -> Self {
        Gpu {
            tile_lines: [0; TILE_LINES_SIZE],
            bg_map: [[0; BG_MAP_SIZE]; 2],
            sprite_ram: [0; SPRITE_RAM_SIZE],
            bg_enabled: false,
            sprites_enabled: false,
            large_sprites_enabled: false,
            active_bg_map: BgMap::Map1,
            bg_and_sprite_tiles_overlap: false,
            window_enabled: false,
            active_window_map: BgMap::Map1,
            lcd_enabled: false,
            scroll_x: 0,
            scroll_y: 0,
            ly: 0,
            ly_compare: 0,
            bg_palette: Palette::from_u8(228),
            obj0_palette: Palette::from_u8(228),
            obj1_palette: Palette::from_u8(228),
            window_x: 0,
            window_y: 0,
            ly_check_enabled: false,
            oam_check_enabled: false,
            vblank_check_enabled: false,
            hblank_check_enabled: false,
            screen_buffer: [0; SCREEN_BUFFER_SIZE],
            mode: Mode::OamAccess,
            cycles_left: 0,
        }
    }
    pub fn update(&mut self, cycles: u32) -> bool {
        const VBLANK_SCANLINES: u8 = 12;

        let mut vblank = false;
        if !self.lcd_enabled { return false; }
        self.cycles_left -= cycles as i32;

        if self.cycles_left > 0 {
            return false;
        }

        match self.mode {
            Mode::HBlank => {
                self.ly += 1;
                if self.ly < SCREEN_HEIGHT as u8 {
                    self.switch_mode(Mode::OamAccess);
                } else {
                    vblank = true;
                    self.switch_mode(Mode::VBlank);
                }

            },
            Mode::VBlank => {
                self.ly += 1;
                if self.ly == SCREEN_HEIGHT as u8 + VBLANK_SCANLINES {
                    self.ly = 0;
                    self.switch_mode(Mode::OamAccess);
                } else {
                    self.cycles_left = VBLANK_CYCLES;
                }
            },
            Mode::OamAccess => {
                self.switch_mode(Mode::VramAccess);
            },
            Mode::VramAccess => {
                self.render_scanline();
                self.switch_mode(Mode::HBlank);
            }
        }
        vblank
    }
    #[inline(always)]
    fn switch_mode(&mut self, new_mode: Mode) {
        self.mode = new_mode;
        let mode_cycles = match self.mode {
            Mode::HBlank => HBLANK_CYCLES,
            Mode::VBlank => VBLANK_CYCLES,
            Mode::OamAccess => OAM_ACCESS_CYCLES,
            Mode::VramAccess => VRAM_ACCESS_CYCLES,
        };
        self.cycles_left += mode_cycles;
    }
    fn render_scanline(&mut self) {
        if self.bg_enabled {
            self.render_bg();
        }
        if self.sprites_enabled {
            self.render_sprites();
        }
    }
    fn render_bg(&mut self) {
        let is_window = self.window_enabled && self.window_y <= self.ly;
        let map = if is_window { self.active_window_map } else { self.active_bg_map };

        let pixel_y = if is_window {
            self.ly - self.window_y
        } else {
            self.scroll_y + self.ly
        };
        let tile_y = (pixel_y / 8) as u16;
        let tile_line_y = (pixel_y % 8) as usize;

        for i in 0..SCREEN_WIDTH as u8 {
            let pixel_x = if is_window { i - self.window_x } else { i + self.scroll_x };
            let tile_x = (pixel_x / 8) as u16;
            let tile_bit_shift = (7 - pixel_x % 8) as u16;
            let tile_val = self.read_bg_map(tile_y * 32 + tile_x, map);

            let tile_index = if self.bg_and_sprite_tiles_overlap {
                tile_val as usize
            } else {
                (256 + (tile_val as i16)) as usize
            };

            let tile_line = self.tile_lines[tile_index * 8 + tile_line_y];
            let palette_index =
                ((tile_line >> tile_bit_shift) & 1) * 2 +
                ((tile_line >> (tile_bit_shift + 8)) & 1);
            let colour = self.bg_palette.get(palette_index as usize);
            self.draw_pixel(i as usize, colour);
        }
    }
    fn render_sprites(&mut self) {
        unimplemented!("Sprites");
    }
    #[inline(always)]
    fn draw_pixel(&mut self, x: usize, colour: u8) {
            let slice_start = self.ly as usize * SCREEN_WIDTH * 3;
            let slice_end = (self.ly + 1) as usize * SCREEN_WIDTH * 3;
            let screen_buffer_slice = &mut self.screen_buffer[slice_start..slice_end];
            screen_buffer_slice[x as usize * 3] = colour;
            screen_buffer_slice[x as usize * 3 + 1] = colour;
            screen_buffer_slice[x as usize * 3 + 2] = colour;
    }

    #[inline(always)]
    pub fn read_tile_ram(&self, addr: u16) -> u8 {
        let line = self.tile_lines[addr as usize / 2];
        if addr & 0x1 == 0 {
            (line & 0x0f) as u8
        } else {
            (line >> 8) as u8
        }
    }
    #[inline(always)]
    pub fn write_tile_ram(&mut self, addr: u16, val: u8) {
        let index = addr as usize / 2;
        let line = self.tile_lines[index];
        self.tile_lines[index] = if addr & 0x1 == 0 {
            (line & 0xf0) | val as u16
        } else {
            (line & 0x0f) | ((val as u16) << 8)
        };
    }
    #[inline(always)]
    pub fn read_bg_map(&self, addr: u16, bg_map: BgMap) -> u8 {
        let index = match bg_map { BgMap::Map1 => 0, BgMap::Map2 => 1 };
        self.bg_map[index][addr as usize]
    }
    #[inline(always)]
    pub fn write_bg_map(&mut self, addr: u16, bg_map: BgMap, val: u8) {
        let index = match bg_map { BgMap::Map1 => 0, BgMap::Map2 => 1 };
        self.bg_map[index][addr as usize] = val;
    }
    #[inline(always)]
    pub fn read_sprite_ram(&self, addr: u16) -> u8 {
        self.sprite_ram[addr as usize]
    }
    #[inline(always)]
    pub fn write_sprite_ram(&mut self, addr: u16, val: u8) {
        self.sprite_ram[addr as usize] = val;
    }
    #[inline(always)]
    pub fn get_control(&self) -> u8 {
        let mut result = 0;
        if self.bg_enabled                           { result |= 1 << 0; }
        if self.sprites_enabled                      { result |= 1 << 1; }
        if self.large_sprites_enabled                { result |= 1 << 2; }
        if let BgMap::Map2 = self.active_bg_map     { result |= 1 << 3; }
        if self.bg_and_sprite_tiles_overlap          { result |= 1 << 4; }
        if self.window_enabled                       { result |= 1 << 5; }
        if let BgMap::Map2 = self.active_window_map { result |= 1 << 6; }
        if self.lcd_enabled                          { result |= 1 << 7; }
        result
    }
    #[inline(always)]
    pub fn set_control(&mut self, value: u8) {
        self.bg_enabled = (value & (1 << 0)) != 0;
        self.sprites_enabled = (value & (1 << 1)) != 0;
        self.large_sprites_enabled = (value & (1 << 2)) != 0;
        self.active_bg_map = if (value & (1 << 3)) == 0 { BgMap::Map1 } else { BgMap::Map2 };
        self.bg_and_sprite_tiles_overlap = (value & (1 << 4)) != 0;
        self.window_enabled = (value & (1 << 5)) != 0;
        self.active_window_map = if (value & (1 << 6)) == 0 { BgMap::Map1 } else { BgMap::Map2 };
        self.lcd_enabled = (value & (1 << 7)) != 0;
    }
    #[inline(always)]
    pub fn get_stat(&self) -> u8 {
        let mut result = self.mode.get_bits();
        if self.ly == self.ly_compare { result |= 1 << 2 };
        if self.hblank_check_enabled  { result |= 1 << 3 };
        if self.vblank_check_enabled  { result |= 1 << 4 };
        if self.oam_check_enabled  { result |= 1 << 5 };
        if self.ly_check_enabled      { result |= 1 << 6 };

        result
    }
    #[inline(always)]
    pub fn set_stat(&mut self, value: u8) {
        self.hblank_check_enabled = (value & (1 << 3)) != 0;
        self.vblank_check_enabled = (value & (1 << 4)) != 0;
        self.oam_check_enabled = (value & (1 << 5)) != 0;
        self.ly_check_enabled     = (value & (1 << 6)) != 0;
    }
    #[inline(always)]
    pub fn get_scroll_x(&self) -> u8 {
        self.scroll_x
    }
    #[inline(always)]
    pub fn set_scroll_x(&mut self, value: u8) {
        self.scroll_x = value;
    }
    #[inline(always)]
    pub fn get_scroll_y(&self) -> u8 {
        self.scroll_y
    }
    #[inline(always)]
    pub fn set_scroll_y(&mut self, value: u8) {
        self.scroll_y = value;
    }
    #[inline(always)]
    pub fn get_ly(&self) -> u8 {
        if self.lcd_enabled { self.ly } else { 0 }
    }
    #[inline(always)]
    pub fn reset_ly(&mut self) {
        self.ly = 0;
    }
    #[inline(always)]
    pub fn get_ly_compare(&self) -> u8 {
        self.ly_compare
    }
    #[inline(always)]
    pub fn set_ly_compare(&mut self, value: u8) {
        self.ly_compare = value;
    }
    #[inline(always)]
    pub fn get_bg_palette(&self) -> u8 {
        self.bg_palette.to_u8()
    }
    #[inline(always)]
    pub fn set_bg_palette(&mut self, value: u8) {
        self.bg_palette = Palette::from_u8(value);
    }
    #[inline(always)]
    pub fn get_obj0_palette(&self) -> u8 {
        self.obj0_palette.to_u8()
    }
    #[inline(always)]
    pub fn set_obj0_palette(&mut self, value: u8) {
        self.obj0_palette = Palette::from_u8(value);
    }
    #[inline(always)]
    pub fn get_obj1_palette(&self) -> u8 {
        self.obj1_palette.to_u8()
    }
    #[inline(always)]
    pub fn set_obj1_palette(&mut self, value: u8) {
        self.obj1_palette = Palette::from_u8(value);
    }
    #[inline(always)]
    pub fn get_window_x(&self) -> u8 {
        self.window_x
    }
    #[inline(always)]
    pub fn set_window_x(&mut self, value: u8) {
        self.window_x = value;
    }
    #[inline(always)]
    pub fn get_window_y(&self) -> u8 {
        self.window_y
    }
    #[inline(always)]
    pub fn set_window_y(&mut self, value: u8) {
        self.window_y = value;
    }
    #[inline(always)]
    pub fn get_screen_buffer(&self) -> &[u8] {
        &self.screen_buffer
    }
}