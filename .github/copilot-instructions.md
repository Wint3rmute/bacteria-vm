# Bacteria VM - Genetic Algorithm Virtual Machine Simulation

**ALWAYS follow these instructions first. Only search for additional information or use bash commands if the information here is incomplete or found to be in error.**

Bacteria VM is a Rust application that simulates genetic evolution using simple 8-bit virtual machines. It displays a 4x4 grid of VMs, each with 256 bytes of memory, executing random programs that evolve over time. The VMs that run longest before halting have their programs saved and used as the basis for the next generation.

## Working Effectively

### Bootstrap and Build
- Install Rust toolchain: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Source the environment: `source $HOME/.cargo/env`
- **Initial build times** (with dependency downloads):
  - `cargo check` -- takes ~20 seconds first time. NEVER CANCEL. Set timeout to 60+ seconds.
  - `cargo build` -- takes ~10 seconds debug, ~20 seconds release. NEVER CANCEL. Set timeout to 60+ seconds.  
  - `cargo build --release` -- takes ~20 seconds. NEVER CANCEL. Set timeout to 60+ seconds.
- **Subsequent builds** (cached dependencies): All commands take <1 second but still use 60+ second timeouts.

### Testing
- `cargo test` -- takes ~1 second. No tests currently exist, but command validates compilation.
- **CRITICAL**: There are no unit tests. Manual validation is required for all changes.

### Running the Application
- **GRAPHICS REQUIREMENT**: Application requires X11 display and cannot run in headless environments.
- Run with: `cargo run` or `target/debug/life` (debug) or `target/release/life` (release)
- **Expected behavior**: Opens a window showing a 4x4 grid of colored squares representing VM memory states
- **Controls**:
  - Space: Pause/unpause simulation
  - Left/Right arrows: Adjust simulation speed  
  - 'R': Reset speed to default
  - 'S': Single step when paused
  - 'F': Toggle fullscreen

### Code Quality and Linting
- `cargo fmt` -- formats code. ALWAYS run before committing.
- `cargo clippy` -- lints code. ALWAYS run to catch issues.
- **Expected warnings**: Several clippy warnings exist (deprecated rand functions, unused variables). These are non-blocking.

## Validation Scenarios

**CRITICAL**: Since this is a graphics application that cannot run in headless CI, you MUST validate changes through code inspection and compilation testing.

### After Making Changes:
1. **ALWAYS** run `cargo check` to verify compilation (20s timeout minimum)
2. **ALWAYS** run `cargo clippy` to check for new issues  
3. **ALWAYS** run `cargo fmt` to format code
4. **Test VM logic changes**: If modifying `src/compute.rs`, create a simple test program and verify the VM executes expected instruction sequences
5. **Test graphics changes**: If modifying `src/main.rs`, verify the drawing logic compiles and makes sense

### Manual Testing Scenarios:
- **VM instruction execution**: Verify new instructions follow the pattern in `compute.rs::step()`
- **Memory management**: Ensure memory access stays within bounds (MEM_SIZE = 256)
- **Genetic algorithm**: Changes to evolution logic should maintain the longest-running program selection
- **Graphics rendering**: Visual changes should maintain the 16x16 memory grid per VM

### Code Validation Snippet:
For testing VM logic changes, add this to `src/main.rs` temporarily:
```rust
// Test basic VM functionality - add to main() for validation
fn test_vm_basic() {
    let mut vm = compute::VM::new();
    
    // Test 1: Basic execution
    vm.memory[0] = compute::Instruction::INC as u8;  // INC
    vm.memory[1] = compute::Instruction::INC as u8;  // INC
    vm.memory[2] = compute::Instruction::HLT as u8;  // HLT
    
    vm.step(); // INC - acc should be 1
    vm.step(); // INC - acc should be 2  
    vm.step(); // HLT - should halt
    
    assert_eq!(vm.acc, 2, "Accumulator should be 2 after two INC operations");
    assert_eq!(vm.halted, true, "VM should be halted after HLT instruction");
    
    println!("✓ Basic VM test passed");
    
    // Test 2: Memory operations
    vm = compute::VM::new();
    vm.acc = 42;
    vm.memory[0] = compute::Instruction::STA as u8;  // STA 10
    vm.memory[1] = 10;                               // address 10
    vm.memory[2] = compute::Instruction::LDA as u8;  // LDA 10  
    vm.memory[3] = 10;                               // address 10
    vm.memory[4] = compute::Instruction::HLT as u8;  // HLT
    
    vm.step(); // STA 10 - store acc (42) to memory[10]
    vm.step(); // LDA 10 - load memory[10] to acc
    vm.step(); // HLT
    
    assert_eq!(vm.memory[10], 42, "Memory[10] should contain 42");
    assert_eq!(vm.acc, 42, "Accumulator should still be 42");
    
    println!("✓ Memory operations test passed");
}
```

## Codebase Navigation

### Key Files and Locations
- `src/main.rs` (215 lines): Graphics, UI, main game loop, genetic algorithm coordination
- `src/compute.rs` (256 lines): VM implementation, instruction set, execution engine
- `Cargo.toml`: Dependencies (macroquad, rand, tracing)

### Important Code Sections
- **VM struct** (`src/compute.rs:8-17`): Core VM state with memory, PC, accumulator
- **Instruction enum** (`src/compute.rs:19-33`): Complete 8-bit instruction set
- **VM::step()** (`src/compute.rs:103+`): Main execution engine for VM instructions
- **Main loop** (`src/main.rs:106+`): Graphics rendering and evolution logic
- **Genetic evolution** (`src/main.rs:190+`): Selection and mutation of successful programs

### Architecture Overview
- **16 VMs** run in parallel in a 4x4 visual grid
- Each **VM has 256 bytes** of memory displayed as 16x16 colored squares
- **Instruction set**: 11 basic operations (NOP, LDA, STA, ADD, SUB, JMP, JZ, INC, DEC, SWP, CMP, HLT)
- **Evolution**: Programs that run longest before halting are saved and used for genetic mutations
- **Visual feedback**: Memory values mapped to colors, PC highlighted with white border

### Common Patterns
- **Memory bounds checking**: All memory access uses `.get(addr).copied().unwrap_or(0)`
- **Instruction matching**: `match opcode { x if x == Instruction::NAME as u8 => ... }`
- **Logging**: Uses `tracing::trace!()` for instruction execution details
- **Genetic mutation**: `partial_randomize()` modifies 1-10% of program bytes

## Build Timing and Timeouts

**CRITICAL**: Always use adequate timeouts for build commands:
- `cargo check`: ~20s initial, <1s cached → Set 60+ second timeout
- `cargo build`: ~10s initial, <1s cached → Set 60+ second timeout  
- `cargo build --release`: ~20s initial, <1s cached → Set 60+ second timeout
- `cargo test`: <1 second → Set 30+ second timeout
- `cargo fmt`: <1 second → Set 30+ second timeout
- `cargo clippy`: <1 second → Set 30+ second timeout

**NEVER CANCEL** build commands. If they appear to hang, wait at least 60 seconds.

## Common Tasks Reference

### Repository Root Contents
```
.
├── .git/
├── .gitignore           # Excludes /target
├── Cargo.toml          # Project config, dependencies
├── Cargo.lock          # Locked dependency versions  
├── best_vm_program.bin # Generated: best evolved program
├── src/
│   ├── main.rs         # Graphics and main loop
│   └── compute.rs      # VM implementation
└── target/             # Build artifacts (gitignored)
    ├── debug/life      # Debug executable
    └── release/life    # Release executable
```

### Dependencies (Cargo.toml)
- `macroquad = "0.4.14"` - Graphics and windowing
- `rand = "0.9.2"` - Random number generation for mutations  
- `tracing = "*"` - Logging framework
- `tracing-subscriber = "*"` - Log output formatting

### Binary Targets
- Main executable: `life` (named in Cargo.toml, not matching repo name)
- Debug build: `target/debug/life`
- Release build: `target/release/life`

## Known Issues and Limitations
- **No headless mode**: Cannot run without X11 display
- **No unit tests**: Manual validation required for all changes
- **Deprecation warnings**: Uses older rand API (non-breaking)
- **Clippy warnings**: Several style warnings that don't affect functionality
- **Single binary**: No library crates or additional tools