use rustyline::Editor;
use std::collections::HashSet;

use cpu::Cpu;

type CommandFn = fn(&mut Cpu, &mut Debugger, &Vec<&str>);

#[derive(Clone,Copy)]
struct Command {
    name: &'static str,
    func: CommandFn
}

pub struct Debugger {
    commands: Vec<Command>,
    readline: Editor<()>,
    breakpoints: HashSet<u16>,
    paused: bool,
    execute: bool
}

impl Debugger {
    pub fn new() -> Self {
        let mut result = Debugger {
            commands: Vec::new(),
            readline: Editor::new(),
            breakpoints: HashSet::new(),
            paused: false,
            execute: false
        };
        result.register_command("continue", cmd_continue);
        result.register_command("registers", cmd_registers);
        result.register_command("breakpoint", cmd_breakpoint);
        result
    }
    pub fn debug(&mut self, cpu: &mut Cpu) {
        self.paused = false;
        self.execute = false;
        println!("Stopped at {:#04x}", cpu.reg.pc);
        while !self.execute {
            let line = match self.readline.readline("> ") {
                Ok(l) => l,
                Err(_) => continue
            };
            let mut words = line.split_whitespace();
            if let Some(command) = words.next() {
                let args: Vec<&str> = words.collect();
                match self.lookup_command(command) {
                    Ok(cmd) => (cmd.func)(cpu, self, &args),
                    Err(_) => {}
                }
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
        if self.breakpoints.contains(&cpu.reg.pc) {
            self.paused = true;
        }
    }
    fn register_command(&mut self, name: &'static str, func: CommandFn) {
        self.commands.push(Command { name: name, func: func });
    }
    fn lookup_command(&self, command: &str) -> Result<Command,()> {
        let mut candidates: Vec<Command> = Vec::new();
        for c in self.commands.iter() {
            if c.name.starts_with(command) {
                candidates.push(*c);
            }
        }
        match candidates.len() {
            0 => println!("Unknown command: {}", command),
            1 => return Ok(candidates[0]),
            _ => {
                print!("Did you mean:");
                for c in candidates { print!(" {}", c.name); }
                println!("");
            }
        }
        Err(())
    }
}

fn cmd_continue(_cpu: &mut Cpu, dbg: &mut Debugger, _args: &Vec<&str>) {
    dbg.execute = true;
}
fn cmd_registers(cpu: &mut Cpu, _dbg: &mut Debugger, _args: &Vec<&str>) {
    println!(" A F   B C   D E   H L\n{}", cpu.reg);
}
fn cmd_breakpoint(_cpu: &mut Cpu, dbg: &mut Debugger, args: &Vec<&str>) {
    if args.len() != 1 {
        println!("Usage: breakpoint <addr>");
        return;
    }
    if let Ok(addr) = u16::from_str_radix(args[0], 16) {
        dbg.breakpoints.insert(addr);
    }
}
