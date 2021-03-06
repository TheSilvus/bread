use palette::{FromColor, Mix};

use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::ops::{Add, AddAssign};

pub struct Buffer<T> {
    width: usize,
    height: usize,
    buffer: Vec<T>,
}
impl<T: Default + Clone> Buffer<T> {
    pub fn new_default(width: usize, height: usize) -> Buffer<T> {
        Buffer {
            width,
            height,
            buffer: vec![T::default(); width * height],
        }
    }
}
impl<T> Buffer<T> {
    pub fn get(&self, x: usize, y: usize) -> &T {
        &self.buffer[y * self.width + x]
    }
    pub fn set(&mut self, x: usize, y: usize, t: T) {
        self.buffer[y * self.width + x] = t;
    }

    pub fn buffer(&self) -> &Vec<T> {
        &self.buffer
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
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
// TODO is it possible to remove the Copy bound here?
// TODO implement different variants for references
impl<T: AddAssign + Copy> AddAssign for Buffer<T> {
    fn add_assign(&mut self, other: Buffer<T>) {
        assert!(self.width == other.width);
        assert!(self.height == other.height);

        for i in 0..self.buffer.len() {
            self.buffer[i] += other.buffer[i];
        }
    }
}

impl<U, T: Add<T, Output = U>> Add for Buffer<T> {
    type Output = Buffer<U>;
    fn add(mut self, mut other: Buffer<T>) -> Buffer<U> {
        assert!(self.width() == other.width && self.height == other.height);
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .drain(..)
                .zip(other.buffer.drain(..))
                .map(|(x, y)| x + y)
                .collect(),
        }
    }
}

impl Buffer<u32> {
    pub fn to_u8(&self) -> Buffer<u8> {
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
    pub fn to_f32(&self) -> Buffer<f32> {
        let max = *self.buffer.iter().max().expect("") as f32;
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|x| *x as f32 / max)
                .collect::<Vec<_>>(),
        }
    }

    pub fn store(&self, file: &str) -> Result<(), io::Error> {
        let mut f = BufWriter::new(File::create(file)?);
        for i in &self.buffer {
            f.write_all(&i.to_le_bytes())?;
        }

        Ok(())
    }

    pub fn load(width: usize, height: usize, file: &str) -> Result<Buffer<u32>, io::Error> {
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
impl Buffer<f32> {
    /**
     * $a \in [1, \infty)$
     *
     * $a = 1$ leads to identity, as $a \to \infty$ we almost everywhere approach f(x) = 1
     */
    pub fn polynomial(&mut self, a: f32) -> Buffer<f32> {
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|x| (*x as f32).powf(1.0 / a))
                .collect(),
        }
    }

    /**
     * $a \in (0, \infty)$
     *
     * Approaches identity as $a \to 0$, approaches f(x) = 1 almost everywhere as $a \to infty$
     */
    pub fn exponential(&mut self, a: f32) -> Buffer<f32> {
        use std::f32::consts::E;
        let divisor = 1.0 - E.powf(-a);
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|x| (1.0 - E.powf(-a * x)) / divisor)
                .collect(),
        }
    }

    pub fn expose(&mut self, a: f32) -> Buffer<f32> {
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self.buffer.iter().map(|x| a * x).collect(),
        }
    }

    pub fn to_u8(&self) -> Buffer<u8> {
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|x| (x * 255.0).clamp(0.0, 255.0) as u8)
                .collect::<Vec<_>>(),
        }
    }

    pub fn to_lab_rgb(&self, c: (u8, u8, u8)) -> Buffer<palette::Laba<palette::white_point::D65>> {
        self.to_lab(palette::Lab::from_rgb(
            palette::LinSrgb::from_components(c).into_format(),
        ))
    }

    pub fn to_lab(&self, c: palette::Lab) -> Buffer<palette::Laba<palette::white_point::D65>> {
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|x| {
                    let (l, a, b) = c.into_components();
                    palette::Laba::<palette::white_point::D65>::new(l, a, b, *x)
                })
                .collect::<Vec<_>>(),
        }
    }
}
impl Buffer<palette::Alpha<palette::Lab, f32>> {
    pub fn mix(b: Vec<Self>) -> Buffer<palette::Alpha<palette::Lab, f32>> {
        let mut result = Vec::new();
        for i in 0..b[0].buffer.len() {
            let mut color = b[0].buffer[i];
            for j in 1..b.len() {
                color = b[j].buffer[i].mix(&color, 1.0 / (j as f32 + 1.0));
            }

            result.push(color);
        }
        Buffer {
            width: b[0].width,
            height: b[0].height,
            buffer: result,
        }
    }

    pub fn to_3u8(&self, base: palette::Lab) -> Buffer<(u8, u8, u8)> {
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|x| {
                    palette::Srgb::from_lab(base.mix(&x.color, x.alpha))
                        .into_format::<u8>()
                        .into_components()
                })
                .collect::<Vec<_>>(),
        }
    }
    pub fn to_3u8_rgb(&self, c: (u8, u8, u8)) -> Buffer<(u8, u8, u8)> {
        self.to_3u8(palette::Lab::from_rgb(
            palette::LinSrgb::from_components(c).into_format(),
        ))
    }
}
impl Buffer<(u8, u8, u8)> {
    pub fn join(b1: Buffer<u8>, b2: Buffer<u8>, b3: Buffer<u8>) -> Buffer<(u8, u8, u8)> {
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

    pub fn flatten(&self) -> Vec<u8> {
        use std::iter::once;

        self.buffer
            .iter()
            .flat_map(|(u1, u2, u3)| once(*u1).chain(once(*u2)).chain(once(*u3)))
            .collect()
    }
    pub fn inverse(&self) -> Self {
        Buffer {
            width: self.width,
            height: self.height,
            buffer: self
                .buffer
                .iter()
                .map(|(r, g, b)| (255 - r, 255 - g, 255 - b))
                .collect::<Vec<_>>(),
        }
    }
}
