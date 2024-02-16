use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// SemiContinuity
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SemiContinuity {
    /// `f(x-) = f(x)`
    ///
    /// ```txt
    ///           o---------
    ///           |
    /// ----------x
    /// ```
    ///
    #[schemars(description = "Left continuity: f(x-) = f(x)")]
    LeftContinuous,

    /// `f(x) = f(x+)`
    ///
    /// ```txt
    ///           x---------
    ///           |
    /// ----------o
    /// ```
    #[schemars(description = "Right continuity: f(x) = f(x+)")]
    RightContinuous,
}

//
// display, serde
//
impl std::fmt::Display for SemiContinuity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemiContinuity::LeftContinuous => write!(f, "left_continuous"),
            SemiContinuity::RightContinuous => write!(f, "right_continuous"),
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(
            SemiContinuity::LeftContinuous.to_string(),
            "left_continuous"
        );
        assert_eq!(
            SemiContinuity::RightContinuous.to_string(),
            "right_continuous"
        );
    }

    #[test]
    fn test_serialize() {
        let json = serde_json::to_string(&SemiContinuity::LeftContinuous).unwrap();
        assert_eq!(json, r#""left_continuous""#);
        let json = serde_json::to_string(&SemiContinuity::RightContinuous).unwrap();
        assert_eq!(json, r#""right_continuous""#);
    }

    #[test]
    fn test_deserialize() {
        let semi = serde_json::from_str::<SemiContinuity>(r#""left_continuous""#).unwrap();
        assert_eq!(semi, SemiContinuity::LeftContinuous);
        let semi = serde_json::from_str::<SemiContinuity>(r#""right_continuous""#).unwrap();
        assert_eq!(semi, SemiContinuity::RightContinuous);

        // error
        let semi = serde_json::from_str::<SemiContinuity>(r#""left_continuous_""#);
        assert!(semi.is_err());

        let semi = serde_json::from_str::<SemiContinuity>(r#""right_continuous_""#);
        assert!(semi.is_err());

        let semi = serde_json::from_str::<SemiContinuity>(r#""LeftContinuous""#);
        assert!(semi.is_err());

        let semi = serde_json::from_str::<SemiContinuity>(r#""RightContinuous""#);
        assert!(semi.is_err());
    }
}
