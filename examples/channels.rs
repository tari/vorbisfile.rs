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
            println!("Input has {} channel{}", channels.len(),
                     if channels.len() != 1 {
                         "s"
                     } else {
                         ""
                     });
        }
        Err(EndOfStream) => {
            return;
        },
        Err(e) => {
            println!("Error decoding input: {}", e);
            return;
        }
    }
}
