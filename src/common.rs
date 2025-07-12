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