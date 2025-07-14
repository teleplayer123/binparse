pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffffffff;
    let poly = 0xedb88320;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ poly;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

pub fn xor_encode_bytes(data: &mut Vec<u8>, key: u8) -> Vec<u8> {
    let mut encoded_data = Vec::new();
    for byte in data.iter_mut() {
        *byte ^= key;
        encoded_data.push(*byte);
    }
    encoded_data
}
