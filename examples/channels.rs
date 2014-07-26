extern crate libvorbisfile;

use libvorbisfile::{VorbisFile, EndOfStream};
use std::io;

fn main() {
    let mut vf = match VorbisFile::new(io::stdin()) {
        Ok(f) => f,
        Err(e) => {
            println!("Error opening file: {}", e);
            return;
        }
    };

    let res = vf.decode();
    match res {
        Ok(channels) => {
            println!("{} channels x {} samples", channels.len(), channels[0].len());
        }
        Err(EndOfStream) => {
            return;
        },
        Err(e) => {
            println!("Execpected decode error: {}", e);
            return;
        }
    }
}
