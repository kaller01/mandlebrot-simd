use packed_simd::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::time::Instant;

type Fsimd = f32x8; // AVX2
type Msimd = m32x8;

struct ComplexArea {
    cmin_r: f32,
    cmax_r: f32,
    cmin_i: f32,
    cmax_i: f32,
}

fn mandlebrot_simd(area: &ComplexArea, width: u32, height: u32, image: &mut Vec<u8>) {
    const MAX_ITER: usize = 256;

    let cmin_r = area.cmin_r;
    let cmax_r = area.cmax_r;
    let cmin_i = area.cmin_i;
    let cmax_i = area.cmax_i;

    let scale_x = (cmax_r - cmin_r) / (width as f32);
    let scale_y = (cmax_i - cmin_i) / (height as f32);
    let _iter_scale = 255 / MAX_ITER;

    for y in 0..height {
        for x in (0..width).step_by(Fsimd::lanes()) {
            // Initate vx from x + 0..lanes (max 16)
            let vx: Fsimd = Fsimd::splat(x as f32)
                + unsafe {
                    Fsimd::from_slice_unaligned_unchecked(&[
                        0., 1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 13., 14., 15.,
                    ])
                };

            // Calculate coordinates based of vx and y
            let c_r = cmin_r + (vx * scale_x);
            let c_i = Fsimd::splat(cmax_i - y as f32 * scale_y);
            // println!("{:?}, {:?}", c_r, c_i);

            let mut z_r = c_r;
            let mut z_i = c_i;

            // Mask for (a,bi) that are unfinished
            let mut unfinished = Msimd::splat(true);
            let mut iter = Fsimd::splat(1.);

            for _ in 1..MAX_ITER {
                // Mandlebrot calculation thing;
                let next_z_r = z_r * z_r - z_i * z_i + c_r;
                let next_z_i = z_r * z_i + z_r * z_i + c_i;

                // Only update to those still unfinished looping
                z_r = unfinished.select(next_z_r, z_r);
                z_i = unfinished.select(next_z_i, z_i);

                // If abs value of Z less than 2, stop iteration
                let abs_square = z_r * z_r + z_i * z_i;
                // Update the unfinished mask
                unfinished = (abs_square).le(Fsimd::splat(4.0));
                iter = unfinished.select(iter + 1., iter);

                // All finished iteration, break loop
                if unfinished.none() {
                    break;
                }
            }

            // Calculate the index in the image for all lanes
            let index = vx + Fsimd::splat((y * width) as f32);
            // Transfer the lanes into array at once
            let mut indexs = [0.0; Fsimd::lanes()];
            unsafe { index.write_to_slice_unaligned_unchecked(&mut indexs) };
            // Calculate the color (iter for now 0..256)
            let color = iter;
            // 
            let mut colors = [0.0; Fsimd::lanes()];
            unsafe { color.write_to_slice_unaligned_unchecked(&mut colors) };

            for lane in 0..Fsimd::lanes() {
                image[indexs[lane] as usize] = colors[lane] as u8;
            }
        }
    }
}

fn mandlebrot(area: &ComplexArea, width: u32, height: u32, image: &mut Vec<u8>) {
    const MAX_ITER: usize = 256;
    let cmin_r = area.cmin_r;
    let cmax_r = area.cmax_r;
    let cmin_i = area.cmin_i;
    let cmax_i = area.cmax_i;
    let scale_x = (cmax_r - cmin_r) / (width as f32);
    let scale_y = (cmax_i - cmin_i) / (height as f32);
    let _iter_scale = 255 / MAX_ITER;
    for y in 0..height {
        for x in 0..width {
            let vx = x as f32;

            let c_r = cmin_r + (vx * scale_x);
            let c_i = cmax_i - y as f32 * scale_y;

            let mut z_r = c_r;
            let mut z_i = c_i;

            let mut iter = 0;

            for _ in 1..MAX_ITER {
                // Mandlebrot calculation thing;
                let next_z_r = z_r * z_r - z_i * z_i + c_r;
                let next_z_i = z_r * z_i + z_r * z_i + c_i;
                z_r = next_z_r;
                z_i = next_z_i;
                // If abs value of Z less than 2, stop iteration
                let abs_square = z_r * z_r + z_i * z_i;
                if abs_square > 4. {
                    break;
                }
                iter += 1;
            }
            let index = vx + (y * width) as f32;
            image[index as usize] = iter as u8;
        }
    }
}

fn write_image(path: &Path, image: &Vec<u8>, width: u32, height: u32) {
    let file = File::create(path).unwrap();
    let bufw = &mut BufWriter::new(file);

    let mut encoder = png::Encoder::new(bufw, width, height);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_color(png::ColorType::Grayscale);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&image).unwrap();
}

#[cfg(target_arch = "x86_64")]
fn main() {
    const WIDTH: u32 = 1024*4;
    const HEIGHT: u32 = 1024*4;
    let mut image: Vec<u8> = vec![0; (WIDTH * HEIGHT) as usize];

    let complex_area = ComplexArea {
        cmin_r: -2.0,
        cmax_r: 1.0,
        cmin_i: -1.5,
        cmax_i: 1.5,
    };

    if is_x86_feature_detected!("avx2") {
        println!("AVX2 is supported!");
        println!("Mandlebrot size of {}x{}", WIDTH, HEIGHT);

        let start_time = Instant::now();
        mandlebrot_simd(&complex_area, WIDTH, HEIGHT, &mut image);
        let end_time = Instant::now();
        let elapsed_time = end_time - start_time;
        println!("SIMD: Time taken (ms): {}", elapsed_time.as_millis());
        write_image(Path::new(r"mandlebrot_simd.png"), &image, WIDTH, HEIGHT);

        let start_time = Instant::now();
        mandlebrot(&complex_area, WIDTH, HEIGHT, &mut image);
        let end_time = Instant::now();
        let elapsed_time = end_time - start_time;
        println!("SEQ: Time taken (ms): {}", elapsed_time.as_millis());
        write_image(
            Path::new(r"mandlebrot_sequential.png"),
            &image,
            WIDTH,
            HEIGHT,
        );
    } else {
        println!("AVX2 is not supported on this platform.");
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn main() {
    println!("SIMD and AVX2 are not supported on this platform.");
}
