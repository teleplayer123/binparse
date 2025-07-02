use std::env;
use std::io;

mod srec;
mod uboot;
mod ihex;
mod dtb;


fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let cli_args = CliArgs::new(&args);
    let outfile = cli_args.outfile.to_string();
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
    } else if mode == "dtb" {
        match dtb::parse_dtb_file(&filename) {
            Ok(header) => {
                println!("Successfully parsed DTB file '{}'. Header: {:?}", filename, header);
            }
            Err(e) => {
                eprint!("Error processing DTB file: {}", e);
            }
        }
        Ok(())
    } else {
        eprintln!("Unsupported mode: {}", mode);
        Ok(())
    }
}

struct CliArgs {
    mode: String,
    infile: String,
    outfile: String,
}

impl CliArgs {
    fn new(args: &[String]) -> CliArgs {
        if args.len() < 3 {
            panic!("Usage: {} <input_file> <output_file> [mode]", args[0]);
        }
        let infile = args[1].clone();
        let outfile = args[2].clone();
        let mode = if args.len() > 3 {
            args[3].clone()
        } else {
            "uboot".to_string()
        };
        if mode != "uboot" && mode != "srec" && mode != "ihex" && mode != "dtb" {
            panic!("Invalid mode: {}. Options: [uboot|srec|ihex|dtb]", mode);
        }

        CliArgs { infile, outfile, mode }
    }
}
