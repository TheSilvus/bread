use num::complex::Complex32;
use std::time::Duration;

#[derive(Clone)]
pub struct BufferConfig {
    pub width: usize,
    pub height: usize,

    pub min: Complex32,
    pub max: Complex32,

    pub min_iterations: u32,
    pub max_iterations: u32,
}

#[derive(Clone)]
pub struct Config {
    pub thread_count: usize,
    pub duration: Duration,
    pub cycles : u64,
    pub keep: bool,

    pub buffers: Vec<BufferConfig>,
}



pub fn get_config() -> Config {
    let width = 10000;
    let height = 10000;

    //let center = Complex32::new(-0.158, 1.033);
    //let size = 0.03;
    let center = Complex32::new(0.0, 0.0);
    let size = 4.0;

    let min = center - Complex32::new(size / 2., size / 2.);
    let max = center + Complex32::new(size / 2., size / 2.);

    Config {
        thread_count: 6,
        duration: Duration::from_secs(30),
        //cycles: 1,
        cycles: std::u64::MAX,
        keep: true,

        buffers: vec![
            BufferConfig {
                width,
                height,
                min,
                max,
                min_iterations: 10,
                max_iterations: 50,
            },
            BufferConfig {
                width,
                height,
                min,
                max,
                min_iterations: 10,
                max_iterations: 400,
            },
            BufferConfig {
                width,
                height,
                min,
                max,
                min_iterations: 10,
                max_iterations: 2000,
            },
        ],
    }
}
