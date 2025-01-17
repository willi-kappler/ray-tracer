use std::fs::OpenOptions;
use std::io::prelude::*;
use rand::prelude::*;
use std::time::Instant;

use ray_tracer::vector::Vec3;
use ray_tracer::scene::Scene;
use ray_tracer::hitable::Hitable;
use ray_tracer::hitable::primitive::Sphere;
use ray_tracer::hitable::primitive::Rectangle;
use ray_tracer::hitable::primitive::Cube;
use ray_tracer::hitable::primitive::Group;
use ray_tracer::hitable::transform::Translation;
use ray_tracer::camera::Camera;
use ray_tracer::camera::perspective::PerspectiveCamera;
use ray_tracer::renderer::Renderer;
use ray_tracer::renderer::Image;
use ray_tracer::material::Material;
use ray_tracer::material::plain::PlainMaterial;
use ray_tracer::material::lambertian::LambertianMaterial;
use ray_tracer::material::metal::MetalMaterial;
use ray_tracer::material::dielectric::DielectricMaterial;
use ray_tracer::actor::Actor;
use ray_tracer::tree::TreeType;
use ray_tracer::texture::uniform::UniformTexture;
use ray_tracer::texture::checker::CheckerTexture;
use ray_tracer::constants::Axis;

fn to_u8(f: f64) -> u8 {
    (f * 255.0) as u8
}

fn mix_images(image: &mut Image<f64>, delta: &Image<f64>, iteration: usize) {
    assert_eq!(delta.height, image.height);
    assert_eq!(delta.width, image.width);

    let frac_delta = 1.0 / (iteration + 1) as f64;
    let frac_image = 1.0 - frac_delta;
    for j in 0..image.height {
        for i in 0..image.width {
            let index = j * image.width + i;
            image.data[3 * index] = frac_image * image.data[3 * index] + frac_delta * delta.data[3 * index];
            image.data[3 * index + 1] = frac_image * image.data[3 * index + 1] + frac_delta * delta.data[3 * index + 1];
            image.data[3 * index + 2] = frac_image * image.data[3 * index + 2] + frac_delta * delta.data[3 * index + 2];
        }
    }
}

fn image_diff(reference: &Image<f64>, image: &Image<f64>) -> f64 {
    assert_eq!(reference.height, image.height);
    assert_eq!(reference.width, image.width);

    let mut diff = 0.0;
    for j in 0..image.height {
        for i in 0..image.width {
            let index = j * image.width + i;
            let ref_color = Vec3::from_array([reference.data[3 * index], reference.data[3 * index + 1], reference.data[3 * index + 2]]);
            let image_color = Vec3::from_array([image.data[3 * index], image.data[3 * index + 1], image.data[3 * index + 2]]);
            diff += (ref_color - image_color).norm();
        }
    }
    diff
}

fn print_ppm(image: &Image<f64>, gamma: f64, filename: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(filename)
        .unwrap();

    if let Err(e) = writeln!(file, "P3\n# asd\n{} {}\n255", image.width, image.height) {
        eprintln!("Couldn't write to file: {}", e);
    }

    for j in 0..image.height {
        for i in 0..image.width {
            let index = j * image.width + i;
            if let Err(e) = writeln!(
                file, "{} {} {}",
                to_u8(image.data[3 * index].powf(1.0 / gamma)),
                to_u8(image.data[3 * index + 1].powf(1.0 / gamma)),
                to_u8(image.data[3 * index + 2].powf(1.0 / gamma))
            ) {
                eprintln!("Couldn't write to file: {}", e);
            }
        }
    }
}

fn create_rectangle_room(length: f64, width: f64, height: f64, light: f64) -> Vec<Actor<f64>> {
    let mut actors = vec![];

    let dimming = 1.0;

    // Rectangle used as light
    let _width_axis = Axis::X;
    let _height_axis = Axis::Y;
    // let hitable = Box::new(Rectangle::new(light, width_axis, light, height_axis));
    let hitable = Box::new(Cube::new(light, light, 0.125 * light));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, width / 4.0, height / 2.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(PlainMaterial::<f64>::new(texture));
    let actor = Actor::<f64> { hitable, material};
    actors.push(actor);

    // Rectangle used as floor
    let width_axis = Axis::X;
    let height_axis = Axis::Y;
    let hitable = Box::new(Rectangle::new(length, width_axis, width, height_axis));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.0, -height / 2.0])));
    let texture0 = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let texture1 = Box::new(UniformTexture::new(Vec3::from_array([0.8, 0.8, 0.8])));
    let mut texture = Box::new(CheckerTexture::new(texture0, texture1));
    texture.set_period(Vec3::from_array([length / 8.0, length / 8.0, 1.0]));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, dimming));
    let actor = Actor::<f64> { hitable, material};
    actors.push(actor);

    // Rectangle used as front wall
    let width_axis = Axis::X;
    let height_axis = Axis::Z;
    let rectangle = Box::new(Rectangle::new(length, width_axis, height, height_axis));
    let rectangle = Box::new(Translation::new(rectangle, Vec3::from_array([0.0, width / 2.0, 0.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, dimming));
    let actor = Actor::<f64> { hitable: rectangle, material};
    actors.push(actor);

    // Rectangle used as back wall
    let width_axis = Axis::Z;
    let height_axis = Axis::X;
    let rectangle = Box::new(Rectangle::new(height, width_axis, length, height_axis));
    let rectangle = Box::new(Translation::new(rectangle, Vec3::from_array([0.0, - width / 2.0, 0.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, dimming));
    let _actor = Actor::<f64> { hitable: rectangle, material};
    // actors.push(actor);

    // Rectangle used as left wall
    let width_axis = Axis::Y;
    let height_axis = Axis::Z;
    let rectangle = Box::new(Rectangle::new(width, width_axis, height, height_axis));
    let rectangle = Box::new(Translation::new(rectangle, Vec3::from_array([-length / 2.0, 0.0, 0.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([0.1, 1.0, 0.1])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, dimming));
    let actor = Actor::<f64> { hitable: rectangle, material};
    actors.push(actor);

    // Rectangle used as right wall
    let width_axis = Axis::Z;
    let height_axis = Axis::Y;
    let rectangle = Box::new(Rectangle::new(height, width_axis, width, height_axis));
    let rectangle = Box::new(Translation::new(rectangle, Vec3::from_array([length / 2.0, 0.0, 0.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 0.1, 0.1])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, dimming));
    let actor = Actor::<f64> { hitable: rectangle, material};
    actors.push(actor);

    // Rectangle used as ceiling
    let width_axis = Axis::Y;
    let height_axis = Axis::X;
    let rectangle = Box::new(Rectangle::new(width, width_axis, length, height_axis));
    let rectangle = Box::new(Translation::new(rectangle, Vec3::from_array([0.0, 0.0, height / 2.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, dimming));
    let actor = Actor::<f64> { hitable: rectangle, material};
    actors.push(actor);

    actors
}

fn create_cube_box(length: f64, width: f64, height: f64, thickness: f64) -> Box<Group<f64>> {
    let mut group : Box<Group<f64>> = Box::new(Group::<f64>::new());

    // cube used as floor
    let hitable = Box::new(Cube::new(length, width, thickness));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.0, -height / 2.0])));
    group.add_hitable(hitable);

    // cube used as ceiling
    let hitable = Box::new(Cube::new(length, width, thickness));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.0, height / 2.0])));
    group.add_hitable(hitable);

    // cube used as left wall
    let hitable = Box::new(Cube::new(thickness, width, height));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([- length / 2.0, 0.0, 0.0])));
    group.add_hitable(hitable);

    // cube used as right wall
    let hitable = Box::new(Cube::new(thickness, width, height));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([length / 2.0, 0.0, 0.0])));
    group.add_hitable(hitable);

    // cube used as back wall
    let hitable = Box::new(Cube::new(length, thickness, height));
    let _hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, width / 2.0, 0.0])));
    // group.add_hitable(hitable);

    group
}

#[test]
fn rectangle_room() {
    let room_length = 16.0;
    let room_width = 16.0;
    let room_height = 9.0;
    let mut actors = create_rectangle_room(room_length, room_width, room_height, 6.5);

    let mut scene = Scene::<f64>::new();
    // scene.set_background(Vec3::from_array([0.1, 0.1, 0.1]));

    loop {
        let actor = actors.pop();
        match actor {
            Some(actor) => {
                scene.add_actor(actor);
            },
            None => {
                break;
            }
        }
    };

    let box_size = 4.0;
    let box_thickness = 0.05 * box_size;
    let hitable = create_cube_box(box_size, box_size, box_size, box_thickness);
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([- 0.3 * room_length, 0.3 * room_width, - 0.5 * room_height + 0.5 * box_size])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([0.2, 0.2, 1.0])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, 1.0));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    // cube used as front glass wall
    let hitable = Box::new(Cube::new(box_size, box_thickness, box_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, - box_size / 2.0, 0.0])));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([- 0.3 * room_length, 0.3 * room_width, - 0.5 * room_height + 0.5 * box_size])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(DielectricMaterial::<f64>::new(texture, 1.6));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    // cube used as back glass wall
    let hitable = Box::new(Cube::new(box_size, box_thickness, box_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, box_size / 2.0, 0.0])));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([- 0.3 * room_length, 0.3 * room_width, - 0.5 * room_height + 0.5 * box_size])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(DielectricMaterial::<f64>::new(texture, 1.6));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    let sphere_size = 1.0;
    let hitable = Box::new(Sphere::new(sphere_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([- 0.3 * room_length, 0.3 * room_width, - 0.5 * room_height + 0.5 * box_size])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 0.2, 0.2])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, 1.0));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    // Large glass sphere in the front
    let sphere_size = 3.0;
    let hitable = Box::new(Sphere::new(sphere_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.1 * room_width, - 0.5 * room_height + sphere_size])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(DielectricMaterial::<f64>::new(texture, 2.4));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    // Large metal sphere in the front;
    let sphere_size = 2.0;
    let hitable = Box::new(Sphere::new(sphere_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.3 * room_length, 0.3 * room_width, - 0.5 * room_height + sphere_size])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([0.9, 0.9, 0.9])));
    let material = Box::new(MetalMaterial::<f64>::new(texture, 0.0));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    let mul = 4;
    let width = 12 * mul;
    let height = 8 * mul;
    let aspect = width as f64 / height as f64;
    let mut camera = PerspectiveCamera::<f64>::new();
    camera.set_aspect(aspect);
    camera.set_fov(0.37 * std::f64::consts::PI);
    camera.set_position(&[0.0, - 0.49 * room_width, 0.0]);
    camera.set_direction(&[0.0, 1.0, 0.0]);
    // camera.set_lookat(&[0.0, 0.0, 0.0]);
    camera.set_up(&[0.0, 0.0, 1.0]);
    camera.set_fov(0.3 * std::f64::consts::PI);
    camera.set_focus(1.0);

    scene.set_tree_type(TreeType::Oct);

    let renderer = Renderer::new(0, width, 0, height, width, height, 0, 0, false);
    let image = renderer.render(&mut scene, &camera);
    let gamma = 2.0;
    print_ppm(&image, gamma, "rectangle_room_preview.ppm");

    let gamma = 2.6;
    let renderer = Renderer::new(0, width, 0, height, width, height, 1, 32, false);
    let sampling = 1024;
    let mut image = Image::new(width, height);
    for i in 0..sampling {
        let delta = renderer.render(&scene, &camera);
        mix_images(&mut image, &delta, i);
        print_ppm(&image, gamma, "rectangle_room.ppm");
    }
}

#[test]
fn cube_scene() {
    let mut scene = Scene::<f64>::new();
    scene.set_background(Vec3::from_array([0.2, 0.2, 0.7]));

    let room_size = 15.0;
    let light_size = 2.0 * room_size / 3.0;

    // Rectangle used as floor
    let width_axis = Axis::X;
    let height_axis = Axis::Y;
    let hitable = Box::new(Rectangle::new(room_size, width_axis, room_size, height_axis));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.0, -room_size / 2.0])));
    let texture0 = Box::new(UniformTexture::new(Vec3::from_array([0.9, 0.9, 0.9])));
    let texture1 = Box::new(UniformTexture::new(Vec3::from_array([0.75, 0.75, 0.75])));
    let texture = Box::new(CheckerTexture::new(texture0, texture1));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, 0.65));
    let actor = Actor::<f64> { hitable, material};
    scene.add_actor(actor);

    // Box on the floor
    let length = 6.0;
    let width = 3.0;
    let heigth = 5.0;
    let hitable = Box::new(Cube::new(length, width, heigth));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([4.0, room_size / 3.0, -room_size / 2.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([0.0, 1.0, 0.0])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, 0.65));
    let actor = Actor::<f64> { hitable, material};
    scene.add_actor(actor);

    // Rectangle used as light
    let width_axis = Axis::X;
    let height_axis = Axis::Y;
    let hitable = Box::new(Rectangle::new(light_size, width_axis, light_size, height_axis));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.0, room_size / 2.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([2.0, 2.0, 2.0])));
    let material = Box::new(PlainMaterial::<f64>::new(texture));
    let actor = Actor::<f64> { hitable, material};
    scene.add_actor(actor);

    let mul = 4;
    let width = 12 * mul;
    let height = 8 * mul;
    let aspect = width as f64 / height as f64;
    let mut camera = PerspectiveCamera::<f64>::new();
    camera.set_aspect(aspect);
    camera.set_fov(0.35 * std::f64::consts::PI);
    camera.set_position(&[0.0, - 0.5 * room_size, 0.0]);
    camera.set_direction(&[0.0, 1.0, 0.0]);
    // camera.set_lookat(&[0.0, 0.0, 0.0]);
    camera.set_up(&[0.0, 0.0, 1.0]);
    camera.set_fov(0.4 * std::f64::consts::PI);
    camera.set_focus(1.0);

    scene.set_tree_type(TreeType::Oct);

    let renderer = Renderer::new(0, width, 0, height, width, height, 0, 0, false);
    let image = renderer.render(&mut scene, &camera);
    let gamma = 2.0;
    print_ppm(&image, gamma, "cube_scene_preview.ppm");

    let renderer = Renderer::new(0, width, 0, height, width, height, 32, 8, false);
    let image = renderer.render(&mut scene, &camera);
    print_ppm(&image, gamma, "cube_scene.ppm");
}

#[test]
fn basic_scene() {
    let mut scene = Scene::<f64>::new();
    scene.set_background(Vec3::from_array([0.2, 0.2, 0.7]));
    scene.set_background(Vec3::from_array([0.75, 0.75, 0.75]));

    let r = 1.0;
    let sphere = Box::new(Sphere::<f64>::new(r));
    let sphere = Translation::new(sphere, Vec3::from_array([0.0, r, -4.0]));
    let texture = UniformTexture::new(Vec3::from_array([1.0, 0.2, 0.2]));
    let material = LambertianMaterial::<f64>::new(Box::new(texture), 0.5);
    let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
    scene.add_actor(actor);
}

#[test]
fn sphere_in_box() {
    let mut scene = Scene::<f64>::new();
    scene.set_background(Vec3::from_array([0.2, 0.2, 0.8]));

    let box_size = 5.0;
    let box_thickness = 0.05 * box_size;
    let hitable = create_cube_box(box_size, box_size, box_size, box_thickness);
    let texture = Box::new(UniformTexture::new(Vec3::from_array([0.9, 0.9, 0.9])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, 0.75));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    // cube used as front glass wall
    let hitable = Box::new(Cube::new(box_size, box_thickness, box_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, - box_size / 2.0, 0.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 1.0, 1.0])));
    let material = Box::new(DielectricMaterial::<f64>::new(texture, 1.5));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    let sphere_size = 1.0;
    let hitable = Box::new(Sphere::new(sphere_size));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([1.0, 0.2, 0.2])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, 0.65));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    // Light
    let sphere_size = 3.0;
    let hitable = Box::new(Sphere::new(sphere_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, - 2.5 * box_size + sphere_size + 0.1, 0.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([2.0, 2.0, 2.0])));
    let material = Box::new(PlainMaterial::<f64>::new(texture));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    // Light
    let sphere_size = 3.0;
    let hitable = Box::new(Sphere::new(sphere_size));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([- 2.5 * box_size + sphere_size + 0.1, 0.0, 0.0])));
    let texture = Box::new(UniformTexture::new(Vec3::from_array([2.0, 2.0, 2.0])));
    let material = Box::new(PlainMaterial::<f64>::new(texture));
    let actor = Actor {hitable, material};
    scene.add_actor(actor);

    let mul = 4;
    let width = 12 * mul;
    let height = 8 * mul;
    let aspect = width as f64 / height as f64;
    let mut camera = PerspectiveCamera::<f64>::new();
    camera.set_aspect(aspect);
    camera.set_fov(0.35 * std::f64::consts::PI);
    camera.set_position(&[-4.0, - 1.5 * box_size, 0.0]);
    // camera.set_direction(&[0.0, 1.0, 0.0]);
    camera.set_lookat(&[0.0, 0.0, 0.0]);
    camera.set_up(&[0.0, 0.0, 1.0]);
    camera.set_fov(0.4 * std::f64::consts::PI);
    camera.set_focus(1.0);

    scene.set_tree_type(TreeType::Oct);

    let renderer = Renderer::new(0, width, 0, height, width, height, 0, 0, false);
    let image = renderer.render(&mut scene, &camera);
    let gamma = 2.0;
    print_ppm(&image, gamma, "sphere_in_box_preview.ppm");

    let renderer = Renderer::new(0, width, 0, height, width, height, 1, 8, false);
    let sampling = 128;
    let mut image = Image::new(width, height);
    for i in 0..sampling {
        let delta = renderer.render(&scene, &camera);
        mix_images(&mut image, &delta, i);
        print_ppm(&image, gamma, "sphere_in_box.ppm");
    }
}

#[test]
fn random_scene() {
    let mut scene = Scene::<f64>::new();
    // scene.set_background(Vec3::from_array([0.2, 0.2, 0.7]));
    scene.set_background(Vec3::from_array([0.5, 0.7, 0.9]));

    const N_SPHERES_X : usize = 20;
    const N_SPHERES_Y : usize = N_SPHERES_X;

    const MIN_X : f64 = -20.0;
    const MAX_X : f64 = 20.0;

    const MIN_Y : f64 = MIN_X;
    const MAX_Y : f64 = MAX_X;

    const MIN_RADIUS : f64 = 0.2;
    const MAX_RADIUS : f64 = 0.4;

    const SPHERE_PROBABILITY : f64 = 0.66666666;

    const LAMBERTIAN_PROBABILITY : f64 = 0.3333;
    const METAL_PROBABILITY : f64 = 0.3333;
    // DIELECTRIC_PROBABILITY is 1 - LAMBERTIAN_PROBABILITY - METAL_PROBABILITY

    const MIN_FUZZINESS : f64 = 0.0;
    const MAX_FUZZINESS : f64 = 0.4;

    const MIN_REFRACTIVE : f64 = 1.2;
    const MAX_REFRACTIVE : f64 = 2.4;

    let mut rng = rand::thread_rng();

    for i in 0..N_SPHERES_X {
        for j in 0..N_SPHERES_Y {
            let radius = MIN_RADIUS + (MAX_RADIUS - MIN_RADIUS) * rng.gen::<f64>();
            let mut x = i as f64 + rng.gen::<f64>() * (1.0 - radius);
            x = MIN_X + (MAX_X - MIN_X) * x / N_SPHERES_X as f64;
            let mut y = j as f64 + rng.gen::<f64>() * (1.0 - radius);
            y = MIN_Y + (MAX_Y - MIN_Y) * y / N_SPHERES_Y as f64;

            let hitable_select = rng.gen::<f64>();
            let hitable : Box<dyn Hitable<f64>> = if hitable_select < SPHERE_PROBABILITY {
                let hitable = Box::new(Sphere::<f64>::new(radius));
                Box::new(Translation::new(hitable, Vec3::from_array([x, y, radius])))
            } else {
                let l = radius * 2.0 * 0.8;
                let hitable = Box::new(Cube::<f64>::new(l, l, l));
                Box::new(Translation::new(hitable, Vec3::from_array([x, y, radius * 0.8])))
            };

            let color = Vec3::from_array([rng.gen::<f64>(), rng.gen::<f64>(), rng.gen::<f64>()]);
            let texture = Box::new(UniformTexture::new(color));
            let material_select = rng.gen::<f64>();
            let material : Box<dyn Material<f64>> = if material_select < LAMBERTIAN_PROBABILITY {
                Box::new(LambertianMaterial::<f64>::new(texture, 0.5))
            } else if material_select < LAMBERTIAN_PROBABILITY + METAL_PROBABILITY {
                let fuzziness = MIN_FUZZINESS + (MAX_FUZZINESS - MIN_FUZZINESS) * rng.gen::<f64>();
                Box::new(MetalMaterial::<f64>::new(texture, fuzziness))
            } else {
                let n = MIN_REFRACTIVE + (MAX_REFRACTIVE - MIN_REFRACTIVE) * rng.gen::<f64>();
                Box::new(DielectricMaterial::<f64>::new(texture, n))
            };
            let actor = Actor::<f64> { hitable, material};
            scene.add_actor(actor);
        }
    }

    // Three larger spheres in the center
    let radius = 2.0;
    let sphere = Box::new(Sphere::<f64>::new(radius));
    let sphere = Translation::new(sphere, Vec3::from_array([0.0, 0.0, radius]));
    let color = Vec3::from_array([0.78, 1.0, 0.78]);
    let texture = Box::new(UniformTexture::new(color));
    let material = DielectricMaterial::<f64>::new(texture, 2.4);
    let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
    scene.add_actor(actor);

    let sphere = Box::new(Sphere::<f64>::new(radius));
    let sphere = Translation::new(sphere, Vec3::from_array([0.0, - 2.0 * radius, radius]));
    let color = Vec3::from_array([0.9, 0.9, 0.9]);
    let texture = Box::new(UniformTexture::new(color));
    let material = MetalMaterial::<f64>::new(texture, 0.0);
    let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
    scene.add_actor(actor);

    let sphere = Box::new(Sphere::<f64>::new(radius));
    let sphere = Translation::new(sphere, Vec3::from_array([0.0, 2.0 * radius, radius]));
    let color = Vec3::from_array([1.0, 0.15, 0.15]);
    let texture = Box::new(UniformTexture::new(color));
    let material = MetalMaterial::<f64>::new(texture, 0.1);
    let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
    scene.add_actor(actor);

    // Sphere used as light
    let radius = 4.0;
    let sphere = Box::new(Sphere::<f64>::new(radius));
    let sphere = Translation::new(sphere, Vec3::from_array([0.0, 1.0, 12.5]));
    let color = Vec3::from_array([1.0, 1.0, 1.0]);
    let texture = Box::new(UniformTexture::new(color));
    let material = PlainMaterial::<f64>::new(texture);
    let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
    scene.add_actor(actor);

    // Rectangle used as floor
    let length = 2000.0;
    let color0 = Vec3::from_array([1.0, 1.0, 1.0]);
    let color1 = Vec3::from_array([0.8, 0.8, 0.8]);
    let texture0 = UniformTexture::new(color0);
    let texture1 = UniformTexture::new(color1);
    let texture = Box::new(CheckerTexture::new(Box::new(texture0), Box::new(texture1)));

    let hitable = Box::new(Rectangle::<f64>::new(length, Axis::X, length, Axis::Y));
    let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.0, -radius])));
    let material = Box::new(LambertianMaterial::<f64>::new(texture, 0.75));
    let actor = Actor::<f64> { hitable, material };
    scene.add_actor(actor);

    let mul = 4;
    let width = 16 * mul;
    let height = 9 * mul;
    let aspect = width as f64 / height as f64;
    let mut camera = PerspectiveCamera::<f64>::new();
    camera.set_aspect(aspect);
    camera.set_fov(0.25 * std::f64::consts::PI);
    camera.set_position(&[-6.0, -10.0, 3.0]);
    camera.set_lookat(&[0.0, 0.0, 2.0]);
    camera.set_up(&[0.0, 0.0, 1.0]);

    // camera.set_position(&[0.0, 0.0, 20.0]);
    // camera.set_lookat(&[0.0, 0.0, 0.0]);
    // camera.set_up(&[0.0, 1.0, 0.0]);

    camera.set_aperture(0.0);
    let focus = (camera.get_lookat() - camera.get_position()).norm();
    camera.set_focus(focus);

    scene.set_tree_type(TreeType::Oct);

    let renderer = Renderer::new(0, width/4, 0, height/4, width/4, height/4, 0, 2, false);
    let image = renderer.render(&mut scene, &camera);
    let gamma = 2.0;
    print_ppm(&image, gamma, "random_scene_preview.ppm");

    let mut image = Image::new(width, height);
    let renderer = Renderer::new(0, width, 0, height, width, height, 1, 16, false);
    let sampling = 1024;
    for i in 0..sampling {
        let delta = renderer.render(&scene, &camera);
        mix_images(&mut image, &delta, i);
        print_ppm(&image, gamma, "random_scene.ppm");
    }
}

#[test]
fn tree() {
    let mut scene = Scene::<f64>::new();
    scene.set_background(Vec3::from_array([0.6, 0.8, 1.0]));

    const N_SPHERES_X : usize = 10;
    const N_SPHERES_Y : usize = N_SPHERES_X;
    const N_SPHERES_Z : usize = N_SPHERES_X;

    const MIN_X : f64 = -20.0;
    const MAX_X : f64 = 20.0;

    const MIN_Y : f64 = MIN_X;
    const MAX_Y : f64 = MAX_X;

    const MIN_Z : f64 = MIN_X;
    const MAX_Z : f64 = MAX_X;

    const MIN_RADIUS : f64 = 0.2;
    const MAX_RADIUS : f64 = 1.0;

    let mut rng = rand::thread_rng();

    for i in 0..N_SPHERES_X {
        for j in 0..N_SPHERES_Y {
            for k in 0..N_SPHERES_Z {
                let radius = MIN_RADIUS + (MAX_RADIUS - MIN_RADIUS) * rng.gen::<f64>();
                let mut x = i as f64 + rng.gen::<f64>() * (1.0 - radius);
                x = MIN_X + (MAX_X - MIN_X) * x / N_SPHERES_X as f64;
                let mut y = j as f64 + rng.gen::<f64>() * (1.0 - radius);
                y = MIN_Y + (MAX_Y - MIN_Y) * y / N_SPHERES_Y as f64;
                let mut z = k as f64 + rng.gen::<f64>() * (1.0 - radius);
                z = MIN_Z + (MAX_Z - MIN_Z) * z / N_SPHERES_Z as f64;

                let sphere = Box::new(Sphere::<f64>::new(radius));
                let sphere = Translation::new(sphere, Vec3::from_array([x, y, z]));

                let color = Vec3::from_array([rng.gen::<f64>(), rng.gen::<f64>(), rng.gen::<f64>()]);
                let texture = Box::new(UniformTexture::new(color));
                let material : Box<dyn Material<f64>> = Box::new(MetalMaterial::new(texture, 0.0));

                let actor = Actor::<f64> { hitable: Box::new(sphere), material};
                scene.add_actor(actor);
            }
        }
    }

    let mul = 4;
    let width = 16 * mul;
    let height = 9 * mul;
    let aspect = width as f64 / height as f64;
    let mut camera = PerspectiveCamera::<f64>::new();
    camera.set_aspect(aspect);
    camera.set_fov(0.3 * std::f64::consts::PI);
    camera.set_position(&[-6.0, -10.0, 3.0]);
    camera.set_lookat(&[0.0, 0.0, 2.0]);
    camera.set_up(&[0.0, 0.0, 1.0]);

    camera.set_aperture(0.0);
    let focus = (camera.get_lookat() - camera.get_position()).norm();
    camera.set_focus(focus);

    let renderer = Renderer::new(0, width, 0, height, width, height, 0, 0, false);

    scene.set_tree_type(TreeType::Linear);
    let now = Instant::now();
    let image_linear = renderer.render(&scene, &camera);
    let _t_linear = now.elapsed().as_millis();
    // println!("Linear: {}", t_linear);

    scene.set_tree_type(TreeType::Binary);
    let now = Instant::now();
    let image_binary = renderer.render(&mut scene, &camera);
    let _t_binary = now.elapsed().as_millis();
    let diff = image_diff(&image_linear, &image_binary);
    //assert!(t_binary < t_linear);
    assert_eq!(diff, 0.0);
    // println!("Binary -  t: {}  diff: {}", t_binary, diff);

    scene.set_tree_type(TreeType::Oct);
    let now = Instant::now();
    let image_oct = renderer.render(&scene, &camera);
    let _t_oct = now.elapsed().as_millis();
    let diff = image_diff(&image_linear, &image_oct);
    //assert!(t_oct < t_linear);
    assert_eq!(diff, 0.0);
    // println!("Oct -  t: {}  diff: {}", t_oct, diff);
}
