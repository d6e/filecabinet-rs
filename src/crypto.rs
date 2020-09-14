use cocoon::{Cocoon, Creation};
use std::fs::File;
use rand::rngs::ThreadRng;
use std::error::Error;


pub fn encrypt_file(
    file: &mut File,
    data: Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    let cocoon = Cocoon::new(b"password");
    cocoon.dump(data, file).unwrap();
    Ok(())
}

pub fn decrypt_file(
    cocoon: &Cocoon<ThreadRng, Creation>,
    file: &mut File,
) -> Result<(), Box<dyn Error>> {
    let data = cocoon.parse(file).unwrap();
    Ok(())
}
