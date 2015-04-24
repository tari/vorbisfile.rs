#![doc(html_root_url = "http://www.rust-ci.org/tari/vorbisfile.rs/doc/libvorbisfile/")]

#![deny(dead_code, missing_docs)]
#![feature(core)]

//! Ogg Vorbis file decoding, library bindings.

extern crate libc;
use libc::{c_void, c_int, c_long, size_t};

use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::io::Read;
use std::mem;
use std::str;
use std::ptr;
use std::raw;
use std::slice::from_raw_parts_mut;

#[allow(dead_code, non_snake_case)]
mod ffi;

/// `OVError` or `T`.
pub type OVResult<T> = Result<T, OVError>;

/// Decode error.
#[derive(Debug, Clone)]
pub enum OVError {
    /// Reached end of file.
    EndOfStream,
    /// Encountered missing or corrupt data.
    ///
    /// Recovery from this error is usually automatic and is returned for
    /// informational purposes only.
    StreamInterrupted,
    /// I/O error while reading compressed data to decode.
    ReadError,
    /// Internal inconsistency in encode or decode state. Recovery impossible.
    InternalFault,
    /// Feature not implemented.
    NotImplemented,
    /// User passed an invalid argument to a function.
    InvalidArgument,
    /// Provided data is not recognized as Ogg Vorbis.
    NotVorbis,
    /// Provided data appears to be Ogg Vorbis but has a corrupt or
    /// indecipherable header.
    InvalidHeader,
    /// Bitstream format revision is not supported.
    UnsupportedVersion,
    /// The specified Vorbis link exists but is corrupt.
    CorruptLink,
    /// The stream is not seekable.
    NotSeekable,
}

impl Error for OVError {
    fn description(&self) -> &str {
        match *self {
            OVError::EndOfStream => "End of stream",
            OVError::StreamInterrupted => "Stream interrupted",
            OVError::ReadError => "Read error",
            OVError::InternalFault => "Internal library fault",
            OVError::NotImplemented => "Feature not implemented",
            OVError::InvalidArgument => "Invalid argument",
            OVError::NotVorbis => "Not a Vorbis stream",
            OVError::InvalidHeader => "Invalid Vorbis header",
            OVError::UnsupportedVersion => "Bitstream format revision not supported",
            OVError::CorruptLink => "Vorbis link is corrupt",
            OVError::NotSeekable => "Not seekable",
        }
    }
}

impl fmt::Display for OVError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(self.description())
    }
}

impl Copy for OVError { }

impl OVError {
    fn from_native(code: c_int) -> OVError {
        match code {
            ffi::OV_HOLE => OVError::StreamInterrupted,
            ffi::OV_EREAD => OVError::ReadError,
            ffi::OV_EFAULT => OVError::InternalFault,
            ffi::OV_EIMPL => OVError::NotImplemented,
            ffi::OV_EINVAL => OVError::InvalidArgument,
            ffi::OV_ENOTVORBIS => OVError::NotVorbis,
            ffi::OV_EBADHEADER => OVError::InvalidHeader,
            ffi::OV_EVERSION => OVError::UnsupportedVersion,
            ffi::OV_EBADLINK => OVError::CorruptLink,
            ffi::OV_ENOSEEK => OVError::NotSeekable,
            x => panic!("Unexpected OVError code: {}", x)
        }
    }
}

/// Ogg Vorbis file decoder.
pub struct VorbisFile<R: Read> {
    src: R,
    decoder: ffi::OggVorbis_File,
    // Totally not 'static, but need a lifetime specifier to get a slice.
    channels: Vec<raw::Slice<f32>>,
}

/// File metadata
pub struct Comments<'a> {
    /// The Vorbis implementation that encoded the stream.
    pub vendor: &'a str,
    /// User-specified key-value pairs of the form KEY=VALUE.
    pub comments: Vec<&'a str>
}

#[allow(unused_variables)]
extern "C" fn seek(datasource: *mut c_void, offset: i64, whence: c_int) -> c_int {
    // TODO permit seeking
    -1
}
#[allow(unused_variables)]
extern "C" fn close(datasource: *mut c_void) -> c_int {
    // No need to do anything. VorbisFile owns the Reader we're using.
    0
}
#[allow(unused_variables)]
extern "C" fn tell(datasource: *mut c_void) -> c_long {
    // TODO permit seeking
    -1
}

// Don't expose ov_fopen and friends because that won't play nicely
// with non-libnative runtime.
impl<R: Read> VorbisFile<R> {
    /// Ensures the FFI struct is consistent for callback invocation.
    ///
    /// Because the user may move this struct, the `datasource` pointer
    /// passed back to FFI callbacks might be invalidated. This function
    /// should be called before FFI actions that might fire callbacks to ensure
    /// the self-pointer is valid.
    fn callback_setup(&mut self) {
        let ds = self as *mut _ as *mut c_void;
        self.decoder.datasource = ds;
    }

    /// Create a Ogg Vorbis decoder.
    pub fn new(src: R) -> OVResult<VorbisFile<R>> {
        let mut vf = VorbisFile {
            src: src,
            decoder: unsafe { mem::uninitialized() },
            channels: Vec::new()
        };
        let callbacks = ffi::ov_callbacks {
            read: VorbisFile::<R>::read,
            seek: seek,
            tell: tell,
            close: close,
        };

        let status = unsafe {
            ffi::ov_open_callbacks(&mut vf.src as *mut _ as *mut c_void, 
                                   &mut vf.decoder,
                                   ptr::null_mut(), 0, callbacks)
        };

        match status {
            0 => Ok(vf),
            f => {
                // Must not run the destructor. decoder is still uninitialized.
                // XXX if VorbisFile's Drop impl does more than freeing self.decoder,
                // this must also be updated.
                unsafe {
                    mem::forget(vf);
                }
                Err(OVError::from_native(f))
            }
        }
    }

    /// Gets the comment struct for the specified bitstream.
    ///
    /// For nonseekable streams, returns the comments for the current
    /// bitstream. Otherwise, specify bitstream -1 to get the current
    /// bitstream.
    pub fn comment<'a>(&'a mut self, link: isize) -> Option<Comments<'a>> {
        let cm = unsafe {
            match ffi::ov_comment(&mut self.decoder, link as c_int).as_ref() {
                Some(r) => r,
                None => return None
            }
        };

        unsafe fn make_str<'a>(data: *const u8, len: usize) -> Option<&'a str> {
            let slice = raw::Slice {
                data: data,
                len: len
            };
            str::from_utf8(mem::transmute(slice)).ok()
        }

        Some(Comments {
            vendor: unsafe {
                match str::from_utf8(CStr::from_ptr(cm.vendor).to_bytes()) {
                    Ok(x) => x,
                    Err(_) => "<INVALID UTF-8>"
                }
            },
            comments: unsafe {
                // Collect user comments, ignoring ones that are invalid UTF-8.
                // These are length-prefixed (not C strings).
                let mut v = Vec::with_capacity(cm.comments as usize);
                for i in 0..(*cm).comments {
                    let len = *cm.comment_lengths.offset(i as isize);
                    match make_str(*cm.user_comments.offset(i as isize) as *const _,
                                   len as usize) {
                        Some(s) => {
                            v.push(s);
                        }
                        None => {
                            // Ignore. Vorbis specifies all comment data is valid
                            // UTF-8, but we need to protect against invalid input
                        }
                    }
                }
                v
            }
        })
    }

    /// Decode a block of samples.
    ///
    /// The emitted values are a slice of channels, each containing an equal
    /// number of samples.
    pub fn decode<'a>(&'a mut self) -> OVResult<&'a mut [&'a mut [f32]]> {
        let max_samples = 4096;
        self.callback_setup();
        let mut sample_buffer: *mut *mut f32 = unsafe {
            mem::uninitialized()
        };
        let mut bitstream_idx: c_int = unsafe {
            mem::uninitialized()
        };
        
        let n_samples = unsafe {
            match ffi::ov_read_float(&mut self.decoder, &mut sample_buffer,
                                     max_samples, &mut bitstream_idx) {
                0 => {
                    return Err(OVError::EndOfStream);
                }
                x if x < 0 => {
                    return Err(OVError::from_native(x as c_int));
                }
                x => x
            }
        };
        let n_channels = unsafe {
            (*ffi::ov_info(&mut self.decoder, bitstream_idx)).channels
        };

        self.channels.truncate(0);
        for i in 0..n_channels {
            unsafe {
                let channel_buffer = *sample_buffer.offset(i as isize);
                let channel_slice = raw::Slice::<f32> {
                    data: channel_buffer,
                    len: n_samples as usize
                };
                self.channels.push(channel_slice);
            };
        }
        Ok(unsafe {
            mem::transmute(&self.channels[..])
        })
    }

    /// Read `nmemb` items into `ptr` of `size` bytes each.
    /// 
    /// If 0 is returned, error status is implied by errno. If nonzero, there was
    /// a read error. Otherwise, reached EOF.
    extern "C" fn read(buffer: *mut c_void, size: size_t, nmemb: size_t,
                       datasource: *mut c_void) -> size_t {
        let vf: *mut VorbisFile<R> = unsafe { mem::transmute(datasource) };
        let ptr = buffer as *mut u8;

        for i in 0..nmemb {
            let more = unsafe {
                let bufp = ptr.offset(i as isize);
                let buf = from_raw_parts_mut(bufp, size as usize);
                match (*vf).src.by_ref().take(size).read(buf) {
                    Ok(n) if n == (size as usize) => true,
                    // Assume errno is set under the covers in the Err case. Ok(0) is EOF.
                    Ok(0) | Err(_) => false,
                    // Stupid hack for partial reads: recurse for the rest of this element
                    Ok(n) => {
                        // This call forces us to make `vf` a raw pointer, because we're aliasing
                        // `datasource` to `vf` so the mutable refs must not alias.
                        VorbisFile::<R>::read(bufp as *mut _, size - n as size_t, 1, datasource);
                        true
                    }
                }
            };
            if !more {
                // Hit EOF or I/O error. Don't even attempt futher reads
                return i;
            }
        }
        // Completed successfully
        return nmemb;
    }
}

impl<R: Read> Drop for VorbisFile<R> {
    fn drop(&mut self) {
        self.callback_setup();
        unsafe {
            ffi::ov_clear(&mut self.decoder);
        }
    }
}
