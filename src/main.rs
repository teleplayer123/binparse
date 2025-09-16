use std::env;
use std::io;

mod utils;
mod srec;
mod uboot;
mod ihex;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let cli_args = CliArgs::new(&args);
    let outfile = match &cli_args.outfile {
        Some(out) => out.to_string(),
        None => "".to_string(),
    };
    let filename = cli_args.infile.to_string();
    let mode = cli_args.mode.to_string();
    if mode == "uboot" {
        match uboot::parse_file(&filename, &outfile) {
            Ok(()) => {
                println!("Successfully extracted hex code from '{}' and wrote to '{}'", filename, outfile);
            }
            Err(e) => {
                eprint!("Error processing file: {}", e);
            }
        }
        Ok(())
    } else if mode == "srec" {
        match srec::parse_srecord_file(&filename, &outfile) {
            Ok(()) => {
                // Process the records if needed
                println!("Successfully extracted hex code from '{}' and wrote to '{}'", filename, outfile);
            }
            Err(e) => {
                eprint!("Error processing file: {}", e);
            }
        }
        Ok(())
    } else if mode == "ihex" {
        match ihex::parse_ihex_file(&filename, &outfile) {
            Ok(()) => {
                println!("Successfully extracted hex code from '{}' and wrote to '{}'", filename, outfile);
            }
            Err(e) => {
                eprint!("Error processing file: {}", e);
            }
        }
        Ok(())
    } else if mode == "crc32" {
        let data = std::fs::read(&filename)?;
        let crc = utils::utils::crc32(&data);
        println!("CRC32 of file '{}': {:08x}", filename, crc);
        Ok(())
    } else if mode == "oddparity" {
        let data = std::fs::read(&filename)?;
        for byte in data {
            let parity = utils::utils::oddparity(byte);
            println!("Odd parity for byte {:02x}: {:02x}", byte, parity);
        }
        Ok(())
    } else if mode == "xor" {
        let key: u8 = match cli_args.xorkey {
            Some(ref key_str) => key_str.parse().expect("Invalid XOR key, must be a byte (0-255)"),
            None => {
                eprintln!("XOR key is required for xor mode");
                return Ok(());
            }
        };
        let mut data = std::fs::read(&filename)?;
        let encoded_data = utils::utils::xor_encode_bytes(&mut data, key);
        std::fs::write(&outfile, encoded_data)?;
        println!("XOR encoded data written to '{}'", &outfile);
        Ok(())
        
    } else {
        eprintln!("Unsupported mode: {}", mode);
        Ok(())
    }
}

struct CliArgs {
    mode: String,
    infile: String,
    outfile: Option<String>,
    xorkey: Option<String>,
}

impl CliArgs {
    fn new(args: &[String]) -> CliArgs {
        if args.len() < 3 || args.len() > 5 {
            panic!("Usage: {} <mode> <input_file> [output_file] [xorkey]\nmodes: [uboot|srec|ihex|crc32|oddparity|xor]", args[0]);
        }
        let mode = args[1].clone();
        let infile = args[2].clone();
        let outfile = if args.len() >= 4 {
            Some(args[3].clone())
        } else {
            None
        };
        let xorkey = if args.len() == 5 {
            Some(args[4].clone())
        } else {
            None
        };
        if mode != "uboot" && mode != "srec" && mode != "ihex" && mode != "crc32" && mode != "oddparity" && mode != "xor" {
            panic!("Invalid mode: {}. Options: [uboot|srec|ihex|crc32|oddparity|xor]", mode);
        }

        CliArgs { mode, infile, outfile, xorkey}
    }
}
