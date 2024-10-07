use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufReader, Bytes, Read},
};

fn compress(stream: Bytes<BufReader<File>>) -> Vec<u16> {
    let mut dictionary: HashMap<Vec<u8>, u16> = (0..=255).map(|x| (vec![x], x as u16)).collect();
    dictionary.insert(256u16.to_be_bytes().to_vec(), 256);
    let mut output = Vec::new();
    let mut symbol = Vec::new();

    for byte in stream.map(|x| x.unwrap()) {
        symbol.push(byte);

        if !dictionary.contains_key(&symbol) {
            output.push(dictionary[&symbol[..symbol.len() - 1]]);
            dictionary.insert(
                std::mem::replace(&mut symbol, vec![byte]),
                dictionary.len() as u16,
            );
        }
    }
    output.push(dictionary[&symbol[..]]);
    output
}

fn decompress(mut stream: impl Iterator<Item = u16>) -> Vec<u8> {
    let mut dictionary: HashMap<u16, Vec<u8>> = (0..=255).map(|x| (x as u16, vec![x])).collect();
    dictionary.insert(256, 256u16.to_be_bytes().to_vec());
    let mut symbol = vec![stream.next().unwrap() as u8];
    let mut output = symbol.clone();

    for word in stream {
        let val = dictionary.get(&word);
        dictionary.insert(dictionary.len() as u16, {
            if let Some(val) = val {
                symbol.push(val[0]);
                output.extend_from_slice(val);
                std::mem::replace(&mut symbol, val.clone())
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
    let file_path = env::args().nth(1).expect("No file path given");
    let bytes = BufReader::new(File::open(file_path)?).bytes();
    let output = decompress(compress(bytes).into_iter());

    for byte in output {
        if byte <= 255 {
            print!("{}", char::from_u32(byte as u32).unwrap());
        } else {
            print!("{} ", byte);
        }
    }
    Ok(())
}
