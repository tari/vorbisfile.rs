extern crate libvorbisfile;

use libvorbisfile::{VorbisFile, EndOfStream};
use std::io;

fn main() {
    let mut vf = match VorbisFile::new(io::stdin()) {
        Ok(f) => f,
        Err(e) => {
            println!("Error opening input file: {}", e);
            return;
        }
    };
    let mut prev_channels = 0;

    loop {
        let res = vf.decode();
        match res {
            Ok(channels) if channels.len() != prev_channels => {
                println!("Input has {} channel{}", channels.len(),
                         if channels.len() != 1 {
                             "s"
                         } else {
                             ""
                         });
                prev_channels = channels.len();
            }
            Ok(_) => {}
            Err(EndOfStream) => {
                return;
            },
            Err(e) => {
                println!("Error decoding input: {}", e);
                return;
            }
        }
    }
}
