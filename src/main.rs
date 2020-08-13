#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[allow(dead_code)]
use std::fs::{File, read_to_string};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use std::io::prelude::*;
use clap::{Arg, App, value_t};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, NewAead};
use rand::prelude::*;
use blake3;

struct Config {
    verbose: bool,
    launch_web: bool,
    p_value: f32,
}

fn get_program_input() -> Config {
    let name_verbose = "verbose";
    let name_launch_web = "web";
    let name_p_value = "pvalue";
    let default_p_value = 1.0;
    let matches = App::new("filecabinet")
        .version("1.0")
        .author("Danielle <filecabinet@d6e.io>")
        .about("Filecabinet - A secure solution to managing scanned files.")
        .arg(Arg::with_name(name_verbose)
            .short("v")
            .long(name_verbose)
            .multiple(true)
            .help("Sets the level of verbosity"))
        .arg(Arg::with_name(name_launch_web)
            .short("w")
            .long(name_launch_web)
            .help("Launches the web server."))
        .arg(Arg::with_name(name_p_value)
            .short("p")
            .long("pvalue")
            .value_name("P")
            .help("Sets the P gain value of the PID controller.")
            .takes_value(true))
        .get_matches();
    Config {
        verbose: matches.is_present(name_verbose),
        launch_web: matches.is_present(name_launch_web),
        p_value: value_t!(matches, name_p_value, f32).unwrap_or(default_p_value),
    }
}

fn mk_nonce() -> Nonce {
    let mut rng = rand::thread_rng(); // TODO: maybe reuse
    let rand_int: u128 = rng.gen();
    let le = rand_int.to_le_bytes();
    Nonce::clone_from_slice(&le[0..12]) // A 96-bit nonce (12 bytes); unique per message
}

fn hash_key(key: &str) -> blake3::Hash {
    blake3::hash(key.as_bytes())
}

fn mk_cipher(key: blake3::Hash) -> ChaCha20Poly1305 {
    let b: &[u8;32] = key.as_bytes();
    let k = Key::from_slice(b); // 256-bit secret key (32 bytes)
    ChaCha20Poly1305::new(k)
}

fn encrypt(cipher: &ChaCha20Poly1305, nonce: &Nonce, plaintext: &Vec<u8>) -> Result<Vec<u8>, chacha20poly1305::aead::Error> {
    cipher.encrypt(&nonce, plaintext.as_slice())
}

fn decrypt(cipher: &ChaCha20Poly1305, nonce: &Nonce, ciphertext: Vec<u8>) -> Result<Vec<u8>, chacha20poly1305::aead::Error> {
    cipher.decrypt(&nonce, ciphertext.as_ref())
}

#[test]
fn test_encrypt_decrypt() {
    let nonce = mk_nonce();
    let s_key = "my key";
    // Create two ciphers to test that recreating the cipher reliably works
    let cipher1 = mk_cipher(hash_key(s_key));
    let cipher2 = mk_cipher(hash_key(s_key));
    let plaintext = String::from("my plaintext message").into_bytes();
    let ciphertext = encrypt(&cipher1, &nonce, &plaintext).unwrap();
    let decrypted_plaintext = decrypt(&cipher2, &nonce, ciphertext).unwrap();
    assert_eq!(plaintext, decrypted_plaintext);
}

#[test]
fn test_encrypt_decrypt_diff_key_fails() {
    let nonce = mk_nonce();
    let cipher1 = mk_cipher(hash_key("key1"));
    let cipher2 = mk_cipher(hash_key("key2"));
    let plaintext = String::from("my plaintext message").into_bytes();
    let ciphertext = encrypt(&cipher1, &nonce, &plaintext).unwrap();
    let result = decrypt(&cipher2, &nonce, ciphertext);
    assert!(result.is_err());
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = get_program_input();

    if config.launch_web {
        rocket::ignite().mount("/", routes![index]).launch();
    }
    Ok(())
}
