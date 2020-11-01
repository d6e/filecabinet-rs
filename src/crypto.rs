use cocoon::Cocoon;
use std::fs::File;
use std::error::Error;


pub fn encrypt_file(file: &mut File, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let cocoon = Cocoon::new(b"password");
    cocoon.dump(data, file).unwrap();
    Ok(())
}

pub fn decrypt_file(file: &mut File) -> Result<Vec<u8>, Box<dyn Error>> {
    let cocoon = Cocoon::new(b"password");
    let data = cocoon.parse(file);
    Ok(data.unwrap())
}
