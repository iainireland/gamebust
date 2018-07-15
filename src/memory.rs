const MEM_SIZE: usize = 1 << 16;

pub struct Memory {
    data: [u8; MEM_SIZE],
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            data: [0; MEM_SIZE]
        }
    }
    pub fn r8(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    pub fn r16(&self, addr: u16) -> u16 {
        let lo = self.r8(addr) as u16;
        let hi = self.r8(addr+1) as u16;
        hi << 8 | lo
    }
    pub fn w8(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }
    pub fn w16(&mut self, addr: u16, val: u16) {
        let lo = val & 0x00ff;
        let hi = (val & 0xff00) >> 8;
        self.w8(addr, lo as u8);
        self.w8(addr+1, hi as u8);
    }
    pub fn write(&mut self, addr: usize, data: &[u8]) {
        assert!(addr + data.len() < MEM_SIZE);
        self.data[addr..addr + data.len()].copy_from_slice(data);
    }
}
