mod utils;

use std::fs::File;
use std::io::BufWriter;
use image::{DynamicImage, ExtendedColorType, GenericImageView, ImageBuffer, ImageEncoder, ImageReader, Pixel, Rgba};
use image::codecs::png::PngEncoder;
use itertools::Itertools;

type Coord = (u32, u32);

fn dark_channel(image: &DynamicImage, patch_size: u32) -> Vec<u8> {
    let mut dc = Vec::with_capacity((image.width() * image.height()) as usize);
    for (y, x) in itertools::iproduct!(0..image.height(), 0..image.width()) {
        let mut minimum = 255;

        for yp in 0..patch_size {
            if yp > y + (patch_size / 2) || ((y + (patch_size / 2)) - yp) >= image.height() {
                continue;
            }
            for xp in 0..patch_size {
                if xp > (x + patch_size / 2) || ((x + (patch_size / 2)) - xp) >= image.width() {
                    continue;
                }

                let p = image.get_pixel((x + (patch_size / 2)) - xp, (y + (patch_size / 2)) - yp);
                p.channels().iter().for_each(|c| minimum = minimum.min(*c));
            }
        }

        dc.push(minimum);
    }

    dc
}

fn transmission_map(mut dark_map: Vec<u8>, omega: f32) -> Vec<u8> {
    dark_map.iter_mut().for_each(|d| *d = 255 - (*d as f32 * omega) as u8);
    dark_map
}

fn get_atmospheric(dark_map: &[u8], image: &DynamicImage, a_proportion: f32) -> (u8, u8, u8) {
    let brightest = dark_map.iter().enumerate().sorted_by(|(_, d), (_, d2)| Ord::cmp(&d2, &d)).take((dark_map.len() as f32 * (a_proportion)) as usize).map(|(i, d)| i).collect_vec();

    let mut best_i = 0;
    let mut best_px = (255, 255, 255);
    for i in brightest {
        let x = i as u32 % image.width();
        let y = i as u32 / image.width();
        let px = image.get_pixel(x, y).0;
        let intensity = px[0].max(px[1]).max(px[2]);
        if intensity > best_i {
            best_i = intensity;
            best_px = (px[0], px[1], px[2])
        }
    }

    best_px
}

fn floatify(u: u8) -> f32 {
    u as f32 / 255.0
}

fn defloatify(f: f32) -> u8 {
    (f.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn reconstruct(image: &DynamicImage, atmospheric: &(u8, u8, u8), transmission_map: &[u8], t_0: f32) -> Vec<u8> {
    let atmospheric = (
        floatify(atmospheric.0),
        floatify(atmospheric.1),
        floatify(atmospheric.2),
    );
    let mut output = Vec::with_capacity(transmission_map.len() * 3);
    for (x, y, pixel) in image.pixels() {
        let pixel = (
            floatify(pixel.0[0]),
            floatify(pixel.0[1]),
            floatify(pixel.0[2]),
        );

        let numerator = (
            pixel.0 - atmospheric.0,
            pixel.1 - atmospheric.1,
            pixel.2 - atmospheric.2,
        );

        let t = floatify(transmission_map[(y * image.width() + x) as usize]).max(t_0);

        let j = (
            (numerator.0 / t) + atmospheric.0,
            (numerator.1 / t) + atmospheric.1,
            (numerator.2 / t) + atmospheric.2,
        );

        output.push(defloatify(j.0));
        output.push(defloatify(j.1));
        output.push(defloatify(j.2));
    }
    output
}

fn main() {
    const PATCH_SIZE: u32 = 5;
    const OMEGA: f32 = 0.95;
    const T_0: f32 = 0.1;
    const A_PROPORTION: f32 = 0.002;

    print!("Loading image... ");
    time!(
        let image = ImageReader::open("image.jpg").unwrap().decode().unwrap();
    );

    print!("Calculating dark channel... ");
    time!(
        let dark_channel = dark_channel(&image, PATCH_SIZE);
    );

    print!("Calculating atmospheric... ");
    time!(
        let atmospheric = get_atmospheric(&dark_channel, &image, A_PROPORTION);
        // let atmospheric: (u8, u8, u8) = (213,214,213);
    );

    println!("Using atmospheric value: {:?}", atmospheric);

    print!("Calculating transmission map... ");
    time!(
        let t_map = transmission_map(dark_channel, OMEGA);
    );

    print!("Outputting transmission map image... ");
    time!(
        output_t_map(&t_map, &image);
    );

    print!("Reconstructing... ");
    time!(
        let reconstruct = reconstruct(&image, &atmospheric, &t_map, T_0);
    );

    print!("Outputting reconstruction... ");
    time!(
        output_reconstruct(&reconstruct, &image);
    );
}

fn output_t_map(t_map: &[u8], image: &DynamicImage) {
    let mut t_map_output = Vec::with_capacity(t_map.len() * 3);
    t_map.iter().for_each(|c| {
        t_map_output.push(*c);
        t_map_output.push(*c);
        t_map_output.push(*c);
    });

    let output_path = "transmission_map.png";
    let file = File::create(output_path).expect("File create failed");
    let ref mut buf_writer = BufWriter::new(file);

    let encoder = PngEncoder::new(buf_writer);
    // print!("W: {}, H: {}, WH: {}, WH3: {}, LEN: {}", image.width(), image.height(), image.width() * image.height(), image.width() * image.height() * 3, t_map_output.len());
    encoder.write_image(&t_map_output, image.width(), image.height(), ExtendedColorType::Rgb8).unwrap();
}

fn output_reconstruct(reconstruct: &[u8], image: &DynamicImage) {
    let output_path = "output.png";
    let file = File::create(output_path).expect("File create failed");
    let ref mut buf_writer = BufWriter::new(file);

    let encoder = PngEncoder::new(buf_writer);
    // print!("W: {}, H: {}, WH: {}, WH3: {}, LEN: {}", image.width(), image.height(), image.width() * image.height(), image.width() * image.height() * 3, t_map_output.len());
    encoder.write_image(reconstruct, image.width(), image.height(), ExtendedColorType::Rgb8).unwrap();
}