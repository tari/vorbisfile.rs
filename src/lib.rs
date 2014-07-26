#![crate_name = "libvorbisfile"]
#![doc(html_root_url = "http://www.rust-ci.org/tari/vorbisfile.rs/doc/libvorbisfile/")]

#![deny(dead_code, missing_doc)]

//! Ogg Vorbis file decoding, library bindings.

extern crate debug;
extern crate libc;
use libc::{c_void, c_int, c_long, size_t};

use std::mem;
use std::ptr;
use std::raw;
use std::slice::raw::mut_buf_as_slice;

mod ffi;

pub type OVResult<T> = Result<T, OVError>;

/// Decode error.
#[deriving(Show, Clone)]
pub enum OVError {
    /// Reached end of file.
    EndOfStream,
    /// Encounted missing or corrupt data.
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

impl OVError {
    fn from_native(code: c_int) -> OVError {
        match code {
            ffi::OV_HOLE => StreamInterrupted,
            ffi::OV_EREAD => ReadError,
            ffi::OV_EFAULT => InternalFault,
            ffi::OV_EIMPL => NotImplemented,
            ffi::OV_EINVAL => InvalidArgument,
            ffi::OV_ENOTVORBIS => NotVorbis,
            ffi::OV_EBADHEADER => InvalidHeader,
            ffi::OV_EVERSION => UnsupportedVersion,
            ffi::OV_EBADLINK => CorruptLink,
            ffi::OV_ENOSEEK => NotSeekable,
            x => fail!("Unexpected OVError code: {}", x)
        }
    }
}

/// Ogg Vorbis file decoder.
pub struct VorbisFile {
    // The FFI member holds a pointer to this box.
    // Use a trait object because generic foreign functions are not allowed.
    src: Box<Reader>,
    decoder: ffi::OggVorbis_File,
    // Totally not 'static, but need a lifetime specifier to get a slice.
    channels: Vec<&'static [f32]>,
}

/// Read `nmemb` items into `ptr` of `size` bytes each.
/// 
/// If 0 is returned, error status is implied by errno. If nonzero, there was
/// a read error. Otherwise, reached EOF.
/// 
/// TODO these functions should be parameterized over R in VorbisFile<R>, which obviates the need
/// for type erasure, which in currently puts us in an ugly position where we need to get a raw
/// pointer to a trait object (which doesn't work well without DSTs).
/// Depends on Rust PR #15831.
extern "C" fn read(buffer: *mut c_void, size: size_t, nmemb: size_t, datasource: *mut c_void) -> size_t {
    let src: &mut Box<Reader> = unsafe { mem::transmute(datasource) };
    let ptr = buffer as *mut u8;

    for i in range(0, nmemb) {
        let more = unsafe {
            mut_buf_as_slice(ptr.offset(i as int), size as uint, |buf| {
                match src.read_at_least(size as uint, buf) {
                    Ok(_) => true,
                    // Assume errno is set under the covers.
                    Err(_) => false
                }
            })
        };
        if !more {
            // Got error, which might be EOF.
            return i;
        }
    }
    // Completed successfully
    return nmemb;
}

#[allow(unused_variable)]
extern "C" fn seek(datasource: *mut c_void, offset: i64, whence: c_int) -> c_int {
    // TODO permit seeking
    -1
}
#[allow(unused_variable)]
extern "C" fn close(datasource: *mut c_void) -> c_int {
    // No need to do anything. VorbisFile owns the Reader we're using.
    0
}
#[allow(unused_variable)]
extern "C" fn tell(datasource: *mut c_void) -> c_long {
    // TODO permit seeking
    -1
}

// Lifetime bound on `src` is necessary because ffi::OggVorbis_File holds
// a reference to it, so it must never move as long as this is alive.
//
// Don't expose ov_fopen and friends because that won't play nicely
// with non-libnative runtime.
impl VorbisFile {
    /// Ensures the FFI struct is consistent for callback invocation.
    ///
    /// Because the user may move this struct, the `datasource` pointer
    /// passed back to FFI callbacks might be invalidated. This function
    /// should be called before FFI actions that might fire callbacks to ensure
    /// the self-pointer is valid.
    fn callback_setup(&mut self) {
        let ds = &mut self.src as *mut _ as *mut c_void;
        self.decoder.datasource = ds;
    }

    /// Create a Ogg Vorbis decoder.
    pub fn new<R: Reader+'static>(src: R) -> OVResult<VorbisFile> {
        let mut vf = VorbisFile {
            src: box src as Box<Reader>,
            decoder: unsafe { mem::uninitialized() },
            channels: Vec::new()
        };
        let callbacks = ffi::ov_callbacks {
            read: read,
            seek: seek,
            tell: tell,
            close: close,
        };

        let status = unsafe {
            ffi::ov_open_callbacks(&mut vf.src as *mut _ as *mut c_void, 
                                   &mut vf.decoder as *mut _,
                                   ptr::mut_null(), 0, callbacks)
        };

        match status {
            0 => Ok(vf),
            f => {
                // Must not run the destructor. decoder is still uninitialized.
                unsafe {
                    mem::forget(vf.decoder);
                }
                Err(OVError::from_native(f))
            }
        }
    }

    /// Decode a block of samples.
    ///
    /// The emitted values are a slice of channels, each containing an equal
    /// number of samples.
    pub fn decode<'a>(&'a mut self) -> OVResult<&'a [&'a [f32]]> {
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
                    return Err(EndOfStream);
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
        for i in range(0, n_channels) {
            unsafe {
                let channel_buffer = *sample_buffer.offset(i as int);
                let channel_slice = raw::Slice::<f32> {
                    data: channel_buffer as *const f32,
                    len: n_samples as uint
                };
                self.channels.push(mem::transmute(channel_slice));
            };
        }
        Ok(self.channels.as_slice())
    }
}
