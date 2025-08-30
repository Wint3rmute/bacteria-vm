use macroquad::prelude::*;
use ::rand::Rng;
use tracing::info;
use tracing_subscriber;

// Include the compute module from the parent project
use life::compute::{VM, MEM_SIZE};

// Memory-mapped I/O addresses (using the last bytes of address space)
const MOVE_UP_ADDR: usize = MEM_SIZE - 4;    // 252: Set to non-zero to move up
const MOVE_DOWN_ADDR: usize = MEM_SIZE - 3;  // 253: Set to non-zero to move down
const MOVE_LEFT_ADDR: usize = MEM_SIZE - 2;  // 254: Set to non-zero to move left
const MOVE_RIGHT_ADDR: usize = MEM_SIZE - 1; // 255: Set to non-zero to move right

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
        let mut rng = ::rand::rng();
        vm.randomize(&mut rng);
        
        Self {
            vm,
            x,
            y,
            color: Color::new(
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
                1.0,
            ),
            energy: 100.0,
            age: 0,
        }
    }

    pub fn from_vm(vm: VM, x: f32, y: f32) -> Self {
        let mut rng = ::rand::rng();
        Self {
            vm,
            x,
            y,
            color: Color::new(
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
                1.0,
            ),
            energy: 100.0,
            age: 0,
        }
    }

    /// Update the lifeform - run VM step and process movement commands
    pub fn update(&mut self, dt: f32) {
        if self.vm.halted {
            return;
        }

        // Run one VM step
        self.vm.step();
        
        // Process memory-mapped movement commands
        self.process_movement_commands(dt);
        
        // Age the creature and consume energy
        self.age += 1;
        self.energy -= dt * 0.5; // Slow energy drain
        
        // If energy is depleted, halt the VM
        if self.energy <= 0.0 {
            self.vm.halted = true;
        }
    }

    fn process_movement_commands(&mut self, dt: f32) {
        let speed = 50.0 * dt; // pixels per second
        
        // Check memory-mapped I/O addresses for movement commands
        if self.vm.memory[MOVE_UP_ADDR] != 0 {
            self.y -= speed;
            self.energy -= 0.1; // Movement costs energy
        }
        
        if self.vm.memory[MOVE_DOWN_ADDR] != 0 {
            self.y += speed;
            self.energy -= 0.1;
        }
        
        if self.vm.memory[MOVE_LEFT_ADDR] != 0 {
            self.x -= speed;
            self.energy -= 0.1;
        }
        
        if self.vm.memory[MOVE_RIGHT_ADDR] != 0 {
            self.x += speed;
            self.energy -= 0.1;
        }
        
        // Clear movement commands after processing (optional - you might want persistence)
        // self.vm.memory[MOVE_UP_ADDR] = 0;
        // self.vm.memory[MOVE_DOWN_ADDR] = 0;
        // self.vm.memory[MOVE_LEFT_ADDR] = 0;
        // self.vm.memory[MOVE_RIGHT_ADDR] = 0;
    }

    pub fn draw(&self, camera_x: f32, camera_y: f32, zoom: f32) {
        let screen_x = (self.x - camera_x) * zoom + screen_width() / 2.0;
        let screen_y = (self.y - camera_y) * zoom + screen_height() / 2.0;
        
        // Only draw if on screen
        if screen_x >= -10.0 && screen_x <= screen_width() + 10.0 
            && screen_y >= -10.0 && screen_y <= screen_height() + 10.0 {
            
            let size = 8.0 * zoom;
            
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
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        !self.vm.halted && self.energy > 0.0
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
            move_speed: 200.0,
            zoom_speed: 1.1,
        }
    }

    pub fn update(&mut self, dt: f32) {
        // Camera movement with WASD or arrow keys
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            self.y -= self.move_speed * dt / self.zoom;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            self.y += self.move_speed * dt / self.zoom;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            self.x -= self.move_speed * dt / self.zoom;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            self.x += self.move_speed * dt / self.zoom;
        }

        // Zoom with Q/E or scroll wheel
        if is_key_down(KeyCode::Q) {
            self.zoom /= self.zoom_speed.powf(dt * 2.0);
        }
        if is_key_down(KeyCode::E) {
            self.zoom *= self.zoom_speed.powf(dt * 2.0);
        }

        // Handle mouse wheel for zooming
        let (_x, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            if wheel_y > 0.0 {
                self.zoom *= self.zoom_speed;
            } else {
                self.zoom /= self.zoom_speed;
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
    
    // Spawn initial population
    let mut rng = ::rand::rng();
    for _ in 0..20 {
        let x = rng.random_range(-200.0..200.0);
        let y = rng.random_range(-200.0..200.0);
        lifeforms.push(Lifeform::new(x, y));
    }

    loop {
        let dt = get_frame_time();
        clear_background(BLACK);

        // Update camera
        camera.update(dt);

        // Update all lifeforms
        for lifeform in &mut lifeforms {
            lifeform.update(dt);
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
        if (current_time - last_spawn_time > 5.0 && lifeforms.len() < 10) 
            || lifeforms.is_empty() {
            
            if lifeforms.is_empty() {
                generation += 1;
                info!("Starting generation {}", generation);
            }
            
            // Spawn new random lifeforms
            for _ in 0..5 {
                let x = rng.random_range(-400.0..400.0);
                let y = rng.random_range(-400.0..400.0);
                lifeforms.push(Lifeform::new(x, y));
            }
            
            last_spawn_time = current_time;
        }

        // Draw all lifeforms
        for lifeform in &lifeforms {
            lifeform.draw(camera.x, camera.y, camera.zoom);
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
        draw_text(&format!("Generation: {}", generation), 10.0, 30.0, 20.0, WHITE);
        draw_text(&format!("Lifeforms: {}", lifeforms.len()), 10.0, 50.0, 20.0, WHITE);
        draw_text(&format!("Camera: ({:.1}, {:.1}) Zoom: {:.2}", camera.x, camera.y, camera.zoom), 10.0, 70.0, 20.0, WHITE);
        draw_text("Controls: WASD/Arrows = Move, Q/E/Scroll = Zoom", 10.0, 90.0, 16.0, LIGHTGRAY);
        
        // Draw memory-mapped I/O legend
        draw_text("Memory-Mapped Movement:", 10.0, screen_height() - 80.0, 16.0, YELLOW);
        draw_text(&format!("Up: addr {} | Down: addr {}", MOVE_UP_ADDR, MOVE_DOWN_ADDR), 10.0, screen_height() - 60.0, 14.0, LIGHTGRAY);
        draw_text(&format!("Left: addr {} | Right: addr {}", MOVE_LEFT_ADDR, MOVE_RIGHT_ADDR), 10.0, screen_height() - 40.0, 14.0, LIGHTGRAY);
        draw_text("Set these memory locations to non-zero to move", 10.0, screen_height() - 20.0, 14.0, LIGHTGRAY);

        // ESC to quit
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await
    }
}
