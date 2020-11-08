use cocoon::Cocoon;
use std::fs::File;
use std::error::Error;


pub fn encrypt_file(password: &str, file: &mut File, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let cocoon = Cocoon::new(password.as_bytes());
    cocoon.dump(data, file).unwrap();
    Ok(())
}

pub fn decrypt_file(password: &str, file: &mut File) -> Result<Vec<u8>, Box<dyn Error>> {
    let cocoon = Cocoon::new(password.as_bytes());
    let data = cocoon.parse(file);
    Ok(data.unwrap())
}
