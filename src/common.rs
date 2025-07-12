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

pub fn crc32_from_file<P: AsRef<Path>>(path: P) -> io::Result<u32> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(crc32(&buffer))
}

pub fn align_8b(size: usize) -> usize {
    (size + 7) & !7
}