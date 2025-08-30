use ::rand::{thread_rng, Rng};
use macroquad::prelude::*;
use tracing::info;
use tracing_subscriber;

// Include the compute module from the parent project
use life::compute::{MEM_SIZE, VM};

// Memory-mapped I/O addresses (using the last bytes of address space)
const MOVE_LEFT_ADDR: usize = MEM_SIZE - 4; // 252: Left movement strength
const MOVE_RIGHT_ADDR: usize = MEM_SIZE - 3; // 253: Right movement strength  
const MOVE_UP_ADDR: usize = MEM_SIZE - 2; // 254: Up movement strength
const MOVE_DOWN_ADDR: usize = MEM_SIZE - 1; // 255: Down movement strength

// Sensory input addresses (using addresses before movement commands)
const FOOD_DISTANCE_X_ADDR: usize = MEM_SIZE - 6; // 250: X distance to nearest food (signed)
const FOOD_DISTANCE_Y_ADDR: usize = MEM_SIZE - 5; // 251: Y distance to nearest food (signed)

// Simulation constants
const INITIAL_POPULATION: usize = 20;
const MAX_ENERGY: f32 = 200.0;
const ENERGY_DRAIN_PER_FRAME: f32 = 0.1;
const MOVEMENT_ENERGY_COST: f32 = 0.2;
const MOVEMENT_SPEED: f32 = 1.0;
const EATING_RADIUS: f32 = 12.0;
const FOOD_SPAWN_INTERVAL: f64 = 2.0;
const MIN_FOOD_COUNT: usize = 10;
const INITIAL_FOOD_COUNT: usize = 15;
const FOOD_DISTRIBUTION_STD: f32 = 150.0;
const MAP_BOUNDARY: f32 = 400.0;
const LIFEFORM_SIZE: f32 = 8.0;

// Sensory system constants
const MAX_FOOD_DETECTION_RANGE: f32 = 100.0; // Maximum range for food detection
const SENSORY_SCALE_FACTOR: f32 = 2.0; // Scale factor to convert world distance to memory value

/// Food that provides energy to lifeforms
#[derive(Debug, Clone)]
pub struct Food {
    pub x: f32,
    pub y: f32,
    pub energy_value: f32,
}

impl Food {
    pub fn new(x: f32, y: f32, energy_value: f32) -> Self {
        Self { x, y, energy_value }
    }

    /// Create food with random energy value in a reasonable range
    pub fn new_random(x: f32, y: f32, rng: &mut impl Rng) -> Self {
        let energy_value = rng.gen_range(20.0..=50.0);
        Self::new(x, y, energy_value)
    }

    pub fn draw(&self, camera_x: f32, camera_y: f32, zoom: f32) {
        let screen_pos = self.world_to_screen(camera_x, camera_y, zoom);
        
        // Only draw if on screen
        if !self.is_on_screen(screen_pos, zoom) {
            return;
        }

        let size = (4.0 + self.energy_value / 10.0) * zoom;

        // Draw food as a green circle with brightness based on energy value
        let brightness = (self.energy_value / 50.0).clamp(0.3, 1.0);
        let food_color = Color::new(0.2, brightness, 0.3, 1.0);

        draw_circle(screen_pos.0, screen_pos.1, size, food_color);

        // Add a small white center for visibility
        if size > 2.0 {
            draw_circle(screen_pos.0, screen_pos.1, size * 0.3, WHITE);
        }
    }

    fn world_to_screen(&self, camera_x: f32, camera_y: f32, zoom: f32) -> (f32, f32) {
        let screen_x = (self.x - camera_x) * zoom + screen_width() / 2.0;
        let screen_y = (self.y - camera_y) * zoom + screen_height() / 2.0;
        (screen_x, screen_y)
    }

    fn is_on_screen(&self, screen_pos: (f32, f32), zoom: f32) -> bool {
        let margin = 10.0 * zoom;
        screen_pos.0 >= -margin
            && screen_pos.0 <= screen_width() + margin
            && screen_pos.1 >= -margin
            && screen_pos.1 <= screen_height() + margin
    }
}

/// Generate a normally distributed random number using Box-Muller transform
/// This is more efficient than the previous version and avoids potential edge cases
fn normal_random(mean: f32, std_dev: f32, rng: &mut impl Rng) -> f32 {
    // Box-Muller transform - generate two independent uniform random numbers
    let u1: f32 = rng.gen_range(f32::EPSILON..1.0); // Avoid exactly 0.0
    let u2: f32 = rng.gen_range(0.0..1.0);
    
    // Box-Muller transform
    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
    
    z0 * std_dev + mean
}

/// Clamp coordinates to map boundaries
fn clamp_to_map_bounds(coord: f32) -> f32 {
    coord.clamp(-MAP_BOUNDARY, MAP_BOUNDARY)
}

/// A simulated bacteria/lifeform controlled by a VM
#[derive(Debug, Clone)]
pub struct Lifeform {
    pub vm: VM,
    pub x: f32,
    pub y: f32,
    pub color: Color,
    pub energy: f32,
    pub age: u32,
}

impl Lifeform {
    pub fn new(x: f32, y: f32) -> Self {
        let mut vm = VM::new();
        let mut rng = thread_rng();
        vm.randomize(&mut rng);

        Self {
            vm,
            x,
            y,
            color: Self::random_color(&mut rng),
            energy: 100.0,
            age: 0,
        }
    }

    pub fn from_vm(vm: VM, x: f32, y: f32) -> Self {
        let mut rng = thread_rng();
        Self {
            vm,
            x,
            y,
            color: Self::random_color(&mut rng),
            energy: 100.0,
            age: 0,
        }
    }

    fn random_color(rng: &mut impl Rng) -> Color {
        Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            1.0,
        )
    }

    /// Update the lifeform - run VM step and process movement commands
    pub fn update(&mut self, food_items: &[Food]) {
        self.update_sensory_input(food_items);
        self.restart_vm_if_halted();
        self.vm.step();
        self.process_movement_commands();
        self.age_and_consume_energy();
    }

    /// Update sensory input by finding the nearest food and writing distance to memory
    fn update_sensory_input(&mut self, food_items: &[Food]) {
        if let Some((distance_x, distance_y)) = self.find_nearest_food_distance(food_items) {
            // Convert world coordinates to memory values (scaled and clamped to u8 range)
            let memory_x = self.distance_to_memory_value(distance_x);
            let memory_y = self.distance_to_memory_value(distance_y);
            
            self.vm.memory[FOOD_DISTANCE_X_ADDR] = memory_x;
            self.vm.memory[FOOD_DISTANCE_Y_ADDR] = memory_y;
        } else {
            // No food detected within range - write neutral values
            self.vm.memory[FOOD_DISTANCE_X_ADDR] = 128; // Neutral (middle value)
            self.vm.memory[FOOD_DISTANCE_Y_ADDR] = 128; // Neutral (middle value)
        }
    }

    /// Find the nearest food within detection range and return relative distance
    fn find_nearest_food_distance(&self, food_items: &[Food]) -> Option<(f32, f32)> {
        let mut nearest_distance_squared = MAX_FOOD_DETECTION_RANGE * MAX_FOOD_DETECTION_RANGE;
        let mut nearest_food_pos: Option<(f32, f32)> = None;

        for food in food_items {
            let dx = food.x - self.x;
            let dy = food.y - self.y;
            let distance_squared = dx * dx + dy * dy;

            if distance_squared < nearest_distance_squared {
                nearest_distance_squared = distance_squared;
                nearest_food_pos = Some((dx, dy));
            }
        }

        nearest_food_pos
    }

    /// Convert a world distance to a memory value (0-255)
    /// Positive distances map to 128-255, negative to 0-127, with 128 being neutral
    fn distance_to_memory_value(&self, distance: f32) -> u8 {
        let scaled_distance = distance * SENSORY_SCALE_FACTOR;
        let clamped = scaled_distance.clamp(-128.0, 127.0);
        ((clamped + 128.0) as u8)
    }

    fn restart_vm_if_halted(&mut self) {
        if self.vm.halted {
            self.vm.halted = false;
            self.vm.pc = 0; // Restart from beginning
        }
    }

    fn age_and_consume_energy(&mut self) {
        self.age += 1;
        self.energy -= ENERGY_DRAIN_PER_FRAME;
    }

    fn process_movement_commands(&mut self) {
        // Compare values to determine movement direction
        let movement_values = [
            self.vm.memory[MOVE_LEFT_ADDR],
            self.vm.memory[MOVE_RIGHT_ADDR],
            self.vm.memory[MOVE_UP_ADDR],
            self.vm.memory[MOVE_DOWN_ADDR],
        ];

        // Horizontal movement: move in direction of larger value
        if movement_values[0] > movement_values[1] {
            self.move_and_consume_energy(-MOVEMENT_SPEED, 0.0);
        } else if movement_values[1] > movement_values[0] {
            self.move_and_consume_energy(MOVEMENT_SPEED, 0.0);
        }

        // Vertical movement: move in direction of larger value
        if movement_values[2] > movement_values[3] {
            self.move_and_consume_energy(0.0, -MOVEMENT_SPEED);
        } else if movement_values[3] > movement_values[2] {
            self.move_and_consume_energy(0.0, MOVEMENT_SPEED);
        }
    }

    fn move_and_consume_energy(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
        self.energy -= MOVEMENT_ENERGY_COST;
    }

    pub fn draw(&self, camera_x: f32, camera_y: f32, zoom: f32) {
        let screen_x = (self.x - camera_x) * zoom + screen_width() / 2.0;
        let screen_y = (self.y - camera_y) * zoom + screen_height() / 2.0;

        // Only draw if on screen
        if screen_x >= -10.0
            && screen_x <= screen_width() + 10.0
            && screen_y >= -10.0
            && screen_y <= screen_height() + 10.0
        {
            let size = LIFEFORM_SIZE * zoom;

            // Draw the lifeform as a circle
            let brightness = (self.energy / 100.0).clamp(0.2, 1.0);
            let final_color = Color::new(
                self.color.r * brightness,
                self.color.g * brightness,
                self.color.b * brightness,
                self.color.a,
            );

            draw_circle(screen_x, screen_y, size, final_color);

            // Draw energy bar above the creature
            if size > 4.0 {
                let bar_width = size * 2.0;
                let bar_height = 2.0;
                let bar_x = screen_x - bar_width / 2.0;
                let bar_y = screen_y - size - 8.0;

                // Background
                draw_rectangle(bar_x, bar_y, bar_width, bar_height, DARKGRAY);
                // Energy level
                let energy_width = bar_width * (self.energy / 100.0).clamp(0.0, 1.0);
                draw_rectangle(bar_x, bar_y, energy_width, bar_height, GREEN);

                // Draw PC value below the energy bar
                let pc_text = format!("PC:{}", self.vm.pc);
                let font_size = (20.0 * zoom); //.max(8.0).min(12.0); // Scale with zoom but keep readable
                let text_x = screen_x - (pc_text.len() as f32 * font_size * 0.3); // Center text roughly
                let text_y = bar_y + bar_height + font_size + 2.0;
                draw_text(&pc_text, text_x, text_y, font_size, WHITE);
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        self.energy > 0.0 // Only check energy, not VM halt status
    }

    /// Check if this lifeform collides with food (within eating distance)
    pub fn can_eat_food(&self, food: &Food) -> bool {
        let distance_squared = (self.x - food.x).powi(2) + (self.y - food.y).powi(2);
        distance_squared <= EATING_RADIUS * EATING_RADIUS
    }

    /// Consume food and gain energy
    pub fn eat_food(&mut self, food: &Food) {
        self.energy = (self.energy + food.energy_value).min(MAX_ENERGY);
    }
}

/// Draw a single VM's memory as a grid at the given offset
fn draw_vm(vm: &VM, offset_x: f32, offset_y: f32, grid_size: f32, padding: f32) {
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

            // Highlight memory-mapped I/O addresses
            if idx >= MOVE_LEFT_ADDR && idx <= MOVE_DOWN_ADDR {
                draw_rectangle_lines(x, y, square_width, square_height, 2.0, YELLOW);
            }
            // Highlight sensory input addresses
            if idx == FOOD_DISTANCE_X_ADDR || idx == FOOD_DISTANCE_Y_ADDR {
                draw_rectangle_lines(x, y, square_width, square_height, 2.0, SKYBLUE);
            }
        }
    }
    // Draw the current number of steps centered and large
    let steps_text = format!("{}", vm.total_steps_count);
    let text_size = grid_size * 0.3;
    let text_dimensions = measure_text(&steps_text, None, text_size as u16, 1.0);
    let text_x = offset_x + (grid_size - text_dimensions.width) / 2.0;
    let text_y = offset_y + (grid_size + text_dimensions.height) / 2.0;
    draw_text(&steps_text, text_x, text_y, text_size, WHITE);

    // Draw the log view to the right of the VM grid (no background, white text)
    let _log_width = grid_size * 1.2;
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

/// Camera controller for navigating the simulation world
#[derive(Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            move_speed: 200.0, // Keep for potential future use
            zoom_speed: 1.1,   // Keep for potential future use
        }
    }

    pub fn update(&mut self) {
        // Camera movement with WASD keys only (arrows reserved for speed control)
        let move_speed = 5.0; // Fixed pixels per frame

        if is_key_down(KeyCode::W) {
            self.y -= move_speed / self.zoom;
        }
        if is_key_down(KeyCode::S) {
            self.y += move_speed / self.zoom;
        }
        if is_key_down(KeyCode::A) {
            self.x -= move_speed / self.zoom;
        }
        if is_key_down(KeyCode::D) {
            self.x += move_speed / self.zoom;
        }

        // Zoom with Q/E or scroll wheel
        let zoom_factor = 1.02; // Fixed zoom per frame

        if is_key_down(KeyCode::Q) {
            self.zoom /= zoom_factor;
        }
        if is_key_down(KeyCode::E) {
            self.zoom *= zoom_factor;
        }

        // Handle mouse wheel for zooming
        let (_x, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            if wheel_y > 0.0 {
                self.zoom *= 1.1;
            } else {
                self.zoom /= 1.1;
            }
        }

        // Clamp zoom
        self.zoom = self.zoom.clamp(0.1, 10.0);
    }
}

// Configure tracing subscriber for logging
fn configure_tracing() {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(LevelFilter::INFO))
        .init();
}

#[macroquad::main("Bacteria Simulation")]
async fn main() {
    configure_tracing();
    info!("Starting bacteria simulation");

    let mut camera = Camera::new();
    let mut lifeforms: Vec<Lifeform> = Vec::new();
    let mut generation = 0;
    let mut last_spawn_time = get_time();
    let mut selected_lifeform: Option<usize> = None;

    // Speed control variables
    let mut paused = false;
    let mut step_delay_ms: f64 = 16.0; // Default ~60 FPS
    let mut last_update_time = get_time();

    // Food system variables
    let mut food_items: Vec<Food> = Vec::new();
    let mut last_food_spawn_time = get_time();
    let map_center_x = 0.0;
    let map_center_y = 0.0;

    // Spawn initial population
    let mut rng = thread_rng();
    for _ in 0..INITIAL_POPULATION {
        let x = rng.gen_range(-200.0..200.0);
        let y = rng.gen_range(-200.0..200.0);
        lifeforms.push(Lifeform::new(x, y));
    }

    // Spawn initial food to ensure minimum count
    for _ in 0..INITIAL_FOOD_COUNT {
        let food_x = clamp_to_map_bounds(normal_random(map_center_x, FOOD_DISTRIBUTION_STD, &mut rng));
        let food_y = clamp_to_map_bounds(normal_random(map_center_y, FOOD_DISTRIBUTION_STD, &mut rng));
        let food = Food::new_random(food_x, food_y, &mut rng);
        food_items.push(food);
    }

    loop {
        clear_background(BLACK);

        // Update camera
        camera.update();

        // Speed control with arrow keys and pause functionality
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
            info!("Simulation {}", if paused { "paused" } else { "running" });
        }

        // Adjust step_delay_ms with left/right arrows
        if is_key_pressed(KeyCode::Right) {
            step_delay_ms = (step_delay_ms * 2.0).min(2000.0); // Max 2 seconds between steps
            info!(
                "Simulation speed decreased: {} ms between steps",
                step_delay_ms
            );
        }
        if is_key_pressed(KeyCode::Left) {
            step_delay_ms = (step_delay_ms / 2.0).max(1.0); // Min 1ms between steps
            info!(
                "Simulation speed increased: {} ms between steps",
                step_delay_ms
            );
        }

        // Update simulation based on timing and pause state
        let current_time = get_time();
        let should_update = if paused {
            // When paused, only update on 's' key press (single step)
            is_key_pressed(KeyCode::S)
        } else {
            // When running, update based on timing
            (current_time - last_update_time) * 1000.0 >= step_delay_ms
        };

        if should_update {
            // Update all lifeforms with sensory input
            for lifeform in &mut lifeforms {
                lifeform.update(&food_items);
            }
            last_update_time = current_time;

            if paused && is_key_pressed(KeyCode::S) {
                info!("Single step executed");
            }
        }

        // Food spawning (ensure minimum food count and spawn periodically using normal distribution)
        let current_time = get_time();

        // Check if we need to spawn food (either time-based or to maintain minimum count)
        let should_spawn_food = (current_time - last_food_spawn_time >= FOOD_SPAWN_INTERVAL)
            || (food_items.len() < MIN_FOOD_COUNT);

        if should_spawn_food {
            // Calculate how many food items to spawn
            let food_count = if food_items.len() < MIN_FOOD_COUNT {
                // Spawn enough to reach minimum count, plus 1-3 extra
                (MIN_FOOD_COUNT - food_items.len()) + rng.gen_range(1..=3)
            } else {
                // Regular spawning: 1-3 food items
                rng.gen_range(1..=3)
            };

            for _ in 0..food_count {
                let food_x = clamp_to_map_bounds(normal_random(map_center_x, FOOD_DISTRIBUTION_STD, &mut rng));
                let food_y = clamp_to_map_bounds(normal_random(map_center_y, FOOD_DISTRIBUTION_STD, &mut rng));
                let food = Food::new_random(food_x, food_y, &mut rng);
                food_items.push(food);
            }
            last_food_spawn_time = current_time;
        }

        // Food consumption (check collisions between lifeforms and food)
        for lifeform in &mut lifeforms {
            let mut eaten_food_indices = Vec::new();

            for (i, food) in food_items.iter().enumerate() {
                if lifeform.can_eat_food(food) {
                    lifeform.eat_food(food);
                    eaten_food_indices.push(i);
                    // Optional: log food consumption for debugging
                    // info!("Lifeform at ({:.1}, {:.1}) ate food worth {:.1} energy",
                    //       lifeform.x, lifeform.y, food.energy_value);
                }
            }

            // Remove eaten food (in reverse order to maintain indices)
            for &i in eaten_food_indices.iter().rev() {
                food_items.remove(i);
            }
        }

        // Remove dead lifeforms
        let alive_count = lifeforms.len();
        lifeforms.retain(|l| l.is_alive());
        let died_count = alive_count - lifeforms.len();

        if died_count > 0 {
            info!("Generation {}: {} lifeforms died", generation, died_count);
        }

        // Spawn new lifeforms periodically or when population is low
        let current_time = get_time();
        if (current_time - last_spawn_time > 5.0 && lifeforms.len() < 10) || lifeforms.is_empty() {
            if lifeforms.is_empty() {
                generation += 1;
                info!("Starting generation {}", generation);
            }

            // Spawn new random lifeforms
            for _ in 0..5 {
                let x = rng.gen_range(-MAP_BOUNDARY..MAP_BOUNDARY);
                let y = rng.gen_range(-MAP_BOUNDARY..MAP_BOUNDARY);
                lifeforms.push(Lifeform::new(x, y));
            }

            last_spawn_time = current_time;
        }

        // Handle mouse clicks to select lifeforms
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            selected_lifeform = None;

            // Convert mouse position to world coordinates
            let world_x = (mouse_x - screen_width() / 2.0) / camera.zoom + camera.x;
            let world_y = (mouse_y - screen_height() / 2.0) / camera.zoom + camera.y;

            // Find the closest lifeform within clicking distance
            let click_radius = 20.0 / camera.zoom; // Adjust click radius based on zoom
            for (idx, lifeform) in lifeforms.iter().enumerate() {
                let dx = lifeform.x - world_x;
                let dy = lifeform.y - world_y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= click_radius {
                    selected_lifeform = Some(idx);
                    info!(
                        "Selected lifeform {} at ({:.1}, {:.1})",
                        idx, lifeform.x, lifeform.y
                    );
                    break;
                }
            }
        }

        // Draw all lifeforms
        for (idx, lifeform) in lifeforms.iter().enumerate() {
            lifeform.draw(camera.x, camera.y, camera.zoom);

            // Highlight selected lifeform
            if Some(idx) == selected_lifeform {
                let screen_x = (lifeform.x - camera.x) * camera.zoom + screen_width() / 2.0;
                let screen_y = (lifeform.y - camera.y) * camera.zoom + screen_height() / 2.0;
                let size = 12.0 * camera.zoom;
                draw_circle_lines(screen_x, screen_y, size, 3.0, YELLOW);
            }
        }

        // Draw all food items
        for food in &food_items {
            food.draw(camera.x, camera.y, camera.zoom);
        }

        // Draw world bounds
        let world_size = 1000.0;
        let bounds = [
            (-world_size, -world_size, world_size * 2.0, 2.0), // Top
            (-world_size, world_size, world_size * 2.0, 2.0),  // Bottom
            (-world_size, -world_size, 2.0, world_size * 2.0), // Left
            (world_size, -world_size, 2.0, world_size * 2.0),  // Right
        ];

        for (bx, by, bw, bh) in bounds {
            let screen_x = (bx - camera.x) * camera.zoom + screen_width() / 2.0;
            let screen_y = (by - camera.y) * camera.zoom + screen_height() / 2.0;
            let screen_w = bw * camera.zoom;
            let screen_h = bh * camera.zoom;
            draw_rectangle(screen_x, screen_y, screen_w, screen_h, DARKGRAY);
        }

        // Draw UI
        draw_text(
            &format!("Generation: {}", generation),
            10.0,
            30.0,
            20.0,
            WHITE,
        );
        draw_text(
            &format!("Lifeforms: {}", lifeforms.len()),
            10.0,
            50.0,
            20.0,
            WHITE,
        );
        draw_text(
            &format!("Food: {}", food_items.len()),
            10.0,
            70.0,
            20.0,
            GREEN,
        );
        draw_text(
            &format!(
                "Camera: ({:.1}, {:.1}) Zoom: {:.2}",
                camera.x, camera.y, camera.zoom
            ),
            10.0,
            90.0,
            20.0,
            WHITE,
        );

        // Speed control UI
        let status_text = if paused { "PAUSED" } else { "RUNNING" };
        let status_color = if paused { RED } else { GREEN };
        draw_text(
            &format!("Status: {}", status_text),
            10.0,
            110.0,
            20.0,
            status_color,
        );
        draw_text(
            &format!("Speed: {:.1} ms/step", step_delay_ms),
            10.0,
            130.0,
            16.0,
            WHITE,
        );

        draw_text("Controls:", 10.0, 150.0, 16.0, YELLOW);
        draw_text(
            "WASD = Camera, Q/E/Scroll = Zoom",
            10.0,
            170.0,
            14.0,
            LIGHTGRAY,
        );
        draw_text(
            "SPACE = Pause/Unpause, S = Single Step",
            10.0,
            185.0,
            14.0,
            LIGHTGRAY,
        );
        draw_text(
            "Left/Right Arrows = Speed Control",
            10.0,
            200.0,
            14.0,
            LIGHTGRAY,
        );
        draw_text(
            "Click on a lifeform to inspect its VM",
            10.0,
            215.0,
            14.0,
            LIGHTGRAY,
        );

        // Draw VM inspector panel if a lifeform is selected
        if let Some(selected_idx) = selected_lifeform {
            if selected_idx < lifeforms.len() {
                let lifeform = &lifeforms[selected_idx];

                // Draw VM panel background
                let panel_size = 300.0;
                let panel_x = screen_width() - panel_size - 20.0;
                let panel_y = 20.0;

                draw_rectangle(
                    panel_x - 10.0,
                    panel_y - 10.0,
                    panel_size + 20.0,
                    panel_size + 140.0, // Increased height for sensory info
                    Color::new(0.0, 0.0, 0.0, 0.8),
                );
                draw_rectangle_lines(
                    panel_x - 10.0,
                    panel_y - 10.0,
                    panel_size + 20.0,
                    panel_size + 140.0, // Increased height for sensory info
                    2.0,
                    WHITE,
                );

                // Draw lifeform info
                draw_text(
                    &format!("Lifeform #{}", selected_idx),
                    panel_x,
                    panel_y - 5.0,
                    18.0,
                    YELLOW,
                );
                draw_text(
                    &format!("Energy: {:.1}", lifeform.energy),
                    panel_x,
                    panel_y + 15.0,
                    14.0,
                    WHITE,
                );
                draw_text(
                    &format!("Age: {}", lifeform.age),
                    panel_x,
                    panel_y + 30.0,
                    14.0,
                    WHITE,
                );
                draw_text(
                    &format!("Position: ({:.1}, {:.1})", lifeform.x, lifeform.y),
                    panel_x,
                    panel_y + 45.0,
                    14.0,
                    WHITE,
                );
                draw_text(
                    &format!("VM Steps: {}", lifeform.vm.total_steps_count),
                    panel_x,
                    panel_y + 60.0,
                    14.0,
                    WHITE,
                );
                draw_text(
                    &format!("PC: {}", lifeform.vm.pc),
                    panel_x,
                    panel_y + 75.0,
                    14.0,
                    WHITE,
                );

                // Display sensory input values
                let food_x_value = lifeform.vm.memory[FOOD_DISTANCE_X_ADDR];
                let food_y_value = lifeform.vm.memory[FOOD_DISTANCE_Y_ADDR];
                draw_text(
                    &format!("Food Sense X: {} ({})", food_x_value, 
                        if food_x_value < 128 { "Left" } 
                        else if food_x_value > 128 { "Right" } 
                        else { "Neutral" }),
                    panel_x,
                    panel_y + 90.0,
                    12.0,
                    SKYBLUE,
                );
                draw_text(
                    &format!("Food Sense Y: {} ({})", food_y_value,
                        if food_y_value < 128 { "Up" } 
                        else if food_y_value > 128 { "Down" } 
                        else { "Neutral" }),
                    panel_x,
                    panel_y + 105.0,
                    12.0,
                    SKYBLUE,
                );

                // Draw the VM memory grid
                draw_vm(&lifeform.vm, panel_x, panel_y + 120.0, panel_size, 1.0);
            } else {
                // Selected lifeform no longer exists (probably died)
                selected_lifeform = None;
            }
        }

        // Draw memory-mapped I/O legend
        draw_text(
            "Memory-Mapped I/O:",
            10.0,
            screen_height() - 120.0,
            16.0,
            YELLOW,
        );
        draw_text(
            "Movement (Comparative):",
            10.0,
            screen_height() - 100.0,
            14.0,
            YELLOW,
        );
        draw_text(
            &format!(
                "Left: addr {} | Right: addr {}",
                MOVE_LEFT_ADDR, MOVE_RIGHT_ADDR
            ),
            10.0,
            screen_height() - 80.0,
            12.0,
            LIGHTGRAY,
        );
        draw_text(
            &format!("Up: addr {} | Down: addr {}", MOVE_UP_ADDR, MOVE_DOWN_ADDR),
            10.0,
            screen_height() - 65.0,
            12.0,
            LIGHTGRAY,
        );
        draw_text(
            "Sensory Input:",
            10.0,
            screen_height() - 45.0,
            14.0,
            SKYBLUE,
        );
        draw_text(
            &format!(
                "Food X: addr {} | Food Y: addr {}",
                FOOD_DISTANCE_X_ADDR, FOOD_DISTANCE_Y_ADDR
            ),
            10.0,
            screen_height() - 25.0,
            12.0,
            LIGHTGRAY,
        );
        draw_text(
            "Values: 0-127=left/up, 128=neutral, 129-255=right/down",
            10.0,
            screen_height() - 10.0,
            10.0,
            LIGHTGRAY,
        );

        // ESC to quit
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await
    }
}
