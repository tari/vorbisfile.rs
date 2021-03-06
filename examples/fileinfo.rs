extern crate vorbisfile;

use vorbisfile::{VorbisFile, OVError};
use std::error::Error;
use std::io;

fn main() {
    let mut vf = match VorbisFile::new(io::stdin()) {
        Ok(f) => f,
        Err(e) => {
            println!("Error opening input file: {}", e.description());
            return;
        }
    };
    let mut prev_channels = 0;

    match vf.comment(-1) {
        Some(mut comments) => {
            println!("Encoded by {}", comments.vendor);
            println!("File comments:");
            comments.comments.sort();
            for comment in comments.comments.iter() {
                match comment.find('=') {
                    None => {
                        println!("\t {}", comment);
                    }
                    Some(i) => {
                        println!("\t {}: {}", &comment[..i],
                                 if i < comment.len() {
                                     &comment[i + 1..]
                                 } else {
                                     ""
                                 });
                    }
                }
            }
        }
        None => {
            println!("Failed to get stream comments.");
        }
    }

    loop {
        let res = vf.decode();
        match res {
            Ok(ref channels) if channels.len() != prev_channels => {
                println!("Input has {} channel{}", channels.len(),
                         if channels.len() != 1 {
                             "s"
                         } else {
                             ""
                         });
                prev_channels = channels.len();
            }
            Ok(_) => {}
            Err(OVError::EndOfStream) => {
                return;
            },
            Err(e) => {
                println!("Error decoding input: {}", e.description());
                return;
            }
        }
    }
}
