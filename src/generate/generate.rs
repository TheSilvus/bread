#![feature(destructuring_assignment)]

use crossbeam::thread;
use num::complex::Complex32;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rand_distr::Normal;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use bread::*;

static TIMER_CHECK_MS: Duration = Duration::from_millis(50);

fn main() {
    let config = get_config();
    if config.cycles > 1 && !config.keep {
        panic!("More than one cycle and not keeping");
    } else if config.cycles == 1 {
        println!("Running");
        run();
    } else {
        for i in 0..config.cycles {
            println!("Cycle {}", i);
            run();
        }
    }
}
fn run() {
    let mut brot = Brot {
        config: get_config(),
        results: None,
    };
    brot.run();
    brot.print_stats1();
    println!("Storing");
    brot.store();
    brot.print_stats2();
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

                        let mut buffers = vec![
                            HitBuffer::new(
                                self.config.width,
                                self.config.height,
                                self.config.buffer_min,
                                self.config.buffer_max
                            );
                            self.config.buffers.len()
                        ];
                        let mut timer =
                            Timer::new(Instant::now(), self.config.duration, TIMER_CHECK_MS);

                        let z_initial = Complex32::new(0.0, 0.0);

                        let mut c = Self::rand_complex(self.config.min, self.config.max, &mut rng);
                        let mut c_iterations = 0;
                        let mut c_hits = 0;
                        while !timer.check() {
                            //let c = Self::rand_complex(self.config.min, self.config.max, &mut rng);
                        
                            let new_c = self.mutate(c, &mut rng);
                            if Self::approximate_is_in_mandelbrot(new_c) {
                                continue;
                            }
                            let (new_iterations, new_hits) = if let Some(i) = self.count_hitting(z_initial, new_c, iterations) {
                                i
                            } else {
                                continue;
                            };
                            if c_hits == 0 || rng.gen_range(0.0..1.0) < (new_hits as f32) / (c_hits as f32) {
                                c = new_c;
                                c_iterations = new_iterations;
                                c_hits = new_hits;
                            }


                            for (b_index, b) in self.config.buffers.iter().enumerate() {
                                if b.min_iterations <= c_iterations && c_iterations < b.max_iterations {
                                    Self::iterate_step(z_initial, c, 2.0, b.max_iterations, |i, z| {
                                        if b.min_iterations <= i && i < b.max_iterations {
                                            buffers[b_index].hit(z, ((iterations as f32)/ (c_hits as f32)) as u32);
                                        }
                                    });
                                }
                            }
                        }

                        buffers
                    }));
                }
                let mut buffers = vec![
                    HitBuffer::new(
                        self.config.width,
                        self.config.height,
                        self.config.buffer_min,
                        self.config.buffer_max
                    );
                    self.config.buffers.len()
                ];
                for thread in threads {
                    for (i, buffer) in thread
                        .join()
                        .expect("Thread panicked")
                        .drain(..)
                        .enumerate()
                    {
                        buffers[i].buffer += buffer.buffer;
                    }
                }

                buffers
            })
            .expect("Error while executing threads"),
        );
    }

    fn count_hitting(&self, z: Complex32, c: Complex32, iterations: u32) -> Option<(u32, u32)> {
        let mut hits = 0;
        let mut hit = false;
        let result = Self::iterate_step(z, c, 2.0, iterations, |_, c| {
            if self.config.buffer_min.re < c.re
                && c.re < self.config.buffer_max.re
                && self.config.buffer_min.im < c.im
                && c.im < self.config.buffer_max.im
            {
                hits += 1;
                hit = true;
            }
        });
        if let Some(it) = result {
            if hit {
                Some((it, hits))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn rand_complex(min: Complex32, max: Complex32, rng: &mut impl Rng) -> Complex32 {
        Complex32::new(rng.gen_range(min.re..max.re), rng.gen_range(min.im..max.im))
    }
    fn rand_complex_normal(mean: Complex32, deviation: f32, rng: &mut impl Rng) -> Complex32 {
        Complex32::new(
            rng.sample(Normal::new(mean.re, deviation).unwrap()),
            rng.sample(Normal::new(mean.im, deviation).unwrap()),
        )
    }
    fn mutate(&self, c: Complex32, rng: &mut impl Rng) -> Complex32 {
        // TODO make mutation probability configurable
        if rng.gen_range(0.0..1.0) < 0.1 {
            Self::rand_complex(self.config.min, self.config.max, rng)
        } else {
            // TODO make deviation configurable
            Self::rand_complex_normal(c, 0.005, rng)
        }
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

    fn print_stats1(&self) {
        for (i, buffer) in self.results.as_ref().unwrap().iter().enumerate() {
            let samples = buffer
                .buffer
                .buffer()
                .iter()
                .map(|i| *i as u64)
                .sum::<u64>();
            println!("Buffer {} has {} samples, {:.2} samples/s, {:.2} samples/pixel, {:.4} samples/pixel/s",
                    i,
                    samples,
                    samples as f64 / self.config.duration.as_secs_f64(),
                    samples as f64 / (buffer.buffer.width() * buffer.buffer.height()) as f64,
                    samples as f64 / (buffer.buffer.width() * buffer.buffer.height()) as f64 / self.config.duration.as_secs_f64());
        }
    }
    fn print_stats2(&self) {
        for (i, buffer) in self.results.as_ref().unwrap().iter().enumerate() {
            let samples = buffer
                .buffer
                .buffer()
                .iter()
                .map(|i| *i as u64)
                .sum::<u64>();
            println!(
                "Buffer {} has {:.2} samples, {:.2} samples/pixel",
                i,
                samples,
                samples as f64 / (buffer.buffer.width() * buffer.buffer.height()) as f64
            );
        }
    }

    fn store(&mut self) {
        for (i, buffer) in self.results.as_mut().unwrap().iter_mut().enumerate() {
            let path = format!("buffer-{}.bread", i);

            if Path::new(&path).exists() && self.config.keep {
                let old_buffer = Buffer::load(buffer.buffer.width(), buffer.buffer.height(), &path)
                    .expect("Could not load old buffer");

                buffer.buffer += old_buffer;
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

    fn hit(&mut self, c: Complex32, i: u32) {
        let x = (c.re - self.min.re) / (self.max.re - self.min.re) * self.buffer.width() as f32;
        let y = (c.im - self.min.im) / (self.max.im - self.min.im) * self.buffer.height() as f32;

        if x < 0. || y < 0. || x >= self.buffer.width() as f32 || y >= self.buffer.height() as f32 {
            return;
        }

        self.buffer.set(
            x as usize,
            y as usize,
            self.buffer.get(x as usize, y as usize) + i,
        );
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
