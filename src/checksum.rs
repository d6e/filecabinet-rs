use data_encoding::HEXLOWER;
use ring::digest::{Context, Digest, SHA256};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::ffi::OsStr;

fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest, Box<dyn std::error::Error>> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}

pub fn generate_sha256(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut output_filename: String = path.file_name().and_then(OsStr::to_str).unwrap().to_string();
    let original_filename = output_filename.clone();
    output_filename.push_str(".sha256");
    let mut output_path = path.clone();
    output_path.set_file_name(output_filename);
    let mut output_file = File::create(output_path)?;
    let input = File::open(path)?;
    let reader = BufReader::new(input);
    let digest = sha256_digest(reader)?;
    let hex_encoded = HEXLOWER.encode(digest.as_ref());
    let output = format!("{}  {}\n", hex_encoded, original_filename);
    output_file.write_all(output.as_bytes())?;
    Ok(())
}
