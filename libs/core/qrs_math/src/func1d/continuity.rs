#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// SemiContinuity
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "snake_case"),
    derive(JsonSchema),
    schemars(description = "Semi continuity")
)]
pub enum SemiContinuity {
    /// `f(x-) = f(x)`
    ///
    /// ```txt
    ///           o---------
    ///           |
    /// ----------x
    /// ```
    ///
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "Left continuity: f(x-) = f(x)")
    )]
    LeftContinuous,

    /// `f(x) = f(x+)`
    ///
    /// ```txt
    ///           x---------
    ///           |
    /// ----------o
    /// ```
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "Right continuity: f(x) = f(x+)")
    )]
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

//
// construction
//
impl std::str::FromStr for SemiContinuity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left_continuous" => Ok(SemiContinuity::LeftContinuous),
            "right_continuous" => Ok(SemiContinuity::RightContinuous),
            _ => anyhow::bail!("Invalid semi continuity: {}", s),
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
    #[cfg(feature = "serde")]
    fn test_serialize() {
        let json = serde_json::to_string(&SemiContinuity::LeftContinuous).unwrap();
        assert_eq!(json, r#""left_continuous""#);
        let json = serde_json::to_string(&SemiContinuity::RightContinuous).unwrap();
        assert_eq!(json, r#""right_continuous""#);
    }

    #[test]
    #[cfg(feature = "serde")]
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
