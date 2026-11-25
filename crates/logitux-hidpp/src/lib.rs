pub mod device;
pub mod error;
pub mod features;
pub mod packet;

pub use device::{DeviceSummary, HidppDevice};
pub use error::{Hidpp20ErrorCode, HidppError, Result};
pub use features::{feature_name, FeatureEntry, FeatureId};
pub use packet::{
    HidppPacket, Transport, DEFAULT_SW_ID, DEVICE_INDEX_WIRED, REPORT_LONG, REPORT_SHORT,
    REPORT_VERY_LONG,
};
