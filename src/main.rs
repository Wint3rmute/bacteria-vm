use std::fs::File;
use std::io::Write;


use macroquad::prelude::*;
use tracing::info;
use tracing_subscriber;
use ::rand::thread_rng;

mod compute;

/// Draw a single VM's memory as a grid at the given offset
fn draw_vm(vm: &compute::VM, offset_x: f32, offset_y: f32, grid_size: f32, padding: f32) {
    // Draw the VM grid centered in its pane
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
            let color = if t < 0.15 {
                Color::new(1.0, t * 6.0, 0.0, 1.0)
            } else if t < 0.30 {
                Color::new(1.0, 0.5 + (t - 0.15) * 3.33, 0.0, 1.0)
            } else if t < 0.45 {
                Color::new(1.0, 1.0, (t - 0.30) * 6.66, 1.0)
            } else if t < 0.60 {
                Color::new(1.0 - (t - 0.45) * 6.66, 1.0, 0.0, 1.0)
            } else if t < 0.75 {
                Color::new(0.0, 1.0, (t - 0.60) * 6.66, 1.0)
            } else if t < 0.90 {
                Color::new(0.0, 1.0 - (t - 0.75) * 6.66, 1.0, 1.0)
            } else if t < 0.98 {
                Color::new((t - 0.90) * 12.5, 0.0, 1.0, 1.0)
            } else {
                Color::new(1.0, 1.0, 1.0, 1.0)
            };
            draw_rectangle(x, y, square_width, square_height, color);
            if idx == vm.pc {
                draw_rectangle_lines(x, y, square_width, square_height, 5.0, WHITE);
            }
        }
    }
    // Draw the current number of steps centered and large
    let steps_text = format!("{}", vm.total_steps_count);
    let text_size = grid_size * 0.5;
    let text_dimensions = measure_text(&steps_text, None, text_size as u16, 1.0);
    let text_x = offset_x + (grid_size - text_dimensions.width) / 2.0;
    let text_y = offset_y + (grid_size + text_dimensions.height) / 2.0;
    draw_text(&steps_text, text_x, text_y, text_size, WHITE);
    // Draw the log view to the right of the VM grid (no background, white text)
    let log_width = grid_size * 1.2;
    let log_height = grid_size;
    let log_x = offset_x + grid_size + padding * 2.0;
    let log_y = offset_y;
    let log_font_size = (grid_size / 18.0).max(12.0);
    let mut y = log_y + log_font_size + 4.0;
    let max_lines = (log_height / (log_font_size + 2.0)).floor() as usize;
    let start_idx = if vm.recent_instructions.len() > max_lines {
        vm.recent_instructions.len() - max_lines
    } else {
        0
    };
    for line in vm.recent_instructions.iter().skip(start_idx) {
        draw_text(line, log_x + 8.0, y, log_font_size, WHITE);
        y += log_font_size + 2.0;
    }
}


// Configure tracing subscriber for logging
fn configure_tracing() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::filter::LevelFilter;
    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(LevelFilter::INFO))
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
    let vm_cols = 4;
    let vm_count = vm_rows * vm_cols;
    let mut vms: Vec<compute::VM> = (0..vm_count).map(|_| {
        let mut vm = compute::VM::new();
        vm.randomize(&mut rng);
        vm
    }).collect();

    let mut paused = false;

    let mut step_delay_ms: f64 = 10.0; // milliseconds between VM steps
    let mut last_step_time = get_time();


    loop {
        clear_background(BLACK);

        let padding = 5.0;
        let extra_padding = 10.0; // Extra padding between VMs
    // Calculate cell size so that all VMs fit and use all available space
    let available_width = screen_width() - (padding + extra_padding) * (vm_cols as f32 + 1.0);
    let available_height = screen_height() - (padding + extra_padding) * (vm_rows as f32 + 1.0);
    let cell_width = available_width / vm_cols as f32;
    let cell_height = available_height / vm_rows as f32;

    // Calculate total grid size
    let total_grid_width = vm_cols as f32 * cell_width + (vm_cols as f32 + 1.0) * (padding + extra_padding);
    let total_grid_height = vm_rows as f32 * cell_height + (vm_rows as f32 + 1.0) * (padding + extra_padding);

    // Calculate offsets to center the grid
    let start_x = (screen_width() - total_grid_width) / 2.0 + padding + extra_padding;
    let start_y = (screen_height() - total_grid_height) / 2.0 + padding + extra_padding;

        // Arrange VMs in a vm_rows x vm_cols grid
        for i in 0..vm_count {
            let row = i / vm_cols;
            let col = i % vm_cols;
            let offset_x = start_x + col as f32 * (cell_width + padding + extra_padding);
            let offset_y = start_y + row as f32 * (cell_height + padding + extra_padding);
            // Draw background
            draw_rectangle(
                offset_x - padding,
                offset_y - padding,
                cell_width + 2.0 * padding,
                cell_height + 2.0 * padding,
                DARKGRAY,
            );
            // Center the VM grid inside the background rectangle
            let vm_size = cell_width.min(cell_height);
            let center_x = offset_x + (cell_width - vm_size) / 2.0;
            let center_y = offset_y + (cell_height - vm_size) / 2.0;
            draw_vm(&vms[i], center_x, center_y, vm_size, padding);
        }

        // Toggle pause/unpause with space
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
            info!("Simulation {}", if paused {"paused"} else {"running"});
        }

                // Adjust step_delay_ms with left/right arrows and R key
        if is_key_pressed(KeyCode::Right) {
            step_delay_ms *= 2.0;
            info!("step_delay_ms scaled up to {} ms", step_delay_ms);
        }
        if is_key_pressed(KeyCode::Left) {
            step_delay_ms = (step_delay_ms / 2.0).max(1.0);
            info!("step_delay_ms halved to {} ms", step_delay_ms);
        }
        if is_key_pressed(KeyCode::R) {
            step_delay_ms = 100.0;
            info!("step_delay_ms reset to 100 ms");
        }

        // Run simulation at user-defined interval if not paused
        let now = get_time();
    if !paused && (now - last_step_time) * 1000.0 >= step_delay_ms {
            for vm in &mut vms {
                vm.step();
            }
            last_step_time = now;
        }
        // Single step forward with 's' key when paused
        if paused && is_key_pressed(KeyCode::S) {
            info!("Single step");
            for vm in &mut vms {
                vm.step();
            }
        }
        // Toggle fullscreen with 'f' key
        if is_key_pressed(KeyCode::F) {
            set_fullscreen(true);
        }


        // If any VM is halted, check if it has the longest run
        for vm in &mut vms {
            if vm.halted {
                tracing::debug!("VM halted, generating new program and restarting");
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