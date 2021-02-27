use crossbeam::thread;
use image::ColorType;
use num::complex::Complex32;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::ops::Add;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

static TIMER_CHECK_MS: Duration = Duration::from_millis(50);

fn main() {
    let width = 1000;
    let height = 1000;

    //let center = Complex32::new(-0.158, 1.033);
    //let size = 0.03;
    let center = Complex32::new(0.0, 0.0);
    let size = 4.0;

    let min = center - Complex32::new(size / 2., size / 2.);
    let max = center + Complex32::new(size / 2., size / 2.);

    let mut brot = Brot {
        thread_count: 8,
        duration: Duration::from_secs(10),
        keep: false,

        buffers: vec![
            BrotBuffer {
                width,
                height,
                min,
                max,
                min_iterations: 10,
                max_iterations: 50,
            },
            BrotBuffer {
                width,
                height,
                min,
                max,
                min_iterations: 10,
                max_iterations: 400,
            },
            BrotBuffer {
                width,
                height,
                min,
                max,
                min_iterations: 10,
                max_iterations: 2000,
            },
        ],
        results: None,
    };
    
    println!("Running");
    brot.run();
    brot.print_stats();
    println!("Stats");
    println!("Storing");
    brot.store();

    let buffers = brot.results.unwrap();

    println!("Generating images");
    image::save_buffer(
        "image.png",
        &Buffer::join(
            buffers[2].buffer.to_u8(),
            buffers[1].buffer.to_u8(),
            buffers[0].buffer.to_u8(),
        )
        .flatten(),
        width as u32,
        height as u32,
        ColorType::Rgb8,
    )
    .expect("Couldn't store image");
}

#[derive(Clone)]
struct BrotBuffer {
    width: usize,
    height: usize,

    min: Complex32,
    max: Complex32,

    min_iterations: u32,
    max_iterations: u32,
}

#[derive(Clone)]
struct Brot {
    thread_count: usize,
    duration: Duration,
    keep: bool,

    buffers: Vec<BrotBuffer>,

    results: Option<Vec<HitBuffer>>,
}
impl Brot {
    fn run(&mut self) {
        let iterations = self
            .buffers
            .iter()
            .map(|b| b.max_iterations)
            .max()
            .expect("No buffer");

        self.results = Some(
            thread::scope(|scope| {
                let mut threads = vec![];

                for _ in 0..self.thread_count {
                    threads.push(scope.spawn(|_| {
                        let mut rng = SmallRng::from_entropy();

                        let mut buffers = self
                            .buffers
                            .iter()
                            .map(|b| HitBuffer::new(b.width, b.height, b.min, b.max))
                            .collect::<Vec<_>>();
                        let mut timer = Timer::new(Instant::now(), self.duration, TIMER_CHECK_MS);
                        while !timer.check() {
                            let c =
                                Complex32::new(rng.gen_range(-2.0..2.0), rng.gen_range(-2.0..2.0));
                            let z_initial = Complex32::new(0.0, 0.0);

                            if Brot::approximate_is_in_mandelbrot(c) {
                                continue;
                            }

                            if let Some(i) =
                                Self::iterate_step(z_initial, c, 2.0, iterations, |_, _| ())
                            {
                                for (b_index, b) in self.buffers.iter().enumerate() {
                                    if b.min_iterations <= i && i < b.max_iterations {
                                        Self::iterate_step(
                                            z_initial,
                                            c,
                                            2.0,
                                            iterations,
                                            |i, z| {
                                                if b.min_iterations <= i && i < b.max_iterations {
                                                    buffers[b_index].hit(z);
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        }

                        buffers
                    }));
                }
                let mut buffers = self
                    .buffers
                    .iter()
                    .map(|b| HitBuffer::new(b.width, b.height, b.min, b.max))
                    .collect::<Vec<_>>();
                for thread in threads {
                    for (i, buffer) in thread.join().expect("Thread panicked").iter().enumerate() {
                        buffers[i].add(buffer);
                    }
                }

                buffers
            })
            .expect("Error while executing threads"),
        );
    }

    fn approximate_is_in_mandelbrot(c: Complex32) -> bool {
        let q = (c.re - 0.25) * (c.re - 0.25) + c.im * c.im;
        q * (q + c.re - 0.25) <= 0.25 * c.im * c.im
    }

    fn step(z: Complex32, c: Complex32) -> Complex32 {
        z * z + c
    }

    fn iterate_step(
        mut z: Complex32,
        c: Complex32,
        bailout: f32,
        iterations: u32,
        mut f: impl FnMut(u32, Complex32),
    ) -> Option<u32> {
        for i in 0..iterations {
            z = Self::step(z, c);
            if z.norm_sqr() > bailout * bailout {
                return Some(i);
            }
            f(i, z);
        }
        None
    }

    fn print_stats(&self) {
        for (i, buffer) in self.results.as_ref().unwrap().iter().enumerate() {
            let samples = buffer.buffer.buffer.iter().map(|i| *i as u64).sum::<u64>();
            println!("Buffer {} with {} samples, {:.2} samples/s, {:.2} samples/pixel, {:.4} samples/pixel/s",
                    i,
                    samples,
                    samples as f64 / self.duration.as_secs_f64(),
                    samples as f64 / (buffer.buffer.width * buffer.buffer.height) as f64,
                    samples as f64 / (buffer.buffer.width * buffer.buffer.height) as f64 / self.duration.as_secs_f64());
        }
    }

    fn store(&mut self) {
        for (i, buffer) in self.results.as_mut().unwrap().iter_mut().enumerate() {
            let path = format!("buffer-{}.bread", i);

            if Path::new(&path).exists() && self.keep {
                let old_buffer = Buffer::load(buffer.buffer.width, buffer.buffer.height, &path)
                    .expect("Could not load old buffer");

                buffer.buffer.add(&old_buffer);
            }
            buffer.buffer.store(&path).expect("Couldn't store buffer");
        }
    }
}

struct Timer {
    start: Instant,
    total: Duration,

    last_check: Instant,
    check_difference: Duration,
    frequency: u32,
    current: u32,
}
impl Timer {
    fn new(start: Instant, total: Duration, check_difference: Duration) -> Timer {
        Timer {
            start,
            total,
            last_check: Instant::now(),
            check_difference,
            frequency: 1,
            current: 0,
        }
    }

    fn check(&mut self) -> bool {
        self.current += 1;
        if self.current > self.frequency {
            self.current = 0;
            if self.last_check.elapsed() > self.check_difference {
                self.frequency = 1.max(self.frequency / 2);
            } else {
                self.frequency *= 2;
            }
            self.last_check = Instant::now();

            self.start.elapsed() > self.total
        } else {
            false
        }
    }
}

struct Buffer<T> {
    width: usize,
    height: usize,
    buffer: Vec<T>,
}
impl<T: Default + Clone> Buffer<T> {
    fn new_default(width: usize, height: usize) -> Buffer<T> {
        Buffer {
            width,
            height,
            buffer: vec![T::default(); width * height],
        }
    }
}
impl<T> Buffer<T> {
    fn get(&self, x: usize, y: usize) -> &T {
        &self.buffer[y * self.width + x]
    }
    fn set(&mut self, x: usize, y: usize, t: T) {
        self.buffer[y * self.width + x] = t;
    }
}
impl<T: Clone> Clone for Buffer<T> {
    fn clone(&self) -> Buffer<T> {
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self.buffer.clone(),
        }
    }
}
impl<T: Copy + Add<T, Output = T>> Buffer<T> {
    fn add(&mut self, other: &Buffer<T>) {
        assert!(self.width == other.width);
        assert!(self.height == other.height);

        for i in 0..self.buffer.len() {
            self.buffer[i] = self.buffer[i] + other.buffer[i];
        }
    }
}
impl Buffer<u32> {
    fn to_u8(&self) -> Buffer<u8> {
        let max = self.buffer.iter().max().expect("");
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|x| (x * 255 / max) as u8)
                .collect::<Vec<_>>(),
        }
    }

    fn store(&self, file: &str) -> Result<(), io::Error> {
        let mut f = BufWriter::new(File::create(file)?);
        for i in &self.buffer {
            f.write_all(&i.to_le_bytes())?;
        }

        Ok(())
    }

    fn load(width: usize, height: usize, file: &str) -> Result<Buffer<u32>, io::Error> {
        let mut f = BufReader::new(File::open(file)?);
        let mut buffer = Vec::with_capacity(width * height);

        let mut temp = [0u8; 4];
        while let Ok(()) = f.read_exact(&mut temp) {
            buffer.push(u32::from_le_bytes(temp));
        }

        if buffer.len() != width * height {
            // TODO replace this with result
            panic!("Wrong file size");
        }

        Ok(Buffer {
            width,
            height,
            buffer,
        })
    }
}
impl Buffer<(u8, u8, u8)> {
    fn join(b1: Buffer<u8>, b2: Buffer<u8>, b3: Buffer<u8>) -> Buffer<(u8, u8, u8)> {
        // TODO ensure sizes work
        let mut result = Vec::new();
        for i in 0..b1.width * b1.height {
            result.push((b1.buffer[i], b2.buffer[i], b3.buffer[i]));
        }
        Buffer {
            width: b1.width,
            height: b1.height,
            buffer: result,
        }
    }

    fn flatten(&self) -> Vec<u8> {
        use std::iter::once;

        self.buffer
            .iter()
            .flat_map(|(u1, u2, u3)| once(*u1).chain(once(*u2)).chain(once(*u3)))
            .collect()
    }
}

#[derive(Clone)]
struct HitBuffer {
    buffer: Buffer<u32>,
    min: Complex32,
    max: Complex32,
}
impl HitBuffer {
    fn new(width: usize, height: usize, min: Complex32, max: Complex32) -> HitBuffer {
        HitBuffer {
            buffer: Buffer::new_default(width, height),
            min,
            max,
        }
    }

    fn hit(&mut self, c: Complex32) {
        let x = (c.re - self.min.re) / (self.max.re - self.min.re) * self.buffer.width as f32;
        let y = (c.im - self.min.im) / (self.max.im - self.min.im) * self.buffer.height as f32;

        if x < 0. || y < 0. || x >= self.buffer.width as f32 || y >= self.buffer.height as f32 {
            return;
        }

        self.buffer.set(
            x as usize,
            y as usize,
            self.buffer.get(x as usize, y as usize) + 1,
        );
    }
    fn add(&mut self, other: &HitBuffer) {
        // TODO this should either be in Buffer or check min/max
        for i in 0..self.buffer.buffer.len() {
            self.buffer.buffer[i] += other.buffer.buffer[i];
        }
    }
}
