//! Error type for the ROS bag reader, built on [`tpt_io_core::ParseError`].

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!("tpt-io-rosbag requires the `alloc` or `std` feature: its error type owns text.");

use tpt_io_core::ParseError;

/// Errors produced while reading or writing a ROS bag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RosbagError {
    /// A structural error, with its byte offset.
    Core(ParseError),
    /// A feature this reader does not implement, such as a compressed chunk.
    Unsupported(&'static str),
}

impl From<ParseError> for RosbagError {
    fn from(e: ParseError) -> Self {
        RosbagError::Core(e)
    }
}

impl core::fmt::Display for RosbagError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RosbagError::Core(e) => write!(f, "{e}"),
            RosbagError::Unsupported(what) => write!(f, "unsupported ROS bag feature: {what}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RosbagError {}
