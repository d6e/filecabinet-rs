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

fn make_digest(input: File) -> String {
    let reader = BufReader::new(input);
    let digest = sha256_digest(reader).unwrap();
    let hex_encoded = HEXLOWER.encode(digest.as_ref());
    return hex_encoded;
}

pub fn validate_sha256(path: PathBuf) -> Result<bool, Box<dyn std::error::Error>> {
    let mut f = File::open(&path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    let contents: String = std::str::from_utf8(&buffer).unwrap().to_owned();
    if contents.len() < 0 {
        // TODO: handle empty checksum file.
        // return Err("Empty sha256 file".to_string());
    }
    let parent = path.parent().unwrap();
    let v: Vec<&str> = contents.split("  ").collect();
    // TODO: handle not properly formatted.
    let expected_digest: &str = v[0];
    let file = parent.join(v[1].to_string().trim());
    let input = File::open(&file).expect(&format!("Cannot open '{}'", file.to_str().unwrap()));
    let actual_digest: String = make_digest(input);
    return Ok(actual_digest == expected_digest);
}