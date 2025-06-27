use std::fs;
use std::io::{self, BufRead, Write};


pub fn parse_file(path: &str, outfile: &str) -> io::Result<()> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);  
    let mut output_file = fs::File::create(outfile)?;
    let mut hexstr = String::new();
    let mut line_num = 0;
    for line_result in reader.lines() {
        line_num = line_num + 1;
        println!("parsing line number: {}", line_num);
        match line_result {
            Ok(line) => {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let line = parts[1].trim().to_string();
                    let line = remove_space(&line);
                    if line.len() < 32 {
                        eprintln!("Invalid line {}, skipping...", line);
                    } else {
                        let line = line[..32].to_string();
                        for char in line.chars() {
                            if char.is_ascii_hexdigit() {
                                hexstr.push(char);
                            }
                        }
                    }
                    for i in (0..hexstr.len()).step_by(2) {
                        if i + 1 < hexstr.len() {
                            let byte_str = &hexstr[i..i + 2];
                            if let Ok(byte) = u8::from_str_radix(byte_str, 16) {
                                output_file.write_all(&[byte])?;
                            } else {
                                eprintln!("Warning: Invalid hex sequence '{}' in line: {}", byte_str, line);
                            }
                        } else if hexstr.len() % 2 != 0 {
                            eprintln!("Warning: Odd number of hex digits at the end of a line: {}", line);
                        }
                    }
                    hexstr.clear(); // Clear hexstr for the next line
                } else {
                    eprintln!("Warning: Invalid line format: {}, skipping...", line);
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
            }
        }
    }
    Ok(())
}

fn remove_space(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}