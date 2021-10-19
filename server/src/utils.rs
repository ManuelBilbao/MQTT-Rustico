const MAX_MULTIPLIER: usize = 128*128*128;

fn remaining_length_decode(buffer: &Vec<u8>) -> Result<usize, String> {
    let mut multiplier: usize = 1;
    let mut value: usize = 0;

    for byte in buffer.iter() {
        value = value + ((byte & 127) as usize) * multiplier;
        multiplier *= 128;
        if multiplier > MAX_MULTIPLIER {
            return Err("Malformed reamining length".to_string());
        }
    }

    Ok(value)
}

fn remaining_length_encode(remaining_length: usize) -> Vec<u8> {
    let mut byte: u8;
    let mut result = Vec::<u8>::new();
    let mut length = remaining_length;

    while length > 0 {
        byte = (length % 128) as u8;
        length /= 128;
        if length > 0 {
            byte = byte | 128;
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
        assert_eq!(remaining_length_decode(&vec![0x40]), Ok(64));
    }

    #[test]
    fn test05_remaining_length_decode_correcto2() {
        assert_eq!(remaining_length_decode(&vec![0xC1, 0x02]), Ok(321));
    }

    #[test]
    fn test06_remaining_length_decode_incorrecto() {
        assert_ne!(remaining_length_decode(&vec![0x40]), Ok(60));
    }
}