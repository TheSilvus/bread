use image::ColorType;

use bread::*;

fn main() {
    let buffers = get_config().buffers.iter().enumerate().map(|(i, b)| {
        Buffer::load(b.width, b.height, &format!("buffer-{}.bread", i)).expect("Could not load buffer")
    }).collect::<Vec<_>>();

    println!("Generating images");
    image::save_buffer(
        "image.png",
        &Buffer::join(
            buffers[2].to_u8(),
            buffers[1].to_u8(),
            buffers[0].to_u8(),
        )
        .flatten(),
        buffers[0].width() as u32,
        buffers[0].height() as u32,
        ColorType::Rgb8,
    )
    .expect("Couldn't store image");
}