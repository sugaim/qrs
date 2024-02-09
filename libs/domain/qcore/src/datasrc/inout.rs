use chrono::DateTime;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// RequestType
//

/// Request type for cacheable data source
///
/// - `Newly`: For the first time
/// - `Again`: If you want to consider the data is updated or not
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ReqType {
    Newly,
    Again {
        prev: DateTime<chrono::Utc>,
        force_return: bool,
    },
}

// -----------------------------------------------------------------------------
// Output
//

/// Output template for cacheable data source
///
/// - `WithData`: Data with its timestamp
/// - `NotUpdated`: Indicates that the data is not updated
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Output<T> {
    WithData {
        data: T,
        timestamp: DateTime<chrono::Utc>,
    },
    NotUpdated,
}
