use std::io::Read;
use std::io::{self, Write};

pub struct BitStream<T> {
    stream: T,
    bit_pos: usize,
    word_size: usize,
    buf: [u8; 3],
    exhausted_pad: usize,
}

impl<R: Read> BitStream<R> {
    pub fn read_stream(mut stream: R, word_size: usize) -> io::Result<Self> {
        let mut buf = [0; 3];
        let read = stream.read(&mut buf)?;
        Ok(Self {
            stream,
            word_size,
            buf,
            bit_pos: 0,
            exhausted_pad: 3 - read,
        })
    }
}

impl<R: Read> Iterator for BitStream<R> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        match self.bit_pos {
            16 => {
                self.buf[0] = self.buf[2];
                let read = self.stream.read(&mut self.buf[1..]).ok()?;
                if read != 2 {
                    self.exhausted_pad += 2 - read;
                    if read == 0 {
                        self.buf[1] = 0;
                    }
                    self.buf[2] = 0;
                }
            }
            8.. => {
                self.buf[0] = self.buf[1];
                self.buf[1] = self.buf[2];
                if self.stream.read(&mut self.buf[2..]).ok()? == 0 {
                    self.exhausted_pad += 1;
                    self.buf[2] = 0;
                }
            }
            _ => {}
        };
        if self.exhausted_pad > 2 {
            return None;
        }

        self.bit_pos %= 8;
        let off = 16 - (self.word_size + self.bit_pos) as i8;
        let mut word = if off >= 0 {
            (self.buf[1] as u16) >> off
        } else {
            (self.buf[1] as u16) << -off | (self.buf[2] as u16) >> 8 + off
        };
        word |= ((self.buf[0] << self.bit_pos) as u16) << self.word_size - 8;
        self.bit_pos += self.word_size;
        Some(word)
    }
}

impl<W: Write> BitStream<W> {
    pub fn write_stream(stream: W, word_size: usize) -> io::Result<Self> {
        Ok(Self {
            stream,
            word_size,
            buf: [0; 3],
            bit_pos: 0,
            exhausted_pad: 0,
        })
    }
    pub fn write(&mut self, word: u16) -> io::Result<()> {
        let off = 16 - (self.word_size + self.bit_pos) as i8;
        if off >= 0 {
            self.buf[1] = (word << off) as u8;
        } else {
            self.buf[1] = (word >> -off) as u8;
            self.buf[2] = (word << 8 + off) as u8;
        }
        self.buf[0] |= (word >> 8 - off) as u8;
        self.bit_pos += self.word_size;
        match self.bit_pos {
            16 => {
                self.stream.write_all(&self.buf[..2])?;
                self.buf[0] = self.buf[2];
                self.buf[1] = 0;
                self.buf[2] = 0;
            }
            8.. => {
                self.stream.write_all(&self.buf[..1])?;
                self.buf[0] = self.buf[1];
                self.buf[1] = 0;
                self.buf[2] = 0;
            }
            _ => {}
        }
        self.bit_pos %= 8;
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.bit_pos > 0 {
            self.stream.write_all(&self.buf[..1])?;
        }
        self.stream.flush()
    }
}

// 00000000 00000000 00000000
// --bp--^         - (off = + on left, - on right)
//       ---ws=9---
