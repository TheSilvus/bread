use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::ops::Add;

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
impl<T: Copy + Add<T, Output = T>> Buffer<T> {
    pub fn add(&mut self, other: &Buffer<T>) {
        assert!(self.width == other.width);
        assert!(self.height == other.height);

        for i in 0..self.buffer.len() {
            self.buffer[i] = self.buffer[i] + other.buffer[i];
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
}
