use std::ops::Drop;
use std::os::raw::c_int;
use std::ptr;
use std::{convert::From, fmt::Display};

#[derive(Debug)]
pub enum Error {
    Ok,
    GenericError,
    NoMem,
    BadBitRate,
    BadSampleFreq,
    InternalError,
    Unknown(c_int),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Ok => "No problem",
            Error::GenericError => "Generic error",
            Error::NoMem => "No memory",
            Error::BadBitRate => "Bad bitrate",
            Error::BadSampleFreq => "Bad sample frequency",
            Error::InternalError => "Internal error",
            Error::Unknown(_) => "Unknown error",
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<c_int> for Error {
    fn from(errcode: c_int) -> Error {
        match errcode {
            0 => Error::Ok,
            -1 => Error::GenericError,
            -10 => Error::NoMem,
            -11 => Error::BadBitRate,
            -12 => Error::BadSampleFreq,
            -13 => Error::InternalError,
            _ => Error::Unknown(errcode),
        }
    }
}

fn handle_simple_error(retn: c_int) -> Result<(), Error> {
    match retn.into() {
        Error::Ok => Ok(()),
        err => Err(err),
    }
}

fn int_size(sz: usize) -> c_int {
    if sz > c_int::max_value() as usize {
        panic!("converting {} to c_int would overflow");
    }

    sz as c_int
}

#[derive(Debug)]
pub enum EncodeError {
    OutputBufferTooSmall,
    NoMem,
    InitParamsNotCalled,
    PsychoAcousticError,
    Unknown(c_int),
}

impl std::error::Error for EncodeError {
    fn description<'a>(&'a self) -> &'a str {
        match *self {
            EncodeError::OutputBufferTooSmall => "Output buffer too small",
            EncodeError::NoMem => "No memory",
            EncodeError::InitParamsNotCalled => "Init params not called",
            EncodeError::PsychoAcousticError => "Psycho acoustic error",
            EncodeError::Unknown(_) => "Unknown",
        }
    }
}

impl Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Represents a Lame encoder context.
pub struct Lame {
    ptr: *mut lame_sys::lame_global_flags,
}

impl Lame {
    /// Creates a new Lame encoder context with default parameters.
    ///
    /// Returns None if liblame could not allocate its internal structures.
    pub fn new() -> Option<Lame> {
        let ctx = unsafe { lame_sys::lame_init() };

        if ctx == ptr::null_mut() {
            None
        } else {
            Some(Lame { ptr: ctx })
        }
    }

    /// Sample rate of input PCM data. Defaults to 44100 Hz.
    pub fn sample_rate(&self) -> u32 {
        unsafe { lame_sys::lame_get_in_samplerate(self.ptr) as u32 }
    }

    /// Sets sample rate of input PCM data.
    pub fn set_sample_rate(&mut self, sample_rate: u32) -> Result<(), Error> {
        handle_simple_error(unsafe {
            lame_sys::lame_set_in_samplerate(self.ptr, sample_rate as c_int)
        })
    }

    /// Number of channels in input stream. Defaults to 2.
    pub fn channels(&self) -> u8 {
        unsafe { lame_sys::lame_get_num_channels(self.ptr) as u8 }
    }

    /// Sets number of channels in input stream.
    pub fn set_channels(&mut self, channels: u8) -> Result<(), Error> {
        handle_simple_error(unsafe { lame_sys::lame_set_num_channels(self.ptr, channels as c_int) })
    }

    /// LAME quality parameter. See `set_quality` for more details.
    pub fn quality(&self) -> u8 {
        unsafe { lame_sys::lame_get_quality(self.ptr) as u8 }
    }

    /// Sets LAME's quality parameter. True quality is determined by the
    /// bitrate but this parameter affects quality by influencing whether LAME
    /// selects expensive or cheap algorithms.
    ///
    /// This is a number from 0 to 9 (inclusive), where 0 is the best and
    /// slowest and 9 is the worst and fastest.
    pub fn set_quality(&mut self, quality: u8) -> Result<(), Error> {
        handle_simple_error(unsafe { lame_sys::lame_set_quality(self.ptr, quality as c_int) })
    }

    /// Returns the output bitrate in kilobits per second.
    pub fn kilobitrate(&self) -> i32 {
        unsafe { lame_sys::lame_get_brate(self.ptr) as i32 }
    }

    /// Sets the target output bitrate. This value is in kilobits per second,
    /// so passing 320 would select an output bitrate of 320kbps.
    pub fn set_kilobitrate(&mut self, quality: i32) -> Result<(), Error> {
        handle_simple_error(unsafe { lame_sys::lame_set_brate(self.ptr, quality as c_int) })
    }

    /// Sets more internal parameters according to the other basic parameter
    /// settings.
    pub fn init_params(&mut self) -> Result<(), Error> {
        handle_simple_error(unsafe { lame_sys::lame_init_params(self.ptr) })
    }

    /// Encodes PCM data into MP3 frames. The `pcm_left` and `pcm_right`
    /// buffers must be of the same length, or this function will panic.
    pub fn encode(
        &mut self,
        pcm_left: &mut [i16],
        pcm_right: &mut [i16],
        mp3_buffer: &mut [u8],
    ) -> Result<usize, EncodeError> {
        if pcm_left.len() != pcm_right.len() {
            panic!("left and right channels must have same number of samples!");
        }

        let retn = unsafe {
            lame_sys::lame_encode_buffer(
                self.ptr,
                pcm_left.as_mut_ptr(),
                pcm_right.as_mut_ptr(),
                int_size(pcm_left.len()),
                mp3_buffer.as_mut_ptr(),
                int_size(mp3_buffer.len()),
            )
        };

        match retn {
            -1 => Err(EncodeError::OutputBufferTooSmall),
            -2 => Err(EncodeError::NoMem),
            -3 => Err(EncodeError::InitParamsNotCalled),
            -4 => Err(EncodeError::PsychoAcousticError),
            _ => {
                if retn < 0 {
                    Err(EncodeError::Unknown(retn))
                } else {
                    Ok(retn as usize)
                }
            }
        }
    }
}

impl Drop for Lame {
    fn drop(&mut self) {
        unsafe { lame_sys::lame_close(self.ptr) };
    }
}
