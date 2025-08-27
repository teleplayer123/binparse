use std::fs;
use std::io::{self, BufRead, Write};

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

pub fn parse_ihex_file(file_path: &str, outfile: &str) -> io::Result<()> {
    let file = fs::File::open(file_path)?;
    let reader = io::BufReader::new(file);
    let mut records = Vec::new();
    let mut curr_address: u16 = 0;

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let record = IHexRecord::from_line(&line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        records.push(record);
    }
    let mut output_file = fs::File::create(outfile)?;
    for record in &records {
        println!("Length: 0x{:x}, Address: 0x{:x}, Type: 0x{:x}, Checksum: 0x{:x}", &record.length, &record.address, &record.record_type, &record.checksum);
        let address = record.address;
        if record.record_type == 0 {
            if address > curr_address {
                let padding = vec![0u8; (address - curr_address) as usize];
                output_file.write_all(&padding)?;
            }                
            output_file.write_all(&record.data)?;
            curr_address = address + record.length as u16;
        } else if record.record_type == 1 {
            println!("End of file record encountered.");
            break; // End of file record
        }
    }

    Ok(())
}