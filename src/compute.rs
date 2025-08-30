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
    pub total_steps_count: usize,         // steps before halting
    pub recent_instructions: Vec<String>, // log of recent instructions
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    NOP = 0x00, // No operation
    LDA = 0x01, // Load accumulator from memory
    STA = 0x02, // Store accumulator to memory
    ADD = 0x03, // Add memory to accumulator
    SUB = 0x04, // Subtract memory from accumulator
    JMP = 0x05, // Jump to address
    JZ = 0x06,  // Jump if accumulator is zero
    INC = 0x07, // Increment accumulator
    DEC = 0x08, // Decrement accumulator
    SWP = 0x09, // Swap accumulator with memory
    CMP = 0x0A, // Compare accumulator with memory
    HLT = 0xFF, // Halt
}

impl From<u8> for Instruction {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Instruction::NOP,
            0x01 => Instruction::LDA,
            0x02 => Instruction::STA,
            0x03 => Instruction::ADD,
            0x04 => Instruction::SUB,
            0x05 => Instruction::JMP,
            0x06 => Instruction::JZ,
            0x07 => Instruction::INC,
            0x08 => Instruction::DEC,
            0x09 => Instruction::SWP,
            0x0A => Instruction::CMP,
            0xFF => Instruction::HLT,
            _ => Instruction::HLT, // Default to halt for unknown instructions
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Instruction::NOP => "NOP",
            Instruction::LDA => "LDA",
            Instruction::STA => "STA",
            Instruction::ADD => "ADD",
            Instruction::SUB => "SUB",
            Instruction::JMP => "JMP",
            Instruction::JZ => "JZ",
            Instruction::INC => "INC",
            Instruction::DEC => "DEC",
            Instruction::SWP => "SWP",
            Instruction::CMP => "CMP",
            Instruction::HLT => "HLT",
        };
        write!(f, "{}", name)
    }
}

impl VM {
    /// Helper to safely read memory with bounds checking
    fn read_memory(&self, addr: usize) -> u8 {
        self.memory.get(addr).copied().unwrap_or(0)
    }

    /// Helper to safely write memory with bounds checking
    fn write_memory(&mut self, addr: usize, value: u8) {
        if addr < MEM_SIZE {
            self.memory[addr] = value;
        }
    }

    /// Reset VM state to initial conditions
    fn reset(&mut self) {
        self.pc = 0;
        self.acc = 0;
        self.halted = false;
        self.total_steps_count = 0;
        self.recent_instructions.clear();
    }

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
        self.reset();
    }
    /// Save VM program (memory) to a file
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::{BufWriter, Write};
        let mut file = BufWriter::new(File::create(path)?);
        file.write_all(&self.memory)?;
        Ok(())
    }

    /// Load VM program (memory) from a file
    pub fn load_from_file(&mut self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::{BufReader, Read};
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
        self.reset();
    }

    pub fn randomize<R: rand::Rng>(&mut self, rng: &mut R) {
        for i in 0..MEM_SIZE {
            let val = rng.random();
            self.memory[i] = val;
            self.initial_state[i] = val;
        }
        self.reset();
    }

    pub fn step(&mut self) {
        if self.halted || self.pc >= MEM_SIZE {
            self.halted = true;
            tracing::trace!(
                "VM halted: pc={}, acc={}, halted={}",
                self.pc,
                self.acc,
                self.halted
            );
            return;
        }

        self.total_steps_count += 1;
        let opcode = self.memory[self.pc];
        let instruction = Instruction::from(opcode);
        
        let log_entry = self.execute_instruction(instruction);
        self.log_instruction(log_entry);
        self.check_for_infinite_loop();
    }

    fn execute_instruction(&mut self, instruction: Instruction) -> String {
        match instruction {
            Instruction::NOP => self.execute_nop(),
            Instruction::LDA => self.execute_lda(),
            Instruction::STA => self.execute_sta(),
            Instruction::ADD => self.execute_add(),
            Instruction::SUB => self.execute_sub(),
            Instruction::JMP => self.execute_jmp(),
            Instruction::JZ => self.execute_jz(),
            Instruction::INC => self.execute_inc(),
            Instruction::DEC => self.execute_dec(),
            Instruction::SWP => self.execute_swp(),
            Instruction::CMP => self.execute_cmp(),
            Instruction::HLT => self.execute_hlt(),
        }
    }
    fn execute_nop(&mut self) -> String {
        tracing::trace!("NOP");
        let log = format!("{:04}: {} (0x{:02X})", self.pc, Instruction::NOP, self.memory[self.pc]);
        self.pc += 1;
        log
    }

    fn execute_lda(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let value = self.read_memory(addr);
        let log = format!("{:04}: {} (0x{:02X}) addr={} -> acc={}", 
                         self.pc, Instruction::LDA, self.memory[self.pc], addr, value);
        tracing::trace!("LDA from addr={}", addr);
        self.acc = value;
        self.pc += 2;
        log
    }

    fn execute_sta(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let log = format!("{:04}: {} (0x{:02X}) acc={} -> addr={}", 
                         self.pc, Instruction::STA, self.memory[self.pc], self.acc, addr);
        tracing::trace!("STA to addr={}", addr);
        self.write_memory(addr, self.acc);
        self.pc += 2;
        log
    }

    fn execute_add(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let val = self.read_memory(addr);
        let log = format!("{:04}: {} (0x{:02X}) acc={} + val={} (addr={})", 
                         self.pc, Instruction::ADD, self.memory[self.pc], self.acc, val, addr);
        tracing::trace!("ADD from addr={}, value={}", addr, val);
        self.acc = self.acc.wrapping_add(val);
        self.pc += 2;
        log
    }

    fn execute_sub(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let val = self.read_memory(addr);
        let log = format!("{:04}: {} (0x{:02X}) acc={} - val={} (addr={})", 
                         self.pc, Instruction::SUB, self.memory[self.pc], self.acc, val, addr);
        tracing::trace!("SUB from addr={}, value={}", addr, val);
        self.acc = self.acc.wrapping_sub(val);
        self.pc += 2;
        log
    }

    fn execute_jmp(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let log = format!("{:04}: {} (0x{:02X}) to addr={}", 
                         self.pc, Instruction::JMP, self.memory[self.pc], addr);
        tracing::trace!("JMP to addr={}", addr);
        self.pc = addr;
        log
    }

    fn execute_jz(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let log = format!("{:04}: {} (0x{:02X}) to addr={} if acc==0 (acc={})", 
                         self.pc, Instruction::JZ, self.memory[self.pc], addr, self.acc);
        tracing::trace!("JZ to addr={} if acc==0", addr);
        if self.acc == 0 {
            self.pc = addr;
        } else {
            self.pc += 2;
        }
        log
    }

    fn execute_inc(&mut self) -> String {
        let old_acc = self.acc;
        self.acc = self.acc.wrapping_add(1);
        let log = format!("{:04}: {} (0x{:02X}) acc={} -> {}", 
                         self.pc, Instruction::INC, self.memory[self.pc], old_acc, self.acc);
        tracing::trace!("INC");
        self.pc += 1;
        log
    }

    fn execute_dec(&mut self) -> String {
        let old_acc = self.acc;
        self.acc = self.acc.wrapping_sub(1);
        let log = format!("{:04}: {} (0x{:02X}) acc={} -> {}", 
                         self.pc, Instruction::DEC, self.memory[self.pc], old_acc, self.acc);
        tracing::trace!("DEC");
        self.pc += 1;
        log
    }

    fn execute_swp(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let old_mem_val = self.read_memory(addr);
        let log = format!("{:04}: {} (0x{:02X}) acc={} <-> addr={} val={}", 
                         self.pc, Instruction::SWP, self.memory[self.pc], self.acc, addr, old_mem_val);
        tracing::trace!("SWP with addr={}", addr);
        if addr < MEM_SIZE {
            let tmp = self.memory[addr];
            self.memory[addr] = self.acc;
            self.acc = tmp;
        }
        self.pc += 2;
        log
    }

    fn execute_cmp(&mut self) -> String {
        let addr = self.read_memory(self.pc + 1) as usize;
        let val = self.read_memory(addr);
        let log = format!("{:04}: {} (0x{:02X}) acc={} addr={} val={}", 
                         self.pc, Instruction::CMP, self.memory[self.pc], self.acc, addr, val);
        tracing::trace!("CMP acc={} with addr={}, value={}", self.acc, addr, val);
        self.pc += 2;
        log
    }

    fn execute_hlt(&mut self) -> String {
        let log = format!("{:04}: {} (0x{:02X})", self.pc, Instruction::HLT, self.memory[self.pc]);
        tracing::debug!("HLT - VM halted!");
        self.halted = true;
        log
    }

    fn log_instruction(&mut self, log_entry: String) {
        self.recent_instructions.push(log_entry);
        if self.recent_instructions.len() > 16 {
            self.recent_instructions.remove(0);
        }
    }

    fn check_for_infinite_loop(&mut self) {
        // If only 2 unique instructions in recent_instructions, halt and reset steps
        if self.recent_instructions.len() == 16 {
            let mut unique_instr = std::collections::HashSet::new();
            for s in &self.recent_instructions {
                // Extract instruction name (assumes format: "xxxx: NAME (0xYY)...")
                if let Some(colon) = s.find(':') {
                    if let Some(space) = s[colon + 2..].find(' ') {
                        let name = &s[colon + 2..colon + 2 + space];
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
