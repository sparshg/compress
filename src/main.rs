mod bitstream;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

use bitstream::BitStream;

fn compress<R: Read>(stream: R, output_path: PathBuf) -> io::Result<()> {
    let mut dictionary: HashMap<Vec<u8>, u16> = (0..=255).map(|x| (vec![x], x as u16)).collect();
    dictionary.insert(256u16.to_be_bytes().to_vec(), 256);
    let mut output = BitStream::write_stream(BufWriter::new(File::create(output_path)?), 9)?;
    let mut symbol = Vec::new();

    for byte in stream.bytes().map(|x| x.unwrap()) {
        symbol.push(byte);

        if !dictionary.contains_key(&symbol) {
            output.write(dictionary[&symbol[..symbol.len() - 1]])?;
            dictionary.insert(
                std::mem::replace(&mut symbol, vec![byte]),
                dictionary.len() as u16,
            );
        }
    }
    if !symbol.is_empty() {
        output.write(dictionary[&symbol[..]])?;
    }
    output.flush()?;
    Ok(())
}

fn decompress<R: Read>(mut stream: R) -> Vec<u8> {
    let mut stream = BitStream::read_stream(&mut stream, 9).unwrap();
    let mut dictionary: HashMap<u16, Vec<u8>> = (0..=255).map(|x| (x as u16, vec![x])).collect();
    dictionary.insert(256, 256u16.to_be_bytes().to_vec());

    // let mut buf = [0; 2];
    // stream.read_exact(&mut buf).unwrap();

    let mut symbol = vec![stream.next().unwrap() as u8];
    let mut output = symbol.clone();
    // output_file.write_all(&symbol)?;

    for word in stream {
        dictionary.insert(dictionary.len() as u16, {
            if let Some(entry) = dictionary.get(&word) {
                symbol.push(entry[0]);
                output.extend_from_slice(entry);
                std::mem::replace(&mut symbol, entry.clone())
            } else {
                symbol.push(symbol[0]);
                output.extend_from_slice(&symbol);
                symbol.clone()
            }
        });
    }
    output
}

fn main() -> std::io::Result<()> {
    let input_file_path = env::args().nth(1).expect("No file path given");
    let output_file_path = env::args().nth(2).expect("No file path given");
    let reader = BufReader::new(File::open(input_file_path)?);
    compress(reader, output_file_path.clone().into())?;
    let reader = BufReader::new(File::open(output_file_path)?);
    let output = decompress(reader);

    for byte in output {
        if byte <= 255 {
            print!("{}", char::from_u32(byte as u32).unwrap());
        } else {
            print!("{} ", byte);
        }
    }
    // let stream = BitStream::read_stream(reader, 11)?;
    // for i in stream {
    //     println!("{:011b}", i);
    // }

    Ok(())
}
