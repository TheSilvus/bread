use image::ColorType;

use bread::*;

fn main() {
    let c = get_config();
    let buffers = (0..c.buffers.len())
        .map(|i| {
            Buffer::load(c.width, c.height, &format!("buffer-{}.bread", i))
                .expect("Could not load buffer")
        })
        .collect::<Vec<_>>();

    println!("Generating images");
    image::save_buffer(
        "image.png",
        &Buffer::join(
            buffers[2].to_f32().exponential(8.0).expose(0.9).to_u8(),
            buffers[1].to_f32().exponential(8.0).expose(0.9).to_u8(),
            buffers[0].to_f32().exponential(8.0).to_u8(),
        )
        .flatten(),
        buffers[0].width() as u32,
        buffers[0].height() as u32,
        ColorType::Rgb8,
    )
    .expect("Couldn't store image");

    for i in 0..c.buffers.len() {
        image::save_buffer(
            &format!("buffer-{}.png", i),
            &buffers[i].to_f32().exponential(3.0).expose(1.3).to_u8().buffer(),
            buffers[0].width() as u32,
            buffers[0].height() as u32,
            ColorType::L8,
        )
        .expect("Couldn't store image");
    }
}
