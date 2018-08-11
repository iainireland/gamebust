use rustyline::Editor;
use std::collections::HashSet;

use cpu::Cpu;

pub struct Debugger {
    readline: Editor<()>,
    breakpoints: HashSet<u16>,
    paused: bool
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            readline: Editor::new(),
            breakpoints: HashSet::new(),
            paused: true
        }
    }
    pub fn debug(&mut self, cpu: &mut Cpu) {
        self.paused = false;
        loop {
            let line = match self.readline.readline("> ") {
                Ok(l) => l,
                Err(_) => continue
            };
            match line.as_str() {
                "q" | "quit" => break,
                "r" | "reg"  => princpu.reg
                l => println!("{}", l)
            }
        }
    }
    #[inline(always)]
    pub fn pause(&mut self) {
        self.paused = true;
    }
    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    pub fn check_breakpoints(&mut self, cpu: &Cpu) {
        if self.breakpoints.contains(&cpu.get_pc()) {
            self.paused = true;
        }
    }
}
