use qfincore::Ccy;

// -----------------------------------------------------------------------------
// Collateral
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Collateral {
    Ccy(Ccy),
}

//
// ser/de
//
impl serde::Serialize for Collateral {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        col_serde::Collateral::from(self.clone()).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Collateral {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        col_serde::Collateral::deserialize(deserializer).map(Into::into)
    }
}

impl schemars::JsonSchema for Collateral {
    fn schema_name() -> String {
        "Collateral".to_string()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qproduct::Collateral".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <col_serde::Collateral as schemars::JsonSchema>::json_schema(gen)
    }
}

mod col_serde {
    use qfincore::Ccy;

    #[derive(
        Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
    )]
    pub(super) enum Collateral {
        Ccy { ccy: Ccy },
    }

    impl From<Collateral> for super::Collateral {
        #[inline]
        fn from(src: Collateral) -> Self {
            match src {
                Collateral::Ccy { ccy } => super::Collateral::Ccy(ccy),
            }
        }
    }
    impl From<super::Collateral> for Collateral {
        #[inline]
        fn from(src: super::Collateral) -> Self {
            match src {
                super::Collateral::Ccy(ccy) => Collateral::Ccy { ccy },
            }
        }
    }
}
