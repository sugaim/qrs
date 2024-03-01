// -----------------------------------------------------------------------------
// FiniteDiffMethod
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case"),
    schemars(description = "Finite difference method")
)]
pub enum FiniteDiffMethod {
    /// Forward difference
    Forward,

    /// Backward difference
    Backward,

    /// Central difference
    Central,
}

//
// display, serde
//
impl std::fmt::Display for FiniteDiffMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FiniteDiffMethod::Forward => write!(f, "forward"),
            FiniteDiffMethod::Backward => write!(f, "backward"),
            FiniteDiffMethod::Central => write!(f, "central"),
        }
    }
}

//
// construction
//
impl std::str::FromStr for FiniteDiffMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "forward" => Ok(FiniteDiffMethod::Forward),
            "backward" => Ok(FiniteDiffMethod::Backward),
            "central" => Ok(FiniteDiffMethod::Central),
            _ => Err(anyhow::anyhow!("Invalid FiniteDiffMethod: {}", s)),
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", FiniteDiffMethod::Forward), "forward");
        assert_eq!(format!("{}", FiniteDiffMethod::Backward), "backward");
        assert_eq!(format!("{}", FiniteDiffMethod::Central), "central");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "forward".parse::<FiniteDiffMethod>().unwrap(),
            FiniteDiffMethod::Forward
        );
        assert_eq!(
            "backward".parse::<FiniteDiffMethod>().unwrap(),
            FiniteDiffMethod::Backward
        );
        assert_eq!(
            "central".parse::<FiniteDiffMethod>().unwrap(),
            FiniteDiffMethod::Central
        );
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_serialize() {
        let json = serde_json::to_string(&FiniteDiffMethod::Forward).unwrap();
        assert_eq!(json, r#""forward""#);
        let json = serde_json::to_string(&FiniteDiffMethod::Backward).unwrap();
        assert_eq!(json, r#""backward""#);
        let json = serde_json::to_string(&FiniteDiffMethod::Central).unwrap();
        assert_eq!(json, r#""central""#);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_deserialize() {
        let method = serde_json::from_str::<FiniteDiffMethod>(r#""forward""#).unwrap();
        assert_eq!(method, FiniteDiffMethod::Forward);
        let method = serde_json::from_str::<FiniteDiffMethod>(r#""backward""#).unwrap();
        assert_eq!(method, FiniteDiffMethod::Backward);
        let method = serde_json::from_str::<FiniteDiffMethod>(r#""central""#).unwrap();
        assert_eq!(method, FiniteDiffMethod::Central);

        // error
        let method = serde_json::from_str::<FiniteDiffMethod>(r#""forward ""#);
        assert!(method.is_err());
        let method = serde_json::from_str::<FiniteDiffMethod>(r#""backward ""#);
        assert!(method.is_err());
        let method = serde_json::from_str::<FiniteDiffMethod>(r#""central ""#);
        assert!(method.is_err());
    }
}
