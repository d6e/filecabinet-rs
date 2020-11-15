use cocoon::Cocoon;
use std::fs::File;
use std::error::Error;
use std::convert::AsRef;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub const ENCRYPTION_FILE_EXT: &str = ".cocoon";

fn encrypt(password: &str, file: &mut File, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let cocoon = Cocoon::new(password.as_bytes());
    cocoon.dump(data, file).unwrap();
    Ok(())
}

fn decrypt(password: &str, file: &mut File) -> Result<Vec<u8>, Box<dyn Error>> {
    let cocoon = Cocoon::new(password.as_bytes());
    let data = cocoon.parse(file);
    Ok(data.unwrap())
}

pub fn decrypt_file(path: &str, password: &str) -> Result<(), String> {
    if path.ends_with(ENCRYPTION_FILE_EXT) {
        let decrypted_path = get_decrypted_name(path);
        let mut encrypted_file = File::open(path).unwrap();
        let data = decrypt(password, &mut encrypted_file).unwrap();
        let mut unecrypted: File = File::create(decrypted_path).unwrap();
        unecrypted.write(&data).unwrap();
        return Ok(());
    } else {
        return Err(format!("Error: '{}' is not encrypted.", path));
    }
}

pub fn encrypt_file(path: &str, password: &str) -> Result<(), String> {
    if ! path.ends_with(ENCRYPTION_FILE_EXT) {
        let encrypted_path = get_encrypted_name(path);
        let mut unencrypted = File::open(path).unwrap();
        let mut buffer = Vec::new();
        unencrypted.read_to_end(&mut buffer).unwrap();
        let encrypted_file = &mut File::create(encrypted_path.clone()).unwrap();
        encrypt(password, encrypted_file, buffer).unwrap();
        return Ok(());
    } else {
        return Err(format!("Error: '{}' is already encrypted.", path));
    }
}

pub fn get_decrypted_name<T: AsRef<Path>>(path: T) -> PathBuf {
    let path = path.as_ref();
    let mut filename = path.file_name().unwrap().to_str().unwrap().to_string();
    let basepath = path.parent().unwrap();
    // if it ends with the encryption extension, remove it.
    if filename.ends_with(ENCRYPTION_FILE_EXT) {
        filename = filename.replace(ENCRYPTION_FILE_EXT, "");
    }
    return basepath.join(filename);
}

pub fn get_encrypted_name<T: AsRef<Path>>(path: T) -> PathBuf {
    let path = path.as_ref();
    let mut filename = path.file_name().unwrap().to_str().unwrap().to_string();
    let basepath = path.parent().unwrap();
    // if it doesn't end with the encryption extension, add it.
    if ! filename.ends_with(ENCRYPTION_FILE_EXT) {
        filename.push_str(ENCRYPTION_FILE_EXT);
    }
    return basepath.join(filename);
}

#[test]
fn test_get_decrypted_name() {
    assert_eq!(get_decrypted_name(Path::new("/boop/loop/readme.md.cocoon")), Path::new("/boop/loop/readme.md"));
    assert_eq!(get_decrypted_name(Path::new("readme.md.cocoon")), Path::new("readme.md"));
}
#[test]
fn test_get_encrypted_name() {
    assert_eq!(get_encrypted_name(Path::new("/boop/loop/readme.md")), Path::new("/boop/loop/readme.md.cocoon"));
    assert_eq!(get_encrypted_name(Path::new("readme.md")), Path::new("readme.md.cocoon"));
}

#[test]
fn test_encrypt_decrypt_file() {
    // Create temporary test dir.
    let mut tempdir = std::env::temp_dir();
    tempdir.push("sdfasfd");
    std::fs::create_dir_all(&tempdir).unwrap();
    let data = b"hello world";

    // Create test file and write some data to it.
    let mut clear_text = tempdir.clone();
    clear_text.push("test.txt");
    let mut f: File = File::create(&clear_text).unwrap();
    f.write_all(data).unwrap();
    drop(f);

    // Encrypt it.
    encrypt_file(clear_text.to_str().unwrap(), "password").unwrap();

    // Delete old file.
    std::fs::remove_file(&clear_text).unwrap();

    // Decrypt it.
    let mut cipher_text = tempdir.clone();
    cipher_text.push("test.txt.cocoon");
    decrypt_file(cipher_text.to_str().unwrap(), "password").unwrap();

    // Verify file was encrypted and decrypted without loss.
    let mut f = File::open(&clear_text).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();

    assert_eq!(data.to_vec(), buffer);

    // Cleanup
    std::fs::remove_dir_all(tempdir).unwrap();
}