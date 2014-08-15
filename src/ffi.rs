#![allow(dead_code, missing_doc, uppercase_variables)]

use libc::{c_void, c_int, c_long, size_t, c_float, c_double, c_char};

// There's a lot of leakage from libvorbis into libvorbisfile.
// TODO expose both libvorbis and libvorbisfile APIs.

#[link(name="vorbisfile")]
extern "C" {
    pub fn ov_open_callbacks(datasource: *mut c_void, vf: *mut OggVorbis_File,
                         initial: *mut c_char, ibytes: c_long,
                         callbacks: ov_callbacks) -> c_int;

    pub fn ov_read_float(vf: *mut OggVorbis_File, pcm_channels: *mut *mut *mut c_float,
                         samples: c_int, bitstream: *mut c_int) -> c_long;

    pub fn ov_info(vf: *mut OggVorbis_File, link: c_int) -> *const vorbis_info;

    pub fn ov_clear(vf: *mut OggVorbis_File) -> c_int;

    pub fn ov_comment(vf: *mut OggVorbis_File, link: c_int) -> *const vorbis_comment;
}

pub static OV_FALSE: c_int = -1;
pub static OV_HOLE: c_int = -3;
pub static OV_EREAD: c_int = -128;
pub static OV_EFAULT: c_int = -129;
pub static OV_EIMPL: c_int = -130;
pub static OV_EINVAL: c_int = -131;
pub static OV_ENOTVORBIS: c_int = -132;
pub static OV_EBADHEADER: c_int = -133;
pub static OV_EVERSION: c_int = -134;
pub static OV_ENOTAUDIO: c_int = -135;
pub static OV_EBADPACKET: c_int = -136;
pub static OV_EBADLINK: c_int = -137;
pub static OV_ENOSEEK: c_int = -138;

#[repr(C)]
pub struct OggVorbis_File {
    pub datasource: *mut c_void,
    seekable: c_int,
    offset: i64,
    end: i64,
    oy: ogg_sync_state,

    links: int,
    offsets: *mut i64,
    dataoffsets: *mut i64,
    serialnos: *mut c_long,
    pcmlengths: *mut i64,
    vi: *mut vorbis_info,
    vc: *mut vorbis_comment,

    pcm_offset: i64,
    ready_state: c_int,
    current_serialno: c_long,
    current_link: c_int,

    bittrack: c_double,
    samptrack: c_double,

    os: ogg_stream_state,
    vd: vorbis_dsp_state,
    vb: vorbis_block,

    callbacks: ov_callbacks,
}

#[repr(C)]
pub struct ov_callbacks {
    pub read: extern "C" fn(ptr: *mut c_void, size: size_t, nmemb: size_t, datasource: *mut c_void) -> size_t,
    pub seek: extern "C" fn(datasource: *mut c_void, offset: i64, whence: c_int) -> c_int,
    pub close: extern "C" fn(datasource: *mut c_void) -> c_int,
    pub tell: extern "C" fn(datasource: *mut c_void) -> c_long,
}

#[repr(C)]
pub struct vorbis_block {
    pcm: *mut *mut c_float,
    opb: oggpack_buffer,

    lW: c_long,
    W: c_long,
    nW: c_long,
    pcmend: c_int,
    mode: c_int,

    eofflag: c_int,
    granulepos: i64,
    sequence: i64,
    vd: *const vorbis_dsp_state,

    localstore: *mut c_void,
    localtop: c_long,
    localalloc: c_long,
    totaluse: c_long,
    read: *mut alloc_chain,

    glue_bits: c_long,
    time_bits: c_long,
    floor_bits: c_long,
    res_bits: c_long,

    internal: *mut c_void,
}

#[repr(C)]
pub struct alloc_chain {
    ptr: *mut c_void,
    next: *mut alloc_chain,
}

#[repr(C)]
pub struct vorbis_comment {
    pub user_comments: *mut *mut c_char,
    pub comment_lengths: *mut c_int,
    pub comments: c_int,
    pub vendor: *mut c_char,
}

#[repr(C)]
pub struct ogg_sync_state {
    data: *mut c_char,
    storage: c_int,
    fill: c_int,
    returned: c_int,

    unsynced: c_int,
    headerbytes: c_int,
    bodybytes: c_int,
}

#[repr(C)]
pub struct ogg_stream_state {
    body_data: *mut c_char,
    body_storage: c_long,
    body_fill: c_long,
    body_returned: c_long,

    lacing_vals: *mut c_int,
    granule_vals: *mut i64,
    
    lacing_storage: c_long,
    lacing_fill: c_long,
    lacing_packet: c_long,
    lacing_returned: c_long,

    header: [c_char, ..282],
    header_fill: c_int,

    e_o_s: c_int,
    b_o_s: c_int,

    serialno: c_long,
    pageno: c_long,
    packetno: i64,
    granulepos: i64,
}

#[repr(C)]
pub struct oggpack_buffer {
    endbyte: c_long,
    endbit: c_int,

    buffer: *mut c_char,
    ptr: *mut c_char,
    storage: c_long,
}

#[repr(C)]
pub struct vorbis_info {
    pub version: c_int,
    pub channels: c_int,
    pub rate: c_long,

    pub bitrate_upper: c_long,
    pub bitrate_nominal: c_long,
    pub bitrate_lower: c_long,
    pub bitrate_window: c_long,

    codec_setup: *mut c_void,
}

#[repr(C)]
pub struct vorbis_dsp_state {
    analysisp: c_int,
    vi: *mut vorbis_info,

    pcm: *mut *mut c_float,
    pcmret: *mut *mut c_float,
    pcm_storage: c_int,
    pcm_currenet: c_int,
    pcm_returned: c_int,

    preextrapolate: c_int,
    eofflag: c_int,

    lW: c_long,
    W: c_long,
    nW: c_long,
    centerW: c_long,

    granulepos: i64,
    sequence: i64,

    glue_bits: i64,
    time_bits: i64,
    floor_bits: i64,
    res_bits: i64,

    backend_state: *mut c_void,
}

