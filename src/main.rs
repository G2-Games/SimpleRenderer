#![deny(clippy::all)]
#![forbid(unsafe_code)]

use log::error;
use image::{GenericImageView, DynamicImage, Rgba};
use pixels::{Error, PixelsBuilder, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use std::{time::{SystemTime, UNIX_EPOCH}, path::Path};

const WORLD_WIDTH: u32 = 256;
const WORLD_HEIGHT: u32 = 144;

struct World {
    right_held: bool,
    left_held: bool,
    background_image: DynamicImage,
    sprites: Vec<Sprite>,
}


/// The basic sprite struct, is used to draw an object to the output
struct Sprite {
    size: (u16, u16),
    facing_left: bool,
    position: (f32, f32),
    velocity: (f32, f32),
    sprite_sheet: SpriteSheet,
}

/// Sprite sheet, stores the different looks of a sprite
struct SpriteSheet {
    texture: DynamicImage,
    frame_size: (u16, u16),
    animations: Vec<Animation>,
    current_animation: usize,
    sheet_dimensions: (u16, u16),
}

/// Animations for a sprite sheet
struct Animation {
    starting_frame_position: (u16, u16),
    num_frames: u16,
    frame_duration: u64,
    current_frame: u16,
    current_position: (u16, u16),
    previous_frame_time: i128,
}

fn get_current_time() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => return n.as_millis(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

impl Sprite {
    fn new(sprite_sheet:SpriteSheet) -> Self {
        Self {
            size: sprite_sheet.frame_size,
            sprite_sheet,
            facing_left: false,

            // Physics information
            // TODO: move this to its own struct
            position: (0.0, 0.0),
            velocity: (0.0, 0.0),
        }
    }

    fn collision_y(&mut self) -> bool {
        if self.position.1 as u16 + self.size.0 >= WORLD_HEIGHT as u16 {
            return true;
        }
        return false;
    }

    fn run_animation(&mut self) {
        let frame_size = self.sprite_sheet.frame_size;
        let sheet_width = self.sprite_sheet.sheet_dimensions.0;
        let current = self.sprite_sheet.current_animation;

        self.sprite_sheet.animations[current].increment_frame(frame_size, sheet_width);
    }

    fn get_sheet_offset(&self) -> (u16, u16) {
        let current = self.sprite_sheet.current_animation;
        return self.sprite_sheet.animations[current].current_position;
    }

    fn get_sprite_sheet(&self) -> &DynamicImage {
        let sheet = &self.sprite_sheet.texture;
        return sheet;
    }
}

impl SpriteSheet {
    fn new(texture:DynamicImage, animations:Vec<Animation>, frame_size:(u16, u16)) -> Self {
        let height = texture.height();
        let width = texture.width();
        Self {
            texture,
            frame_size,
            animations,
            current_animation: 0,
            sheet_dimensions: (width as u16, height as u16),
        }
    }
}

/// Create a new animation instance
impl Animation {
    fn new(starting_frame_position:(u16, u16), num_frames:u16, frame_duration:u64) -> Self {
        Self {
            starting_frame_position,
            num_frames,
            frame_duration,
            current_frame: 0,
            current_position: starting_frame_position,
            previous_frame_time: 0,
        }
    }

    /// If the duration has elapsed, increment the currently selected animation by 1 frame
    fn increment_frame(&mut self, frame_size:(u16, u16), sheet_width:u16) {
        if self.frame_duration == 0 {
            return;
        }

        let current_time = get_current_time() as i128;

        // Only increment the frame if time has elapsed
        if current_time - self.previous_frame_time < self.frame_duration as i128 {
            return;
        }

        if self.current_frame >= self.num_frames - 1 {
            self.current_frame = 0;
            self.current_position = self.starting_frame_position;
        } else {
            if self.current_position.0 + frame_size.0 * 2 >= sheet_width {
                self.current_position = (0, self.current_position.1 + frame_size.1);
            } else {
                self.current_position = (self.current_position.0 + frame_size.0, self.current_position.1);
            }
            self.current_frame += 1;
        }

        self.previous_frame_time = current_time;
    }
}

/// Create a new `World` instance that can draw sprites
impl World {
    fn new(sprites: Vec<Sprite>) -> Self {
        Self {
            right_held: false,
            left_held: false,
            background_image: image::open(&Path::new("assets/images/bg.png")).unwrap(),
            sprites,
        }
    }

    /// Set the animation to use in the
    fn set_sprite_animation(&mut self, sprite:usize, animation_index:usize) {
        self.sprites[sprite].sprite_sheet.current_animation = animation_index;
    }

    /// Update held/released keys
    fn key_held(&mut self, key_id:VirtualKeyCode) {
        if key_id == VirtualKeyCode::Right {
            self.right_held = true;
        }
        if key_id == VirtualKeyCode::Left {
            self.left_held = true;
        }
    }

    fn key_released(&mut self, key_id:VirtualKeyCode) {
        if key_id == VirtualKeyCode::Right {
            self.right_held = false;
        }
        if key_id == VirtualKeyCode::Left {
            self.left_held = false;
        }
    }

    fn set_velocity_y(&mut self, velocity:f32, sprite_index:usize) {
        self.sprites[sprite_index].velocity.1 = velocity;
    }

    fn update_movement(&mut self) {
        if self.right_held && self.sprites[0].velocity.0.abs() < 4.75 {
            self.sprites[0].velocity.0 += 0.3;
        }

        if self.left_held && self.sprites[0].velocity.0.abs() < 4.75 {
            self.sprites[0].velocity.0 -= 0.3;
        }

        // Smooth out floating point errors
        self.sprites[0].velocity.0 = (self.sprites[0].velocity.0 * 1000.0).round() / 1000.0;
        if self.sprites[0].velocity.0.abs() < 0.3 {
            self.sprites[0].velocity.0 = 0.0;
        }
    }

    // TODO this is bad, completely refactor
    fn update_physics(&mut self) {
        let acceleration_y:f32 = 0.1;
        let mut friction_x:f32 = 0.01;

        if self.sprites[0].collision_y() {
            friction_x = 0.1;
        }

        // Move the sprite
        self.sprites[0].position.0 += self.sprites[0].velocity.0 / 5.0;
        self.sprites[0].position.1 += self.sprites[0].velocity.1 / 5.0;

        // Define the screen bounds
        if self.sprites[0].position.0 as i16 <= 0 || self.sprites[0].position.0 as u16 + self.sprites[0].size.0 > WORLD_WIDTH as u16 {
            self.sprites[0].velocity.0 = 0.0;
        } else if self.sprites[0].velocity.0 > 0.0 {
            self.sprites[0].velocity.0 -= friction_x;
        } else if self.sprites[0].velocity.0 < 0.0 {
            self.sprites[0].velocity.0 += friction_x;
        }

        self.sprites[0].position.0 = (self.sprites[0].position.0 * 100.0).round() / 100.0;

        if self.sprites[0].position.1 as u16 + self.sprites[0].size.1 < WORLD_HEIGHT as u16 && self.sprites[0].velocity.1 < 5.0 {
            self.sprites[0].velocity.1 += acceleration_y;
        } else if self.sprites[0].collision_y() {
            self.sprites[0].position.1 = WORLD_HEIGHT as f32 - self.sprites[0].size.1 as f32;
            self.sprites[0].velocity.1 = 0.0;
        }
    }

    /// Update all sprite frames
    fn update_sprite_animations(&mut self) {
        for i in 0..self.sprites.len() {
            self.sprites[i].run_animation();
        }
    }

    /// Draw the updated state of all sprites and background to the frame buffer.
    fn draw(&mut self, frame: &mut [u8]) -> Result<(), Box<dyn std::error::Error>> {

        // Run any animation updates
        self.update_sprite_animations();

        // Draw the background
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WORLD_WIDTH as usize) as u16;
            let y = (i / WORLD_WIDTH as usize) as u16;

            let background_pixel = self.background_image.get_pixel(x as u32, y as u32);
            let rgba = background_pixel.0;

            pixel.copy_from_slice(&rgba);
        }

        // Draw sprites on the background
        for i in 0..self.sprites.len() {
            let offset = self.sprites[i].get_sheet_offset();

            // Loop through all pixels in a sprite
            for z in 0..(self.sprites[i].size.0 * self.sprites[i].size.1) as usize {
                let x = (z % self.sprites[i].size.0 as usize) as i16;
                let y = (z / self.sprites[i].size.0 as usize) as u16;

                let viewport_y = (y as i32 + self.sprites[i].position.1 as i32) * WORLD_WIDTH as i32;
                let viewport_x = x as i32 + self.sprites[i].position.0 as i32;

                let index = ((viewport_y + viewport_x) * 4) as usize;

                let world_pixel;
                if index <= ((WORLD_WIDTH * WORLD_HEIGHT) * 4) as usize {
                    world_pixel = &mut frame[index..index + 4];
                } else {
                    world_pixel = &mut frame[0..4];
                }

                let colors:Rgba<u8>;
                let mut output:[u8; 4] = [0, 0, 0, 0];

                // Get the current sprite's pixel
                colors = self.sprites[i].get_sprite_sheet().get_pixel(
                    (x as u16 + offset.0) as u32,
                    (y as u16 + offset.1) as u32);

                // Create proper alpha blending for each color
                for c in 0..3 {
                    output[c] +=
                        (world_pixel[c] as f32 * (1.0 - (colors[3] as f32/255.0))) as u8  // Make the background blend
                        + (colors[c] as f32 * (colors[3] as f32/255.0)) as u8;            // Make the foreground blend
                }

                let rgba = [output[0], output[1], output[2], 255];

                // Write the output to the buffer
                world_pixel.copy_from_slice(&rgba);
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WORLD_WIDTH as f64 * 7.0, WORLD_HEIGHT as f64 * 7.0);
        WindowBuilder::new()
            .with_title("2D Rendering Test")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);

    let mut pixels = PixelsBuilder::new(WORLD_WIDTH, WORLD_HEIGHT, surface_texture)
    .enable_vsync(true)
    .build()?;

    let player_idle = Animation::new((0, 0), 4, 200);
    let player_slide = Animation::new((0, 74), 4, 100);
    let player_animations = vec![player_idle, player_slide];
    let player_sheet = SpriteSheet::new(image::open(&Path::new("assets/images/player_sheet.png")).unwrap(), player_animations, (50, 37));
    let player = Sprite::new(player_sheet);

    let window_static = Animation::new((0, 0), 1, 0);
    let window_animations = vec![window_static];
    let window_sheet = SpriteSheet::new(image::open(&Path::new("assets/images/building.png")).unwrap(), window_animations, (98, 72));
    let mut window_sprite = Sprite::new(window_sheet);

    window_sprite.position = (100.0, (WORLD_HEIGHT - window_sprite.size.1 as u32) as f32);

    let sprite_list: Vec<Sprite> = vec![player, window_sprite];

    let mut world = World::new(sprite_list);

    event_loop.run(move |event, _, control_flow| {

        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.get_frame_mut()).ok();
            if let Err(err) = pixels.render() {
                error!("pixels.render() failed: {err}");
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
            }

            if input.key_pressed(VirtualKeyCode::Up) {
                world.set_velocity_y(-7.0, 0);
                world.set_sprite_animation(0, 0);
            } else if input.key_released(VirtualKeyCode::Up) {
                world.set_sprite_animation(0, 0);
            }

            if input.key_pressed(VirtualKeyCode::Right) {
                world.key_held(VirtualKeyCode::Right);
            } else if input.key_released(VirtualKeyCode::Right) {
                world.key_released(VirtualKeyCode::Right);
            }

            if input.key_pressed(VirtualKeyCode::Left) {
                world.key_held(VirtualKeyCode::Left);
            } else if input.key_released(VirtualKeyCode::Left) {
                world.key_released(VirtualKeyCode::Left);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    error!("pixels.resize_surface() failed: {err}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            // Update internal state and request a redraw
            world.update_movement();
            world.update_physics();
            window.request_redraw();
        }
    });
}
