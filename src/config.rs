use num::complex::Complex32;
use std::time::Duration;

#[derive(Clone)]
pub struct BufferConfig {
    pub min_iterations: u32,
    pub max_iterations: u32,
}

#[derive(Clone)]
pub struct Config {
    pub thread_count: usize,
    pub duration: Duration,
    pub cycles: u64,
    pub keep: bool,

    pub width: usize,
    pub height: usize,

    pub min: Complex32,
    pub max: Complex32,

    pub buffers: Vec<BufferConfig>,
}

pub fn get_config() -> Config {
    let width = 1000;
    let height = 1000;

    let center = Complex32::new(-0.158, 1.033);
    let size = 0.03;
    //let center = Complex32::new(0.0, 0.0);
    //let size = 4.0;

    Config {
        thread_count: 6,
        duration: Duration::from_secs(10),
        cycles: 1,
        //cycles: std::u64::MAX,
        keep: false,

        width: 1000,
        height: 1000,

        min: center - Complex32::new(size / 2., size / 2.),
        max: center + Complex32::new(size / 2., size / 2.),

        buffers: vec![
            BufferConfig {
                min_iterations: 10,
                max_iterations: 50,
            },
            BufferConfig {
                min_iterations: 10,
                max_iterations: 400,
            },
            BufferConfig {
                min_iterations: 10,
                max_iterations: 2000,
            },
        ],
    }
}
