use byteorder::{BigEndian, ReadBytesExt};
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Debug, Clone, PartialEq)]
pub struct PngChunk {
    pub chunk_type: String,
    pub length: u32,
}

impl PngChunk {
    pub fn description(&self) -> &'static str {
        match self.chunk_type.as_str() {
            "IHDR" => "Image Header",
            "PLTE" => "Palette",
            "IDAT" => "Image Data",
            "IEND" => "Image Trailer",
            "tEXt" => "Textual Data",
            "iTXt" => "International Text",
            "zTXt" => "Compressed Text",
            "gAMA" => "Gamma",
            "cHRM" => "Chromaticity",
            "sRGB" => "sRGB Color Space",
            "bKGD" => "Background Color",
            "pHYs" => "Physical Pixel Dimensions",
            "sBIT" => "Significant Bits",
            "tIME" => "Last Modification Time",
            "hIST" => "Image Histogram",
            "tRNS" => "Transparency",
            "iCCP" => "Embedded ICC Profile",
            "sPLT" => "Suggested Palette",
            "eXIf" => "EXIF Metadata",
            _      => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PngFile {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub compression_method: u8,
    pub filter_method: u8,
    pub interlace_method: u8,
    pub chunks: Vec<PngChunk>,
}

impl PngFile {
    pub fn color_type_name(&self) -> &'static str {
        match self.color_type {
            0 => "Grayscale",
            2 => "RGB",
            3 => "Indexed (Palette)",
            4 => "Grayscale + Alpha",
            6 => "RGBA",
            _ => "Unknown",
        }
    }

    pub fn interlace_name(&self) -> &'static str {
        match self.interlace_method {
            0 => "None",
            1 => "Adam7",
            _ => "Unknown",
        }
    }

    pub fn parse(path: &PathBuf) -> io::Result<Self> {
        let mut file = File::open(path)?;

        let mut sig = [0u8; 8];
        file.read_exact(&mut sig)?;
        if sig != PNG_SIGNATURE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not a PNG file"));
        }

        // First chunk must be IHDR (13 bytes of data)
        let ihdr_len = file.read_u32::<BigEndian>()?;
        if ihdr_len < 13 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "IHDR too short"));
        }
        let mut ihdr_type = [0u8; 4];
        file.read_exact(&mut ihdr_type)?;
        if &ihdr_type != b"IHDR" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "First chunk is not IHDR"));
        }

        let width              = file.read_u32::<BigEndian>()?;
        let height             = file.read_u32::<BigEndian>()?;
        let bit_depth          = file.read_u8()?;
        let color_type         = file.read_u8()?;
        let compression_method = file.read_u8()?;
        let filter_method      = file.read_u8()?;
        let interlace_method   = file.read_u8()?;

        // Skip any extra IHDR bytes + CRC (4 bytes)
        let extra = (ihdr_len as usize).saturating_sub(13) + 4;
        let mut skip_buf = vec![0u8; extra];
        file.read_exact(&mut skip_buf)?;

        let mut chunks = vec![PngChunk { chunk_type: "IHDR".to_string(), length: ihdr_len }];

        loop {
            let length = match file.read_u32::<BigEndian>() {
                Ok(l) => l,
                Err(_) => break,
            };
            let mut type_buf = [0u8; 4];
            if file.read_exact(&mut type_buf).is_err() { break; }
            let chunk_type = String::from_utf8_lossy(&type_buf).into_owned();

            let is_end = chunk_type == "IEND";

            let mut data_and_crc = vec![0u8; length as usize + 4];
            let _ = file.read_exact(&mut data_and_crc);

            chunks.push(PngChunk { chunk_type, length });

            if is_end { break; }
        }

        Ok(PngFile {
            width,
            height,
            bit_depth,
            color_type,
            compression_method,
            filter_method,
            interlace_method,
            chunks,
        })
    }
}
