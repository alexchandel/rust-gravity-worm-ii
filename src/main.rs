#![feature(default_type_params)]

extern crate piston;
extern crate sdl2;
extern crate sdl2_game_window;
extern crate gfx;
extern crate gfx_graphics;

use std::iter::count;

use piston::{
	Window,
	EventSettings,
	EventIterator,
};
use sdl2_game_window::WindowSDL2;

use gfx::{Device, DeviceHelper};
use gfx_graphics::G2D;

use piston::event::{
	RenderEvent,
	UpdateEvent,
	PressEvent,
	ReleaseEvent,
};
use piston::graphics::{
	BackEnd,
	ImageSize,
	AddColor,
	AddRectangle,
	Context,
    Draw
};
use piston::input::{
	Button,
	Keyboard,
	keyboard,
};
use piston::event::fps_counter::FPSCounter;

#[allow(non_camel_case_types)]
type pix_t = i32;

enum Status {
	Before,
	During,
	Dead
}

enum Direction {
	Up,
	Down
}
impl Direction {
	fn to_scalar(&self) -> pix_t {
		match *self {
			Up => -1,
			Down => 1
		}
	}
}

/// Player and wall locations, motions, score, time, and game state.
struct Game {
	size: [uint, ..2],
	block_width: pix_t,

	// cave_len: uint,
	cave_height: uint,
	// worm_len: uint,
	pub cave_top: Vec<pix_t>,
	pub cave_bottom: Vec<pix_t>,
	pub worm_height: Vec<f64>,
	cave_dir: Direction,
	worm_dir: Direction,
	worm_vel: pix_t,

	score: i64,
	dt: f64,
	status: Status
}

impl Game {
	fn new(x: uint, y: uint) -> Game {
		println!("Tap space to begin.\nHold to go up.\nRelease to fall.");
		let block_width = x/128;
		let x_blocks = x/block_width;
		let y_blocks = y/block_width;
		let worm_len = x_blocks/2;

		let mut cave_top: Vec<pix_t> = Vec::with_capacity(x_blocks);
		let mut cave_bottom: Vec<pix_t> = Vec::with_capacity(x_blocks);
		let mut worm_height: Vec<f64> = Vec::with_capacity(x_blocks);

		cave_top.extend(count(y_blocks as pix_t/8, 0).take(x_blocks));
		cave_bottom.extend(count(y_blocks as pix_t*7/8, 0).take(x_blocks));
		worm_height.extend(count(y_blocks as f64/2.0, 0.0).take(worm_len));

		return Game {
			size: [x, y],
			block_width: block_width as pix_t,

			// cave_len: x_blocks, // = cave_top.len()
			cave_height: y_blocks,
			// worm_len: worm_len, // = worm_height.len()
			cave_top: cave_top,
			cave_bottom: cave_bottom,
			worm_height: worm_height,
			cave_dir: Up,
			worm_dir: Down,
			worm_vel: -16,

			score: 0,
			dt: 0.0,
			status: Before,
		}
	}

	fn is_worm_collided(&self) -> bool {
		let len = self.worm_height.len();
		let height = self.worm_height[len-1];
		let top = self.cave_top[len-1] as f64 + 1.0;
		let bottom = self.cave_bottom[len-1] as f64;
		height < top || height > bottom
	}

	fn is_wall_collided(&self) -> bool {
		*self.cave_top.last().unwrap() <= 0 ||
		*self.cave_bottom.last().unwrap() >= self.cave_height as pix_t - 1
	}

	/// WARNING: Vec::remove is O(n)! Should use circular data structure
	fn update_dt(&mut self, dt: f64) {
		match self.status {
			Before => (),
			During => {
				self.dt += dt;
				if self.dt > 0.0625 {
					self.dt = 0.0;

					// check collision
					let mut thunk = 0i32;
					if self.is_wall_collided() {
						self.cave_dir = match self.cave_dir {
							Up => {thunk = -1; Down},
							Down => Up
						};
					}

					// check worm controls, clamp velocity to +-16
					self.worm_vel += self.worm_dir.to_scalar();
					self.worm_vel = match self.worm_vel {
						x if x < -16 => -16,
						x if x > 16 => 16,
						x => x
					};

					self.cave_top.remove(0);
					let last = *self.cave_top.last().unwrap();
					self.cave_top.push(last + self.cave_dir.to_scalar());

					self.cave_bottom.remove(0);
					let last = *self.cave_bottom.last().unwrap();
					self.cave_bottom.push(last + self.cave_dir.to_scalar()
						+ thunk);

					self.worm_height.remove(0);
					let last = *self.worm_height.last().unwrap();
					self.worm_height.push(last + self.worm_vel as f64 / 8.0);

					if self.is_worm_collided() {
						println!("DEAD: score = {}\nTap space to restart...",
							self.score);
						self.status = Dead;
					} else {
						self.score += 1;
					}
				}
			},
			Dead => ()
		}
	}

	/// Proper to couple this to graphics code? Why isn't Context a trait?
	/// Possible to shift canvas left, and just draw the change?
	/// TODO render text for instruction, score, and death message.
	fn render<B: BackEnd<I>, I: ImageSize>(&self, c: Context, g: &mut B) {
		let w = self.block_width as f64;
		// Draw top wall
		for (i, &h) in self.cave_top.iter().enumerate() {
			c.rect(w*i as f64, 0.0,
				w, w*h as f64 + w).rgb(0.0, 0.0, 0.0).draw(g);
		};
		// Draw bottom wall
		for (i, &h) in self.cave_bottom.iter().enumerate() {
			c.rect(w*i as f64, w*h as f64, w,
				self.size[1] as f64 - w*h as f64).rgb(0., 0., 0.).draw(g);
		};
		// Draw worm
		for (i, &h) in self.worm_height.iter().enumerate() {
			c.rect(w*i as f64, w*h as f64, w, w).rgb(0., 0., 0.).draw(g);
		}
	}

	fn press_btn(&mut self, button: Button) {
		match button {
        	Keyboard(key) => {
        		if key == keyboard::Space {
        			match self.status {
        				During => self.worm_dir = Up,
        				_ => ()
        			}
        		}
        	},
        	_ => ()
        }
	}

	fn release_btn(&mut self, button: Button) {
		match button {
        	Keyboard(key) => {
        		if key == keyboard::Space {
        			match self.status {
        				Before => self.status = During,
        				During => self.worm_dir = Down,
        				// Ovewrite self with new game. Rust is insane.
						// SURE this doesn't leak memory?
						Dead => *self = Game::new(self.size[0], self.size[1]),
        			}
        		}
        	},
        	_ => ()
        }
	}
}

fn main() {
	let mut game = Game::new(512, 512);

	// Why do I choose an OpenGL version if SDL is platform agnostic,
	// and wouldn't OS X choose one automatically anyways?
	let opengl_version = piston::shader_version::opengl::OpenGL_3_2;

	// Create an SDL2 window.
    let mut window = WindowSDL2::new(
        opengl_version,
        piston::WindowSettings {
            title: "Gravity worm".to_string(),
            size: [512, 512],
            fullscreen: false,
            exit_on_esc: true,
            samples: 4,
        }
    );
    // Some settings for how the game should be run.
    let event_settings = EventSettings {
        updates_per_second: 60,
        max_frames_per_second: 60
    };

    // Set up Gfx-SDL2 device nonsense. Better than opengl_graphics, but
    // still verbose. Does this get an OpenGL/DirectX context from SDL?
    // Is it already bound to a framebuffer?
    let mut device = gfx::GlDevice::new(|s| unsafe {
        std::mem::transmute(sdl2::video::gl_get_proc_address(s))
    });
    let frame = {
    	let (w, h) = window.get_size();
    	gfx::Frame::new(w as u16, h as u16)
    };
    let mut renderer = device.create_renderer();

    // Create a piston::graphics interface.
    let mut g2d = G2D::new(&mut device);

    let show_fps = true;
    let mut fps_counter = FPSCounter::new();
    // Create GameIterator to begin the event iteration loop.
    let mut event_iter = EventIterator::new(&mut window, &event_settings);
    loop {
        let e = match event_iter.next() {
            None => { break; }
            Some(e) => e
        };
    	e.render(|_| {
    		// Draw using the piston Context
    		g2d.draw(&mut renderer, &frame, |c, g| {
    			c.rgb(1.0, 1.0, 1.0).draw(g);
    			game.render(c, g);
    		});

    		// Draw to Gfx, which draws to SDL window.
    		device.submit(renderer.as_buffer());
    		renderer.reset();

            if show_fps {
                event_iter.window.window.set_title(format!("Gravity worm FPS {}", fps_counter.tick()).as_slice());
            }
    	});
        e.update(|args| {
        	game.update_dt(args.dt)
        });
        e.press(|button| {
        	game.press_btn(button)
        });
        e.release(|button| {
        	game.release_btn(button)
        });
    }
}
