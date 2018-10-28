use rustyline::Editor;
use std::collections::HashSet;
use std::str::FromStr;

use cpu::Cpu;

type CommandFn = fn(&Cpu, &mut Debugger, &Vec<&str>);

#[derive(Clone,Copy)]
struct Command {
    name: &'static str,
    func: CommandFn
}

pub struct DebugState {
    pub steps_remaining: u32,
    pub breakpoints: HashSet<u16>,
    pub watchpoints: HashSet<u16>,
    pub paused: bool,
}

impl DebugState {
    pub fn new() -> Self {
        DebugState {
            steps_remaining: 0,
            breakpoints: HashSet::new(),
            watchpoints: HashSet::new(),
            paused: false,
        }
    }
}

pub struct Debugger {
    commands: Vec<Command>,
    readline: Editor<()>,
    execute: bool,
    state: DebugState,
}

impl Debugger {
    pub fn new() -> Self {
        let mut result = Debugger {
            commands: Vec::new(),
            readline: Editor::new(),
            execute: false,
            state: DebugState::new()
        };
        result.register_command("continue", cmd_continue);
        result.register_command("registers", cmd_registers);
        result.register_command("sprites", cmd_sprites);
        result.register_command("breakpoint", cmd_breakpoint);
        result.register_command("watchpoint", cmd_watchpoint);
        result.register_command("delete", cmd_delete);
        result.register_command("xamine", cmd_examine);
        result.register_command("list", cmd_list);
        result.register_command("step", cmd_step);
        result
    }
    pub fn debug(&mut self, cpu: &Cpu) {
        print_instr(cpu, cpu.reg.pc);

        self.state.paused = false;
        self.execute = false;
        while !self.execute {
            let line = match self.readline.readline("> ") {
                Ok(l) => l,
                Err(_) => continue
            };
            self.readline.add_history_entry(&line);
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
        self.state.paused = true;
    }
    #[inline(always)]
    pub fn is_paused(&self) -> bool {
        self.state.paused
    }
    pub fn get_state(&mut self) -> &mut DebugState {
        &mut self.state
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
fn print_instr(cpu: &Cpu, mut addr: u16) -> u16 {
    print!("{:04x}: ", addr);
    println!("{}", cpu.fetch(&mut addr));
    addr
}

fn cmd_continue(_cpu: &Cpu, dbg: &mut Debugger, _args: &Vec<&str>) {
    dbg.execute = true;
}
fn cmd_registers(cpu: &Cpu, _dbg: &mut Debugger, _args: &Vec<&str>) {
    println!(" A F   B C   D E   H L    PC SP\n{}", cpu.reg);
}
fn cmd_breakpoint(_cpu: &Cpu, dbg: &mut Debugger, args: &Vec<&str>) {
    if args.len() != 1 {
        println!("Usage: breakpoint <addr>");
        return;
    }
    if let Ok(addr) = u16::from_str_radix(args[0], 16) {
        dbg.state.breakpoints.insert(addr);
    }
}
fn cmd_watchpoint(_cpu: &Cpu, dbg: &mut Debugger, args: &Vec<&str>) {
    if args.len() != 1 {
        println!("Usage: watchpoint <addr>");
        return;
    }
    if let Ok(addr) = u16::from_str_radix(args[0], 16) {
        dbg.state.watchpoints.insert(addr);
    }
}
fn cmd_delete(_cpu: &Cpu, dbg: &mut Debugger, _args: &Vec<&str>) {
    dbg.state.breakpoints.clear();
}
fn cmd_list(cpu: &Cpu, _dbg: &mut Debugger, args: &Vec<&str>) {
    let mut addr = match args.len() {
        0 => cpu.reg.pc,
        1 => if let Ok(addr) = u16::from_str_radix(args[0], 16) {
            addr
        } else {
            println!("Usage: list <addr>"); return;
        },
        _ => { println!("Too many arguments to list"); return; },
    };
    for _ in 0..10 {
        addr = print_instr(cpu, addr);
    }
}
fn cmd_step(_cpu: &Cpu, dbg: &mut Debugger, args: &Vec<&str>) {
    let steps = match args.len() {
        0 => 1,
        1 => if let Ok(addr) = u32::from_str(args[0]) {
            addr
        } else {
            println!("Usage: step [<n>]"); return;
        },
        _ => { println!("Too many arguments to step"); return; },
    };
    dbg.state.steps_remaining = steps;
    dbg.execute = true;
}
fn cmd_examine(cpu: &Cpu, _dbg: &mut Debugger, args: &Vec<&str>) {
    if args.len() != 1 {
        println!("Usage: x <addr>");
        return;
    }
    if let Ok(addr) = u16::from_str_radix(args[0], 16) {
        println!("0x{:4x}: {:2x}", addr, cpu.bus.r8(addr));
    }
}
fn cmd_sprites(cpu: &Cpu, _dbg: &mut Debugger, _args: &Vec<&str>) {
    const SPRITE_RAM_ADDR: u16 = 0xfe00;
    for i in 0..::gpu::NUM_SPRITES as u16 {
        let y = cpu.bus.r8(SPRITE_RAM_ADDR + i * 4);
        let x = cpu.bus.r8(SPRITE_RAM_ADDR + i * 4 + 1);
        let t = cpu.bus.r8(SPRITE_RAM_ADDR + i * 4 + 2);
        let f = cpu.bus.r8(SPRITE_RAM_ADDR + i * 4 + 3);
        println!("Sprite {}: ({},{}) Tile {} Flags: {:8b}",
                 i, x, y, t, f);
    }
}
