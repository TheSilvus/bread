use crossbeam::thread;
use num::complex::Complex32;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use bread::*;

static TIMER_CHECK_MS: Duration = Duration::from_millis(50);

fn main() {
    let mut brot = Brot {
        config: get_config(),
        results: None,
    };
    println!("Running");
    brot.run();
    brot.print_stats();
    println!("Storing");
    brot.store();
}

#[derive(Clone)]
struct Brot {
    config: Config,

    results: Option<Vec<HitBuffer>>,
}
impl Brot {
    fn run(&mut self) {
        let iterations = self
            .config
            .buffers
            .iter()
            .map(|b| b.max_iterations)
            .max()
            .expect("No buffer");

        self.results = Some(
            thread::scope(|scope| {
                let mut threads = vec![];

                for _ in 0..self.config.thread_count {
                    threads.push(scope.spawn(|_| {
                        let mut rng = SmallRng::from_entropy();

                        let mut buffers = self
                            .config
                            .buffers
                            .iter()
                            .map(|b| HitBuffer::new(b.width, b.height, b.min, b.max))
                            .collect::<Vec<_>>();
                        let mut timer =
                            Timer::new(Instant::now(), self.config.duration, TIMER_CHECK_MS);
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
                                for (b_index, b) in self.config.buffers.iter().enumerate() {
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
                    .config
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
            let samples = buffer.buffer.buffer().iter().map(|i| *i as u64).sum::<u64>();
            println!("Buffer {} with {} samples, {:.2} samples/s, {:.2} samples/pixel, {:.4} samples/pixel/s",
                    i,
                    samples,
                    samples as f64 / self.config.duration.as_secs_f64(),
                    samples as f64 / (buffer.buffer.width() * buffer.buffer.height()) as f64,
                    samples as f64 / (buffer.buffer.width() * buffer.buffer.height()) as f64 / self.config.duration.as_secs_f64());
        }
    }

    fn store(&mut self) {
        for (i, buffer) in self.results.as_mut().unwrap().iter_mut().enumerate() {
            let path = format!("buffer-{}.bread", i);

            if Path::new(&path).exists() && self.config.keep {
                let old_buffer = Buffer::load(buffer.buffer.width(), buffer.buffer.height(), &path)
                    .expect("Could not load old buffer");

                buffer.buffer.add(&old_buffer);
            }
            buffer.buffer.store(&path).expect("Couldn't store buffer");
        }
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
        let x = (c.re - self.min.re) / (self.max.re - self.min.re) * self.buffer.width() as f32;
        let y = (c.im - self.min.im) / (self.max.im - self.min.im) * self.buffer.height() as f32;

        if x < 0. || y < 0. || x >= self.buffer.width() as f32 || y >= self.buffer.height() as f32 {
            return;
        }

        self.buffer.set(
            x as usize,
            y as usize,
            self.buffer.get(x as usize, y as usize) + 1,
        );
    }
    fn add(&mut self, other: &HitBuffer) {
        self.buffer.add(&other.buffer);
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
