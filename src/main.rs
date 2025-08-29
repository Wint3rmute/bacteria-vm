
use std::fs::File;
use std::io::Write;

/// Draw a single VM's memory as a grid at the given offset
fn draw_vm(vm: &compute::VM, offset_x: f32, offset_y: f32, grid_size: f32, padding: f32) {
    let cols = 16;
    let rows = 16;
    let square_width = (grid_size - (cols as f32 - 1.0) * padding) / cols as f32;
    let square_height = (grid_size - (rows as f32 - 1.0) * padding) / rows as f32;
    for row in 0..rows {
        for col in 0..cols {
            let x = offset_x + col as f32 * (square_width + padding);
            let y = offset_y + row as f32 * (square_height + padding);
            let idx = row * cols + col;
            let value = vm.memory[idx];
            let t = value as f32 / 255.0;
            // More varied color palette: red, orange, yellow, green, cyan, blue, purple, white
            let color = if t < 0.15 {
                // Red
                Color::new(1.0, t * 6.0, 0.0, 1.0)
            } else if t < 0.30 {
                // Orange
                Color::new(1.0, 0.5 + (t - 0.15) * 3.33, 0.0, 1.0)
            } else if t < 0.45 {
                // Yellow
                Color::new(1.0, 1.0, (t - 0.30) * 6.66, 1.0)
            } else if t < 0.60 {
                // Green
                Color::new(1.0 - (t - 0.45) * 6.66, 1.0, 0.0, 1.0)
            } else if t < 0.75 {
                // Cyan
                Color::new(0.0, 1.0, (t - 0.60) * 6.66, 1.0)
            } else if t < 0.90 {
                // Blue
                Color::new(0.0, 1.0 - (t - 0.75) * 6.66, 1.0, 1.0)
            } else if t < 0.98 {
                // Purple
                Color::new((t - 0.90) * 12.5, 0.0, 1.0, 1.0)
            } else {
                // White
                Color::new(1.0, 1.0, 1.0, 1.0)
            };
            draw_rectangle(x, y, square_width, square_height, color);
            if idx == vm.pc {
                draw_rectangle_lines(x, y, square_width, square_height, 5.0, WHITE);
            }
        }
    }
}

use macroquad::prelude::*;
use tracing::info;
use tracing_subscriber;
use ::rand::{Rng, thread_rng};

mod compute;

// Configure tracing subscriber for logging
fn configure_tracing() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(fmt::layer())
        .init();
}

#[macroquad::main("BasicShapes")]
async fn main() {
    configure_tracing();

    let mut longest_steps: usize = 0;
    let mut best_initial_state: Option<[u8; compute::MEM_SIZE]> = None;

    let mut rng = thread_rng();
    // Set grid dimensions (e.g., 2x6)
    let vm_rows = 4;
    let vm_cols = 6;
    let vm_count = vm_rows * vm_cols;
    let mut vms: Vec<compute::VM> = (0..vm_count).map(|_| {
        let mut vm = compute::VM::new();
        vm.randomize(&mut rng);
        vm
    }).collect();

    let mut paused = false;


    loop {
        clear_background(BLACK);

        let padding = 5.0;
        let extra_padding = 10.0; // Extra padding between VMs
        // Calculate cell size so that all VMs fit and are square
        let available_width = screen_width() - (padding + extra_padding) * (vm_cols as f32 + 1.0);
        let available_height = screen_height() - (padding + extra_padding) * (vm_rows as f32 + 1.0);
        let cell_size = available_width.min(available_height) / (vm_cols.max(vm_rows) as f32);

        // Calculate total grid size
        let total_grid_width = vm_cols as f32 * cell_size + (vm_cols as f32 + 1.0) * (padding + extra_padding);
        let total_grid_height = vm_rows as f32 * cell_size + (vm_rows as f32 + 1.0) * (padding + extra_padding);

        // Calculate offsets to center the grid
        let start_x = (screen_width() - total_grid_width) / 2.0 + padding + extra_padding;
        let start_y = (screen_height() - total_grid_height) / 2.0 + padding + extra_padding;

        // Arrange VMs in a vm_rows x vm_cols grid
        for i in 0..vm_count {
            let row = i / vm_cols;
            let col = i % vm_cols;
            let offset_x = start_x + col as f32 * (cell_size + padding + extra_padding);
            let offset_y = start_y + row as f32 * (cell_size + padding + extra_padding);
            // Draw background
            draw_rectangle(
                offset_x - padding,
                offset_y - padding,
                cell_size + 2.0 * padding,
                cell_size + 2.0 * padding,
                DARKGRAY,
            );
            draw_vm(&vms[i], offset_x, offset_y, cell_size, padding);
        }

        // Toggle pause/unpause with space
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
            info!("Simulation {}", if paused {"paused"} else {"running"});
        }

        // Run simulation at 60fps if not paused
        if !paused {
            for vm in &mut vms {
                vm.step();
            }
        }
        // Single step forward with 's' key when paused
        if paused && is_key_pressed(KeyCode::S) {
            info!("Single step");
            for vm in &mut vms {
                vm.step();
            }
        }
        

        // If any VM is halted, check if it has the longest run
        for vm in &mut vms {
            if vm.halted {
                info!("VM halted, generating new program and restarting");
                if vm.total_steps_count > longest_steps {
                    longest_steps = vm.total_steps_count;
                    best_initial_state = Some(vm.initial_state);
                    // Save to file
                    if let Ok(mut file) = File::create("best_vm_program.bin") {
                        let _ = file.write_all(&vm.initial_state);
                        info!("Saved best initial_state to best_vm_program.bin (steps: {})", longest_steps);
                    }
                }
                // Genetic evolution: use best VM, then partial_randomize
                if let Some(best) = best_initial_state {
                    vm.memory.copy_from_slice(&best);
                    vm.initial_state.copy_from_slice(&best);
                    vm.partial_randomize(&mut rng);
                } else {
                    vm.randomize(&mut rng);
                }
            }
        }
        next_frame().await;
    }
}