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
            (self.buf[1] as u16) << -off | (self.buf[2] as u16) >> (8 + off)
        };
        word |= ((self.buf[0] << self.bit_pos) as u16) << (self.word_size - 8);
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
            self.buf[2] = (word << (8 + off)) as u8;
        }
        self.buf[0] |= (word >> (8 - off)) as u8;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_1() {
        let data: Vec<u8> = (0..=8).collect();
        let mut stream = BitStream::read_stream(Cursor::new(data), 9).unwrap();
        assert_eq!(stream.next(), Some(0b000000000));
        assert_eq!(stream.next(), Some(0b000000100));
        assert_eq!(stream.next(), Some(0b000010000));
        assert_eq!(stream.next(), Some(0b000110000));
        assert_eq!(stream.next(), Some(0b010000000));
        assert_eq!(stream.next(), Some(0b101000001));
        assert_eq!(stream.next(), Some(0b100000011));
        assert_eq!(stream.next(), Some(0b100001000));
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_read_2() {
        let data: Vec<u8> = (0..=8).collect();
        let mut stream = BitStream::read_stream(Cursor::new(data), 12).unwrap();
        assert_eq!(stream.next(), Some(0b000000000000));
        assert_eq!(stream.next(), Some(0b000100000010));
        assert_eq!(stream.next(), Some(0b000000110000));
        assert_eq!(stream.next(), Some(0b010000000101));
        assert_eq!(stream.next(), Some(0b000001100000));
        assert_eq!(stream.next(), Some(0b011100001000));
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_read_3() {
        let data: Vec<u8> = (0..=8).collect();
        let mut stream = BitStream::read_stream(Cursor::new(data), 16).unwrap();
        assert_eq!(stream.next(), Some(0b0000000000000001));
        assert_eq!(stream.next(), Some(0b0000001000000011));
        assert_eq!(stream.next(), Some(0b0000010000000101));
        assert_eq!(stream.next(), Some(0b0000011000000111));
        assert_eq!(stream.next(), Some(0b0000100000000000));
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_write_1() {
        let mut data = Vec::new();
        let mut stream = BitStream::write_stream(&mut data, 9).unwrap();
        stream.write(0b000000000).unwrap();
        stream.write(0b000000100).unwrap();
        stream.write(0b000010000).unwrap();
        stream.write(0b000110000).unwrap();
        stream.write(0b010000000).unwrap();
        stream.write(0b101000001).unwrap();
        stream.write(0b100000011).unwrap();
        stream.write(0b100001000).unwrap();
        stream.flush().unwrap();
        assert_eq!(data, (0..=8).collect::<Vec<u8>>());
    }

    #[test]
    fn test_write_2() {
        let mut data = Vec::new();
        let mut stream = BitStream::write_stream(&mut data, 12).unwrap();
        stream.write(0b000000000000).unwrap();
        stream.write(0b000100000010).unwrap();
        stream.write(0b000000110000).unwrap();
        stream.write(0b010000000101).unwrap();
        stream.write(0b000001100000).unwrap();
        stream.write(0b011100001000).unwrap();
        stream.flush().unwrap();
        assert_eq!(data, (0..=8).collect::<Vec<u8>>());
    }

    #[test]
    fn test_write_3() {
        let mut data = Vec::new();
        let mut stream = BitStream::write_stream(&mut data, 16).unwrap();
        stream.write(0b0000000000000001).unwrap();
        stream.write(0b0000001000000011).unwrap();
        stream.write(0b0000010000000101).unwrap();
        stream.write(0b0000011000000111).unwrap();
        stream.write(0b0000100000000000).unwrap();
        stream.flush().unwrap();
        assert_eq!(data, (0..=8).chain(0..=0).collect::<Vec<u8>>());
    }
}

// 00000000 00000001 00000010 00000011 00000100 00000101 00000110 00000111 00001000
// 000000000000 000100000010 000000110000 010000000101 000001100000 011100001000
