use cocoon::Cocoon;
use std::fs::File;
use std::convert::AsRef;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use atomicwrites::{AtomicFile, AllowOverwrite};

pub const ENCRYPTION_FILE_EXT: &str = ".cocoon";

pub fn decrypt_file(path: &str, password: &str) -> Result<(), String> {
    if path.ends_with(ENCRYPTION_FILE_EXT) {
        let cocoon = Cocoon::new(password.as_bytes());
        let mut encrypted_file = File::open(path).unwrap();
        let data = cocoon.parse(&mut encrypted_file).expect(&format!("Unable to decrypt {}", path));
        let decrypted_path = get_decrypted_name(path);
        let unencrypted = AtomicFile::new(decrypted_path, AllowOverwrite);
        unencrypted.write(|f| {f.write_all(&data)}).unwrap();
        return Ok(());
    } else {
        return Err(format!("Error: '{}' is not encrypted.", path));
    }
}

pub fn encrypt_file<A: AsRef<Path>, B: AsRef<Path>>(source: A, target: B, password: &str) -> Result<(), String> {
    let source = source.as_ref();
    let target = target.as_ref();
    let cocoon = Cocoon::new(password.as_bytes());
    let encrypted_path = target;
    let mut unencrypted = File::open(source).expect(&format!("Cannot open {}", source.to_string_lossy()));
    let mut buffer = Vec::new();
    unencrypted.read_to_end(&mut buffer).unwrap();
    let encrypted_file = AtomicFile::new(encrypted_path, AllowOverwrite);
    encrypted_file.write(|f| {
        cocoon.dump(buffer, f)
    }).unwrap();
    return Ok(());
}

pub fn get_decrypted_name<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    let mut filename = path.file_name().unwrap().to_str().unwrap().to_string();
    let basepath = path.parent().unwrap();
    // if it ends with the encryption extension, remove it.
    if filename.ends_with(ENCRYPTION_FILE_EXT) {
        filename = filename.replace(ENCRYPTION_FILE_EXT, "");
    }
    return basepath.join(filename);
}

// Appends `.cocoon` on the end of the filepath
pub fn get_encrypted_name<P: AsRef<Path>>(path: P) -> PathBuf {
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
    let target = get_encrypted_name(&clear_text);
    encrypt_file(&clear_text, target, "password").unwrap();

    // Delete old file.
    std::fs::remove_file(&clear_text).unwrap();

    // Decrypt it.
    let mut cipher_text = tempdir.clone();
    cipher_text.push("test.txt.cocoon");
    decrypt_file(cipher_text.to_str().unwrap(), "password").unwrap();

    // Verify file was encrypted and decrypted without loss.
    let mut f = File::open(clear_text).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();

    assert_eq!(data.to_vec(), buffer);

    // Cleanup
    std::fs::remove_dir_all(tempdir).unwrap();
}