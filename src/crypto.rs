use cocoon::{Cocoon, Creation};

// fn main() -> Result<(), Box<dyn Error>> {
    // let cocoon = Cocoon::new(b"password");
    // let mut file = File::create("foo.cocoon")?;
    // encrypt_file(&cocoon, &mut file, "data".as_bytes().to_vec());

    // let mut unencrypted_file = File::create("foo.txt")?;
    // decrypt_file(&cocoon, &mut unencrypted_file);

//     Ok(())
// }

// #[post("/document", data = "<doc>")]
// fn new(doc: Form<Document>) -> Result<(), Box<dyn Error>> {
//     let cocoon = Cocoon::new(b"password");
//     let mut file = File::create(format!(
//         "{}_{}_{}_{}.cocoon",
//         doc.time, doc.institution, doc.document_name, doc.page
//     ))?;
//     let data: String = fs::read_to_string(&doc.original_file)?;
//     encrypt_file(&cocoon, &mut file, data.as_bytes().to_vec())?;
//     Ok(())
// }

fn encrypt_file(
    cocoon: &Cocoon<ThreadRng, Creation>,
    file: &mut File,
    data: Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    cocoon.dump(data, file).unwrap();
    Ok(())
}

fn decrypt_file(
    cocoon: &Cocoon<ThreadRng, Creation>,
    file: &mut File,
) -> Result<(), Box<dyn Error>> {
    let data = cocoon.parse(file).unwrap();
    Ok(())
}

