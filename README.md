# Bacteria VM

A bacteria virtual machine simulation with evolutionary algorithms and visual interface.

## Features

- Visual simulation of virtual machines running in a grid
- Genetic evolution of VM programs
- Real-time visualization using macroquad
- Interactive controls for simulation speed and pausing

## Building and Running

### Prerequisites

- Rust (latest stable version)
- System dependencies for graphics:
  - Linux: `libasound2-dev`, `libudev-dev`
  - Windows: No additional dependencies
  - macOS: No additional dependencies

### Building

```bash
cargo build --release
```

### Running

```bash
cargo run --release
```

## Controls

- **Space**: Pause/unpause simulation
- **S**: Single step when paused
- **F**: Toggle fullscreen
- **Left/Right arrows**: Adjust simulation speed
- **R**: Reset simulation speed

## GitHub Actions

This repository includes several GitHub Actions workflows:

- **CI** (`ci.yml`): Builds, tests, and lints the code on multiple platforms
- **Release** (`release.yml`): Creates release binaries for tagged versions
- **Security** (`security.yml`): Runs security audits and dependency checks
- **Rust Versions** (`rust-versions.yml`): Tests against multiple Rust versions

## License

[Add license information here]