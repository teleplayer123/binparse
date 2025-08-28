use std::fs;
use std::fs::File;
use std::io::{self, BufRead, Write, Seek, SeekFrom};

// Parse intel hex file

struct IHexRecord {
    length: u8,
    address: u16,
    record_type: u8,
    data: Vec<u8>,
    checksum: u8,
}

impl IHexRecord {
    fn new(length: u8, address: u16, record_type: u8, data: Vec<u8>, checksum: u8) -> Self {
        IHexRecord {
            length,
            address,
            record_type,
            data,
            checksum,
        }
    }

    fn from_line(line: &str) -> Result<Self, String> {
        if !line.starts_with(':') {
            return Err(format!("Invalid line format: {}", line));
        }

        let length = u8::from_str_radix(&line[1..3], 16).map_err(|e| e.to_string())?;
        let address = u16::from_str_radix(&line[3..7], 16).map_err(|e| e.to_string())?;
        let record_type = u8::from_str_radix(&line[7..9], 16).map_err(|e| e.to_string())?;
        let data = (0..length)
            .map(|i| {
                let start = 9 + (i as usize) * 2;
                let end = start + 2;
                u8::from_str_radix(&line[start..end], 16)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let checksum = u8::from_str_radix(&line[9 + length as usize * 2..11 + length as usize * 2], 16)
            .map_err(|e| e.to_string())?;

        Ok(IHexRecord::new(length, address, record_type, data, checksum))
    }
}

fn write_records_to_file(records: &[IHexRecord], outfile: &str) -> io::Result<()> {
    let mut file = File::create(outfile)?;
    let mut last_end: u64 = 0;

    for record in records {
        println!("Length: 0x{:x}, Address: 0x{:x}, Type: 0x{:x}, Checksum: 0x{:x}", &record.length, &record.address, &record.record_type, &record.checksum);
        if record.record_type != 0 {
            continue; // Only write data records
        }
        let address = record.address as u64;
        if address > last_end {
            // Pad with zeros
            let gap = address - last_end;
            file.write_all(&vec![0u8; gap as usize])?;
        }
        file.seek(SeekFrom::Start(address))?;
        file.write_all(&record.data)?;
        last_end = address + record.data.len() as u64;
    }
    Ok(())
}

pub fn parse_ihex_file(file_path: &str, outfile: &str) -> io::Result<()> {
    let file = fs::File::open(file_path)?;
    let reader = io::BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let record = IHexRecord::from_line(&line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        records.push(record);
    }
    write_records_to_file(&records, outfile)?;

    Ok(())
}