
// compute.rs

// Simple 8-bit virtual machine

pub const MEM_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct VM {
    pub memory: [u8; MEM_SIZE],
    pub initial_state: [u8; MEM_SIZE],
    pub pc: usize, // program counter
    pub acc: u8,   // accumulator
    pub halted: bool,
    pub total_steps_count: usize, // steps before halting
    pub recent_instructions: Vec<String>, // log of recent instructions
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    NOP = 0x00,      // No operation
    LDA = 0x01,      // Load accumulator from memory
    STA = 0x02,      // Store accumulator to memory
    ADD = 0x03,      // Add memory to accumulator
    SUB = 0x04,      // Subtract memory from accumulator
    JMP = 0x05,      // Jump to address
    JZ  = 0x06,      // Jump if accumulator is zero
    INC = 0x07,      // Increment accumulator
    DEC = 0x08,      // Decrement accumulator
    SWP = 0x09,      // Swap accumulator with memory
    CMP = 0x0A,      // Compare accumulator with memory
    HLT = 0xFF,      // Halt
}

impl VM {
    /// Randomize a random percent of the program
    pub fn partial_randomize<R: rand::Rng>(&mut self, rng: &mut R) {
        // Choose a random percent between 1 and 50
        let percent: u8 = rng.gen_range(1..=10);
        let count = MEM_SIZE * percent as usize / 100;
        for _ in 0..count {
            let idx = rng.gen_range(0..MEM_SIZE);
            let val = rng.random();
            self.memory[idx] = val;
            self.initial_state[idx] = val;
        }
    self.pc = 0;
        self.acc = 0;
        self.halted = false;
        self.total_steps_count = 0;
    }
    /// Save VM program (memory) to a file
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::{Write, BufWriter};
        let mut file = BufWriter::new(File::create(path)?);
        file.write_all(&self.memory)?;
        Ok(())
    }

    /// Load VM program (memory) from a file
    pub fn load_from_file(&mut self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::{Read, BufReader};
        let mut file = BufReader::new(File::open(path)?);
        file.read_exact(&mut self.memory)?;
        Ok(())
    }
    pub fn new() -> Self {
        VM {
            memory: [0; MEM_SIZE],
            initial_state: [0; MEM_SIZE],
            pc: 0,
            acc: 0,
            halted: false,
            total_steps_count: 0,
            recent_instructions: Vec::with_capacity(16),
        }
    }

    pub fn load_program(&mut self, program: &[u8]) {
        let len = program.len().min(MEM_SIZE);
        self.memory[..len].copy_from_slice(&program[..len]);
        self.initial_state[..len].copy_from_slice(&program[..len]);
        self.pc = 0;
        self.halted = false;
        self.acc = 0;
        self.total_steps_count = 0;
    }

    pub fn randomize<R: rand::Rng>(&mut self, rng: &mut R) {
        for i in 0..MEM_SIZE {
            let val = rng.random();
            self.memory[i] = val;
            self.initial_state[i] = val;
        }
    self.pc = 0;
        self.acc = 0;
        self.halted = false;
        self.total_steps_count = 0;
        self.recent_instructions.clear();
    }

    pub fn step(&mut self) {
        if self.halted || self.pc >= MEM_SIZE {
            self.halted = true;
            tracing::trace!("VM halted: pc={}, acc={}, halted={}", self.pc, self.acc, self.halted);
            return;
        }
        self.total_steps_count += 1;
    let opcode = self.memory[self.pc];
    let instr_name = instruction_to_string(opcode);
    let mut instr_log = format!("{:04}: {} (0x{:02X})", self.pc, instr_name, opcode);
/// Convert an instruction opcode to its string name
pub fn instruction_to_string(opcode: u8) -> &'static str {
    match opcode {
        x if x == Instruction::NOP as u8 => "NOP",
        x if x == Instruction::LDA as u8 => "LDA",
        x if x == Instruction::STA as u8 => "STA",
        x if x == Instruction::ADD as u8 => "ADD",
        x if x == Instruction::SUB as u8 => "SUB",
        x if x == Instruction::JMP as u8 => "JMP",
        x if x == Instruction::JZ as u8 => "JZ",
        x if x == Instruction::INC as u8 => "INC",
        x if x == Instruction::DEC as u8 => "DEC",
        x if x == Instruction::SWP as u8 => "SWP",
        x if x == Instruction::CMP as u8 => "CMP",
        x if x == Instruction::HLT as u8 => "HLT",
        _ => "???",
    }
}
        match opcode {
            x if x == Instruction::CMP as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                let val = self.memory.get(addr).copied().unwrap_or(0);
                instr_log = format!("{:04}: {} (0x{:02X}) acc={} addr={} val={}", self.pc, instr_name, opcode, self.acc, addr, val);
                tracing::trace!("CMP acc={} with addr={}, value={}", self.acc, addr, val);
                self.pc += 2;
            }
            x if x == Instruction::NOP as u8 => {
                instr_log = format!("{:04}: {} (0x{:02X})", self.pc, instr_name, opcode);
                tracing::trace!("NOP");
                self.pc += 1;
            }
            x if x == Instruction::LDA as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                instr_log = format!("{:04}: {} (0x{:02X}) addr={} -> acc={}", self.pc, instr_name, opcode, addr, self.memory.get(addr).copied().unwrap_or(0));
                tracing::trace!("LDA from addr={}", addr);
                self.acc = self.memory.get(addr).copied().unwrap_or(0);
                self.pc += 2;
            }
            x if x == Instruction::STA as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                instr_log = format!("{:04}: {} (0x{:02X}) acc={} -> addr={}", self.pc, instr_name, opcode, self.acc, addr);
                tracing::trace!("STA to addr={}", addr);
                if addr < MEM_SIZE {
                    self.memory[addr] = self.acc;
                }
                self.pc += 2;
            }
            x if x == Instruction::ADD as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                let val = self.memory.get(addr).copied().unwrap_or(0);
                instr_log = format!("{:04}: {} (0x{:02X}) acc={} + val={} (addr={})", self.pc, instr_name, opcode, self.acc, val, addr);
                tracing::trace!("ADD from addr={}, value={}", addr, val);
                self.acc = self.acc.wrapping_add(val);
                self.pc += 2;
            }
            x if x == Instruction::SUB as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                let val = self.memory.get(addr).copied().unwrap_or(0);
                instr_log = format!("{:04}: {} (0x{:02X}) acc={} - val={} (addr={})", self.pc, instr_name, opcode, self.acc, val, addr);
                tracing::trace!("SUB from addr={}, value={}", addr, val);
                self.acc = self.acc.wrapping_sub(val);
                self.pc += 2;
            }
            x if x == Instruction::JMP as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                instr_log = format!("{:04}: {} (0x{:02X}) to addr={}", self.pc, instr_name, opcode, addr);
                tracing::trace!("JMP to addr={}", addr);
                self.pc = addr;
            }
            x if x == Instruction::JZ as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                instr_log = format!("{:04}: {} (0x{:02X}) to addr={} if acc==0 (acc={})", self.pc, instr_name, opcode, addr, self.acc);
                tracing::trace!("JZ to addr={} if acc==0", addr);
                if self.acc == 0 {
                    self.pc = addr;
                } else {
                    self.pc += 2;
                }
            }
            x if x == Instruction::INC as u8 => {
                instr_log = format!("{:04}: {} (0x{:02X}) acc={} -> {}", self.pc, instr_name, opcode, self.acc, self.acc.wrapping_add(1));
                tracing::trace!("INC");
                self.acc = self.acc.wrapping_add(1);
                self.pc += 1;
            }
            x if x == Instruction::DEC as u8 => {
                instr_log = format!("{:04}: {} (0x{:02X}) acc={} -> {}", self.pc, instr_name, opcode, self.acc, self.acc.wrapping_sub(1));
                tracing::trace!("DEC");
                self.acc = self.acc.wrapping_sub(1);
                self.pc += 1;
            }
            x if x == Instruction::SWP as u8 => {
                let addr = self.memory.get(self.pc + 1).copied().unwrap_or(0) as usize;
                instr_log = format!("{:04}: {} (0x{:02X}) acc={} <-> addr={} val={}", self.pc, instr_name, opcode, self.acc, addr, self.memory.get(addr).copied().unwrap_or(0));
                tracing::trace!("SWP with addr={}", addr);
                if addr < MEM_SIZE {
                    let tmp = self.memory[addr];
                    self.memory[addr] = self.acc;
                    self.acc = tmp;
                }
                self.pc += 2;
            }
            x if x == Instruction::HLT as u8 => {
                instr_log = format!("{:04}: {} (0x{:02X})", self.pc, instr_name, opcode);
                tracing::debug!("HLT - VM halted!");
                self.halted = true;
            }
            _ => {
                instr_log = format!("{:04}: {} (0x{:02X})", self.pc, instr_name, opcode);
                tracing::trace!("Unknown instruction: 0x{:02X}", opcode);
                self.halted = true;
            }
        }
        // Store recent instructions (keep last 16)
        self.recent_instructions.push(instr_log);
        if self.recent_instructions.len() > 16 {
            self.recent_instructions.remove(0);
        }
        // If only 2 unique instructions in recent_instructions, halt and reset steps
        if self.recent_instructions.len() == 16 {
            let mut unique_instr = std::collections::HashSet::new();
            for s in &self.recent_instructions {
                // Extract instruction name (assumes format: "xxxx: NAME (0xYY)...")
                if let Some(colon) = s.find(':') {
                    if let Some(space) = s[colon+2..].find(' ') {
                        let name = &s[colon+2..colon+2+space];
                        unique_instr.insert(name);
                    }
                }
            }
            if unique_instr.len() <= 2 {
                self.halted = true;
                self.total_steps_count = 0;
            }
        }
    }

    pub fn run(&mut self) {
        while !self.halted {
            self.step();
        }
    }
}
