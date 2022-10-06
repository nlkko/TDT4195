// Uncomment these following global attributes to silence most warnings of "low" interest:
/*
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
*/
extern crate nalgebra_glm as glm;
use std::{ mem, ptr, os::raw::c_void };
use std::f32::consts::PI;
use std::thread;
use std::sync::{Mutex, Arc, RwLock};
use glm::{Mat4, Vec3};

mod shader;
mod util;
mod mesh;
mod toolbox;
mod scene_graph;

use glutin::event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState::{Pressed, Released}, VirtualKeyCode::{self, *}};
use glutin::event_loop::ControlFlow;
use crate::mesh::Mesh;
use scene_graph::SceneNode;
use crate::toolbox::simple_heading_animation;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  pointer_to_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()


// == // Generate your VAO here
unsafe fn create_vao(vertices: &Vec<f32>, indices: &Vec<u32>, colours: &Vec<f32>, normals: &Vec<f32>) -> u32 {

    let mut vao_id: u32 = 0;
    let mut vbo_id: u32 = 0;
    let mut ibo_id: u32 = 0;
    let mut vbo_colour_id: u32 = 0;
    let mut vbo_normals_id: u32 = 0;

    // * VAO
    gl::GenVertexArrays(1, &mut vao_id);
    gl::BindVertexArray(vao_id);

    // * VBO
    gl::GenBuffers(1, &mut vbo_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id);
    gl::BufferData(gl::ARRAY_BUFFER, byte_size_of_array(&vertices), pointer_to_array(&vertices), gl::STATIC_DRAW);

    // * VAP
    gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
    gl::EnableVertexAttribArray(0);

    // * IBO
    gl::GenBuffers(1, &mut ibo_id);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo_id);
    gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, byte_size_of_array(&indices), pointer_to_array(&indices), gl::STATIC_DRAW);

    // * Configure colour
    gl::GenBuffers(1, &mut vbo_colour_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo_colour_id);
    gl::BufferData(gl::ARRAY_BUFFER, byte_size_of_array(&colours), pointer_to_array(&colours), gl::STATIC_DRAW);
    gl::VertexAttribPointer(1, 4, gl::FLOAT, gl::FALSE, 0, ptr::null());
    gl::EnableVertexAttribArray(1);

    // Light
    gl::GenBuffers(1, &mut vbo_normals_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo_normals_id);
    gl::BufferData(gl::ARRAY_BUFFER, byte_size_of_array(&normals), pointer_to_array(&normals), gl::STATIC_DRAW);
    gl::VertexAttribPointer(3, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
    gl::EnableVertexAttribArray(3);

    // * Return the ID of the VAO
    vao_id
}

unsafe fn draw_scene(node: &scene_graph::SceneNode, view_projection_matrix: &glm::Mat4, transformation_so_far: &glm::Mat4) {

    let mut transformation_matrix: glm::Mat4 = glm::identity();
    transformation_matrix *= transformation_so_far;
    transformation_matrix *= glm::translation(&node.position);
    transformation_matrix *= glm::translation(&node.reference_point);
    transformation_matrix *= glm::rotation(node.rotation[0], &glm::vec3(1.0, 0.0, 0.0));
    transformation_matrix *= glm::rotation(node.rotation[1], &glm::vec3(0.0, 1.0, 0.0));
    transformation_matrix *= glm::rotation(node.rotation[2], &glm::vec3(0.0, 0.0, 1.0));
    transformation_matrix *= glm::inverse(&glm::translation(&node.reference_point));

    // Check if node is drawable, if so: set uniforms and draw
    if node.index_count > 0 {
        let uniform_matrix = view_projection_matrix * transformation_matrix;
        gl::UniformMatrix4fv(2, 1, gl::FALSE, uniform_matrix.as_ptr());
        gl::BindVertexArray(node.vao_id);
        gl::DrawElements(gl::TRIANGLES, node.index_count, gl::UNSIGNED_INT, ptr::null());
    }

    // Recurse
    for &child in &node.children {
        draw_scene(&*child, view_projection_matrix, &transformation_matrix);
    }
}


fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(INITIAL_SCREEN_W, INITIAL_SCREEN_H));
    let cb = glutin::ContextBuilder::new()
        .with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!("{}: {}", util::get_gl_string(gl::VENDOR), util::get_gl_string(gl::RENDERER));
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!("GLSL\t: {}", util::get_gl_string(gl::SHADING_LANGUAGE_VERSION));
        }

        let lunarsurface = mesh::Terrain::load("resources/lunarsurface.obj");
        let helicopter = mesh::Helicopter::load("resources/helicopter.obj");

        let lunar_vao = unsafe { create_vao(&lunarsurface.vertices, &lunarsurface.indices, &lunarsurface.colors, &lunarsurface.normals)};
        let helicopter_body = unsafe { create_vao(&helicopter.body.vertices, &helicopter.body.indices, &helicopter.body.colors, &helicopter.body.normals)};
        let helicopter_door = unsafe { create_vao(&helicopter.door.vertices, &helicopter.door.indices, &helicopter.door.colors, &helicopter.door.normals)};
        let helicopter_main_rotor = unsafe { create_vao(&helicopter.main_rotor.vertices, &helicopter.main_rotor.indices, &helicopter.main_rotor.colors, &helicopter.main_rotor.normals)};
        let helicopter_tail_rotor = unsafe { create_vao(&helicopter.tail_rotor.vertices, &helicopter.tail_rotor.indices, &helicopter.tail_rotor.colors, &helicopter.tail_rotor.normals)};

        let mut root = SceneNode::new();
        let mut lunar_scene = SceneNode::from_vao(lunar_vao, lunarsurface.index_count);
        let mut body_scene = SceneNode::from_vao(helicopter_body, helicopter.body.index_count);
        let mut door_scene = SceneNode::from_vao(helicopter_door, helicopter.door.index_count);
        let mut main_rotor_scene = SceneNode::from_vao(helicopter_main_rotor, helicopter.main_rotor.index_count);
        let mut tail_rotor_scene = SceneNode::from_vao(helicopter_tail_rotor, helicopter.tail_rotor.index_count);

        body_scene.position = glm::vec3(10.0, 0.0, 0.0);

        main_rotor_scene.rotation = glm::vec3(0.0, 2.0, 0.0);
        tail_rotor_scene.rotation = glm::vec3(1.0, 0.0, 0.0);

        tail_rotor_scene.reference_point = glm::vec3(0.35, 2.3, 10.4);

        root.add_child(&lunar_scene);
        lunar_scene.add_child(&body_scene);
        body_scene.add_child(&door_scene);
        body_scene.add_child(&main_rotor_scene);
        body_scene.add_child(&tail_rotor_scene);


        // Shaders here
        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./shaders/simple.frag")
                .attach_file("./shaders/simple.vert")
                .link()
        };
        unsafe {
            gl::UseProgram(simple_shader.program_id);
        };

        // Used to demonstrate keyboard handling for exercise 2.
        let mut position = glm::vec3(0.0, 0.0, 0.0);
        let mut rotation = glm::vec2(0.0, 0.0);


        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut prevous_frame_time = first_frame_time;
        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(prevous_frame_time).as_secs_f32();
            prevous_frame_time = now;

            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Resized");
                    unsafe { gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32); }
                }
            }

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html

                        VirtualKeyCode::D => { // Right
                            position[0] -= delta_time * 30.0;
                        }
                        VirtualKeyCode::A => { // Left
                            position[0] += delta_time * 30.0;
                        }
                        VirtualKeyCode::Space => { // Up
                            position[1] -= delta_time * 30.0;
                        }
                        VirtualKeyCode::LShift => { // Down
                            position[1] += delta_time * 30.0;
                        }
                        VirtualKeyCode::S => { // Backwards
                            position[2] -= delta_time * 30.0;
                        }
                        VirtualKeyCode::W => { // Forward
                            position[2] += delta_time * 30.0;
                        }
                        VirtualKeyCode::Right => { // Roll right
                            rotation[1] += delta_time;
                        }
                        VirtualKeyCode::Left => { // Roll left
                            rotation[1] -= delta_time;
                        }
                        VirtualKeyCode::Up => { // Roll Backwards
                            rotation[0] += delta_time;
                        }
                        VirtualKeyCode::Down => { // Roll Forwards
                            rotation[0] -= delta_time;
                        }
                        // default handler:
                        _ => { }
                    }
                }
            }
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {

                // == // Optionally access the acumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (0.0, 0.0); // reset when done
            }

            // == // Please compute camera transforms here (exercise 2 & 3)
            let mut transformation_matrix: glm::Mat4 = glm::identity();
            transformation_matrix *= glm::perspective(1.0,PI / 2.0,1.0,1000.0);
            transformation_matrix *= glm::translation(&glm::vec3(0.0, 0.0, -1.5));
            transformation_matrix *= glm::translation(&position);
            transformation_matrix *= glm::rotation(rotation[0], &glm::vec3(1.0, 0.0, 0.0)) * glm::rotation(rotation[1], &glm::vec3(0.0, 1.0, 0.0));

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky, full opacity
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                main_rotor_scene.rotation += glm::vec3(0.0, delta_time * 5.0, 0.0);
                tail_rotor_scene.rotation += glm::vec3(delta_time * 5.0, 0.0, 0.0);

                let animation = simple_heading_animation(elapsed);
                body_scene.position[0] = animation.x;
                body_scene.position[1] = 0.0;
                body_scene.position[2] = animation.z;
                body_scene.rotation[0] = animation.pitch;
                body_scene.rotation[1] = animation.yaw;
                body_scene.rotation[2] = animation.roll;

                let mut temp: glm::Mat4 = glm::identity();
                draw_scene(&root, &transformation_matrix, &temp);
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });


    // == //
    // == // From here on down there are only internals.
    // == //


    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
                println!("New window size! width: {}, height: {}", physical_size.width, physical_size.height);
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent { event: WindowEvent::KeyboardInput {
                    input: KeyboardInput { state: key_state, virtual_keycode: Some(keycode), .. }, .. }, .. } => {

                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        },
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => { *control_flow = ControlFlow::Exit; }
                    _      => { }
                }
            }
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => { }
        }
    });
}
