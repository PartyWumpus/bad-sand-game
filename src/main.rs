use std::ops::{Index, IndexMut, Neg};
use rand::Rng;
use rand::seq::SliceRandom;
use enumset::{EnumSet, EnumSetType};

use crate::gui::Framework;
use error_iter::ErrorIter as _;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod gui;

const TICK_SPEED: u64 = 3;
const ARR_WIDTH: usize = 320/2;
const ARR_HEIGHT: usize = 240/2;

const WIDTH: u32 = 320*4;
const HEIGHT: u32 = 240*4;

/// Representation of the application state. In this example, a box will bounce around the screen.
//struct World {
//    box_x: i16,
//    box_y: i16,
//    velocity_x: i16,
//    velocity_y: i16,
//}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH, HEIGHT);
        WindowBuilder::new()
            .with_title("Hello Pixels + egui")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let (mut pixels, mut framework) = {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let surface_texture: SurfaceTexture<'_, winit::window::Window> = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(ARR_WIDTH as u32, ARR_HEIGHT as u32, surface_texture)?;
        let framework = Framework::new(
            &event_loop,
            window_size.width,
            window_size.height,
            scale_factor,
            &pixels,
        );

        (pixels, framework)
    };
    let mut world = GameState::new();

    event_loop.run(move |event, _, control_flow| {
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.mouse_held(0) {
                if let Some(real_pos) = input.mouse() {
                    world.matrix.add_square(pos_to_coord(real_pos),5,ElementType::Sand)
                }
            }

            if input.mouse_held(1) {
                if let Some(real_pos) = input.mouse() {
                    world.matrix.add_square(pos_to_coord(real_pos),5,ElementType::Water)
                }
            }

            if input.mouse_held(2) {
                if let Some(real_pos) = input.mouse() {
                    world.matrix.add_square(pos_to_coord(real_pos),5,ElementType::Stone)
                }
            }

            // Update the scale factor
            if let Some(scale_factor) = input.scale_factor() {
                framework.scale_factor(scale_factor);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                framework.resize(size.width, size.height);
            }

            // Update internal state and request a redraw
            world.update();
            window.request_redraw();
        }

        match event {
            Event::WindowEvent { event, .. } => {
                // Update egui inputs
                framework.handle_event(&event);
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                // Draw the world
                world.draw(pixels.frame_mut());

                // Prepare egui
                framework.prepare(&window);

                // Render everything together
                let render_result = pixels.render_with(|encoder, render_target, context| {
                    // Render the world texture
                    context.scaling_renderer.render(encoder, render_target);

                    // Render egui
                    framework.render(encoder, render_target, context);

                    Ok(())
                });

                // Basic error handling
                if let Err(err) = render_result {
                    log_error("pixels.render", err);
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => (),
        }
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

fn clamp(val: f32, max: usize) -> usize {
    if val < 0.0 {
        0
    }
    else if val > max as f32 {
        max
    }
    else {
        val as usize
    }
}

fn pos_to_coord(pos: (f32, f32)) -> Coordinate {
    Coordinate{x: clamp(pos.0/10.0, ARR_WIDTH), y: clamp(pos.1/10.0, ARR_HEIGHT)}
}


#[derive(Debug, PartialEq, Clone, Copy)]
struct NegCoordinate {
    x: i64,
    y: i64,
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct Coordinate {
    x: usize,
    y: usize,
}

impl Coordinate {
    fn in_bounds(self: &Self) -> bool {
        if self.x < 0 || self.x >= ARR_WIDTH {
            false
        }
        else if self.y < 0 || self.y  >= ARR_HEIGHT {
            false
        }
        else {
            true
        }
    }
}

impl From<Coordinate> for usize {
    fn from(c: Coordinate) -> Self {
        c.x + c.y * ARR_WIDTH
    }
}

impl From<usize> for Coordinate {
    fn from(i: usize) -> Self {
        Coordinate{x: i % ARR_WIDTH, y: i / ARR_WIDTH}
    }
}

// if this could be automatic it would be nice :(
//impl From<NegCoordinate> for Coordinate {
//    fn from(c: NegCoordinate) -> Self {
//        Coordinate{x:c.x as usize, y:c.y as usize}
//    }
//}

impl std::ops::Add for Coordinate {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Add<NegCoordinate> for Coordinate {
    type Output = Self;

    fn add(self, other: NegCoordinate) -> Self::Output {
        Coordinate {
            x: (self.x as i64 + other.x) as usize,
            y: (self.y as i64 + other.y) as usize,
        }
    }
}

struct CoordinateIterator {
    count: usize
}

impl CoordinateIterator {
    fn new() -> Self {
        Self { count: (ARR_HEIGHT*ARR_WIDTH)-2 }
    }
}

impl Iterator for CoordinateIterator {
    type Item = Coordinate;

    fn next(&mut self) -> Option<Self::Item> {
        self.count -= 1;

        if self.count > 0 {
            Some(Coordinate::from(self.count))
        } else {
            None
        }
    }
}

#[derive(Debug, EnumSetType)]
enum ElementType {
    EMPTY,
    Stone,
    Sand,
    Water,
}

#[derive(Copy, Clone)]
struct Element {
    position: Coordinate,
    flavor: ElementType,
}

impl Element {
    fn new(position: Coordinate, flavor: ElementType) -> Self {
        Self {
            position,
            flavor,
        }
    }
}

struct ElementMatrix {
    arr: Matrix,
    new_arr: Matrix,
}



impl ElementMatrix {
    fn new() -> Self {
        Self {
            arr: Matrix::new(),
            new_arr: Matrix::new(),
        }
    }

    fn add(self: &mut Self, element: Element) {
        let pos = (&element).position;
        self.new_arr[pos] = Some(element)
    }

    // TODO: i would prefer this. why does it not work??
    //fn add(self: &mut Self, element: Element) {
    //    self[(&element).position] = Some(element);
    //}


    fn add_square(self: &mut Self, coord: Coordinate, size: usize, flavor: ElementType) {
        for offset_x in coord.x-usize::div_ceil(size,2)..coord.x+(size/2) {
            for offset_y in coord.y-usize::div_ceil(size,2)..coord.y+(size/2) {
                self.add(Element::new(Coordinate{x:coord.x+offset_x,y:coord.y+offset_y},flavor));
            }
            
        }
        
    }

    // TODO: none can mean either nothing or out of bounds...
    // this is confusing i think
    fn get_from_new(self: &Self, index: Coordinate) -> Option<&Option<Element>> {
        if index.in_bounds() {
            Some(&self.new_arr[index])
        } else {
            None
        }
    }

    fn move_to(self: &mut Self, a: Coordinate, b: Coordinate) {
        if b.in_bounds() {
            self.new_arr.swap(a, b)
        }
    }

    fn attempt_directions(self: &mut Self, a: Coordinate, moves: &[Move]) {
        for _move in moves {
            let new_pos = a+*_move.directions.choose(&mut rand::thread_rng()).unwrap();
            if let Some(does_element_exist) = self.get_from_new(new_pos) {
                match does_element_exist {
                    Some(new_element) => {
                        if _move.flavors.contains(new_element.flavor){
                            self.move_to(a,new_pos);
                            return;
                        }
                    },
                    None => {
                        if _move.flavors.contains(ElementType::EMPTY) {
                            self.move_to(a,new_pos);
                            return;
                        }
                    },
                }
                
            }
        }
    }

    fn step(self: &mut Self, a: Coordinate) {
        if let Some(element) = &self.arr[a] {
            let moves = element_type_to_moveset(element.flavor);
            self.attempt_directions(a, &moves)
        }
    }

    fn finish_update(self: &mut Self) {
        unsafe {
            let a: *mut Matrix = &mut self.new_arr;
            let b: *mut Matrix = &mut self.arr;
            std::ptr::swap(a, b);
            *a = *b.clone();
        }
    }
}

fn element_type_to_moveset(flavor: ElementType) -> Vec<Move> {
    return match flavor {
        ElementType::Stone => vec![],
        ElementType::Sand => vec![
            Move{flavors:ElementType::EMPTY | ElementType::Water, directions:vec![NegCoordinate{x:0,y:2}]},
            Move{flavors:ElementType::EMPTY | ElementType::Water, directions:vec![NegCoordinate{x:0,y:1}]},
            Move{flavors:ElementType::EMPTY | ElementType::Water, directions:vec![NegCoordinate{x:1,y:1},NegCoordinate{x:-1,y:1}]},
            ],

        ElementType::Water => vec![
            Move{flavors:ElementType::EMPTY.into(), directions:vec![NegCoordinate{x:0,y:1}]},
            Move{flavors:ElementType::EMPTY.into(), directions:vec![NegCoordinate{x:-1,y:1},NegCoordinate{x:1,y:1}]},
            Move{flavors:ElementType::EMPTY.into(), directions:vec![NegCoordinate{x:-2,y:0},NegCoordinate{x:2,y:0}]},
            Move{flavors:ElementType::EMPTY.into(), directions:vec![NegCoordinate{x:-1,y:0},NegCoordinate{x:1,y:0}]},
            Move{flavors:ElementType::Sand.into(), directions:vec![NegCoordinate{x:0,y:-1}]}
            ],
        ElementType::EMPTY => unreachable!()
    };
}

struct Move {
    flavors: EnumSet<ElementType>,
    directions: Vec<NegCoordinate>,
}

#[derive(Copy, Clone)]
struct Matrix {
    arr: [Option<Element>; ARR_WIDTH * ARR_HEIGHT]
}

const EMPTY: Option<Element> = None;

impl Matrix {
    fn swap(self: &mut Self, a:Coordinate, b:Coordinate) {
        self.arr.swap(a.into(), b.into())
    }

    fn new() -> Self {
        Self {
            arr: [EMPTY; ARR_WIDTH * ARR_HEIGHT]
        }
    }
}

impl Index<Coordinate> for Matrix {
    type Output = Option<Element>;

    fn index(&self, index: Coordinate) -> &Self::Output {
        return &self.arr[usize::from(index)]
    }
}

impl IndexMut<Coordinate> for Matrix {
    fn index_mut(&mut self, index: Coordinate) -> &mut Self::Output {
        return &mut self.arr[usize::from(index)]
    }
}

struct GameState {
    matrix: ElementMatrix,
    framecount: u64,
}



impl GameState {
    /// Create a new `World` instance that can draw a moving box.
    fn new() -> Self {
        Self {
            matrix: ElementMatrix::new(),
            framecount: 0,
        }
    }

    /// Update the `World` internal state; bounce the box around the screen.
    fn update(&mut self) {
        /*if self.box_x <= 0 || self.box_x + BOX_SIZE > WIDTH as i16 {
            self.velocity_x *= -1;
        }
        if self.box_y <= 0 || self.box_y + BOX_SIZE > HEIGHT as i16 {
            self.velocity_y *= -1;
        }

        self.box_x += self.velocity_x;
        self.box_y += self.velocity_y;*/
        self.framecount += 1;

        if self.framecount % TICK_SPEED == 0 { 
            for index in CoordinateIterator::new() {
                self.matrix.step(index)
            }
            self.matrix.finish_update();
        }

        /*
        if self.framecount % (TICK_SPEED*300) == 0 {
            let coord = Coordinate{
                x:rand::thread_rng().gen_range(20..25),
                y:rand::thread_rng().gen_range(11..15)};
            self.matrix.add_square(coord,15,ElementType::Sand)
        }

        if self.framecount % (TICK_SPEED) == 0 {
            let coord = Coordinate{
                x:rand::thread_rng().gen_range(10..15),
                y:rand::thread_rng().gen_range(11..15)};
            self.matrix.add_square(coord,10,ElementType::Water)
        }
        */
        
    }

    fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = i % ARR_WIDTH;
            let y = i / ARR_WIDTH;

            let maybe_element = &self.matrix.arr[Coordinate{x,y}];


            let rgba = match maybe_element {
                None => [0x48, 0xb2, 0xe8, 0xff],
                Some(element) => match element.flavor {
                    ElementType::Stone => [0x40, 0x40, 0x40, 0xd0],
                    ElementType::Sand => [0x5e, 0x48, 0xe8, 0xd0],
                    ElementType::Water => [0x00, 0x00, 0xe8, 0xd0],
                    ElementType::EMPTY => unreachable!()
                },
            };

            pixel.copy_from_slice(&rgba);
        }
    }
}
