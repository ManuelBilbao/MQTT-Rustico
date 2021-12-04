use std::io::Read;
use std::net::TcpStream;

const MAX_MULTIPLIER: usize = 128 * 128 * 128;

pub fn remaining_length_read(stream: &mut TcpStream) -> Result<usize, String> {
    let mut buffer = [0u8; 1];
    if let Err(e) = stream.read_exact(&mut buffer) {
        return Err(format!("Error al leer del stream 1: {}", e.to_string()));
    }

    let mut byte: u8 = buffer[0];

    let mut multiplier: usize = 0x80;
    let mut value: usize = (byte & 0x7F) as usize;

    while byte & 0x80 == 0x80 {
        if let Err(e) = stream.read_exact(&mut buffer) {
            return Err(format!("Error al leer del stream: {}", e.to_string()));
        }
        byte = buffer[0];
        value += ((byte & 0x7F) as usize) * multiplier;
        multiplier *= 0x80;
        if multiplier > MAX_MULTIPLIER {
            return Err("Malformed reamining length".to_string());
        }
    }

    Ok(value)
}

fn _remaining_length_decode(buffer: &[u8]) -> Result<usize, String> {
    let mut byte: u8 = buffer[0];

    let mut i: usize = 1;
    let mut multiplier: usize = 0x80;
    let mut value: usize = (byte & 0x7F) as usize;

    while byte & 0x80 == 0x80 {
        byte = buffer[i];
        value += ((byte & 0x7F) as usize) * multiplier;
        multiplier *= 0x80;
        if multiplier > MAX_MULTIPLIER {
            return Err("Malformed reamining length".to_string());
        }
        i += 1;
    }

    Ok(value)
}

pub fn remaining_length_encode(remaining_length: usize) -> Vec<u8> {
    let mut byte: u8;
    let mut result = Vec::<u8>::new();
    let mut length = remaining_length;

    while length > 0 {
        byte = (length % 128) as u8;
        length /= 128;
        if length > 0 {
            byte |= 128;
        }
        result.push(byte);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test01_remaining_length_encode_correcto1() {
        assert_eq!(remaining_length_encode(64), vec![0x40]);
    }

    #[test]
    fn test02_remaining_length_encode_correcto2() {
        assert_eq!(remaining_length_encode(321), vec![0xC1, 0x02]);
    }

    #[test]
    fn test03_remaining_length_encode_incorrecto() {
        assert_ne!(remaining_length_encode(64), vec![0x42]);
    }

    #[test]
    fn test04_remaining_length_decode_correcto1() {
        assert_eq!(_remaining_length_decode(&vec![0x40]), Ok(64));
    }

    #[test]
    fn test05_remaining_length_decode_correcto2() {
        assert_eq!(_remaining_length_decode(&vec![0xC1, 0x02]), Ok(321));
    }

    #[test]
    fn test06_remaining_length_decode_incorrecto() {
        assert_ne!(_remaining_length_decode(&vec![0x40]), Ok(60));
    }
}
