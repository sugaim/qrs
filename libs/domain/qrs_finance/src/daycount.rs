mod _ops;
mod act360;
mod act365f;
mod bd252;
mod nl360;
mod nl365;
mod traits;

use std::str::FromStr;

use anyhow::Context;
use qrs_chrono::{Calendar, CalendarSymbol, NaiveDate};
use qrs_datasrc::{DataSrc, DebugTree};
use qrs_math::num::Real;

pub use act360::{Act360, Act360Rate};
pub use act365f::{Act365f, Act365fRate};
pub use bd252::{Bd252, Bd252Rate};
pub use nl360::{Nl360, Nl360Rate};
pub use nl365::{Nl365, Nl365Rate};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use traits::{Dcf, DcfError, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Rate
//
#[derive(Debug, Clone, PartialEq)]
pub enum Rate<V> {
    Act360(Act360Rate<V>),
    Act365f(Act365fRate<V>),
    Nl360(Nl360Rate<V>),
    Nl365(Nl365Rate<V>),
    Bd252(Bd252Rate<V>),
}

//
// methods
//
impl<V: Real> InterestRate for Rate<V> {
    type Value = V;
    type Convention = DayCount;

    #[inline]
    fn convention(&self) -> Self::Convention {
        match self {
            Rate::Act360(_) => DayCount::Act360,
            Rate::Act365f(_) => DayCount::Act365f,
            Rate::Nl360(_) => DayCount::Nl360,
            Rate::Nl365(_) => DayCount::Nl365,
            Rate::Bd252(rate) => DayCount::Bd252(rate.convention()),
        }
    }

    #[inline]
    fn into_value(self) -> Self::Value {
        match self {
            Rate::Act360(rate) => rate.into_value(),
            Rate::Act365f(rate) => rate.into_value(),
            Rate::Nl360(rate) => rate.into_value(),
            Rate::Nl365(rate) => rate.into_value(),
            Rate::Bd252(rate) => rate.into_value(),
        }
    }
}

// -----------------------------------------------------------------------------
// DayCount
//
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DayCount {
    Act360,
    Act365f,
    Nl360,
    Nl365,
    Bd252(Bd252),
}

//
// methods
//
impl Dcf for DayCount {
    #[inline]
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Result<f64, DcfError> {
        match self {
            DayCount::Act360 => Act360.dcf(from, to),
            DayCount::Act365f => Act365f.dcf(from, to),
            DayCount::Nl360 => Nl360.dcf(from, to),
            DayCount::Nl365 => Nl365.dcf(from, to),
            DayCount::Bd252(dcf) => dcf.dcf(from, to),
        }
    }
}

impl RateDcf for DayCount {
    type Rate<V: Real> = Rate<V>;

    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        match self {
            DayCount::Act360 => Rate::Act360(Act360.to_rate(annual_rate)),
            DayCount::Act365f => Rate::Act365f(Act365f.to_rate(annual_rate)),
            DayCount::Nl360 => Rate::Nl360(Nl360.to_rate(annual_rate)),
            DayCount::Nl365 => Rate::Nl365(Nl365.to_rate(annual_rate)),
            DayCount::Bd252(dcf) => Rate::Bd252(dcf.to_rate(annual_rate)),
        }
    }
}

// -----------------------------------------------------------------------------
// DayCountSymbol
//
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DayCountSymbol {
    Act360,
    Act365f,
    Nl360,
    Nl365,
    Bd252 { cal: CalendarSymbol },
}

//
// display, serde
//
impl std::fmt::Display for DayCountSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DayCountSymbol::Act360 => write!(f, "ACT/360"),
            DayCountSymbol::Act365f => write!(f, "ACT/365F"),
            DayCountSymbol::Nl360 => write!(f, "NL/360"),
            DayCountSymbol::Nl365 => write!(f, "NL/365"),
            DayCountSymbol::Bd252 { cal } => write!(f, "BD/252[{}]", cal),
        }
    }
}

impl Serialize for DayCountSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DayCountSymbol {
    fn deserialize<D>(deserializer: D) -> Result<DayCountSymbol, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s: &str = serde::de::Deserialize::deserialize(deserializer)?;
        DayCountSymbol::from_str(s).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for DayCountSymbol {
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_finance::daycount::DayCountSymbol".into()
    }
    fn schema_name() -> String {
        "DayCountSymbol".into()
    }
    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::SchemaObject;
        let mut schema = SchemaObject::default();
        schema.metadata().description = Some("Day count symbol".to_string());
        schema.subschemas().one_of = Some(vec![
            SchemaObject {
                instance_type: Some(schemars::schema::InstanceType::String.into()),
                const_value: Some("ACT/360".to_string().into()),
                ..Default::default()
            }
            .into(),
            SchemaObject {
                instance_type: Some(schemars::schema::InstanceType::String.into()),
                const_value: Some("ACT/365F".to_string().into()),
                ..Default::default()
            }
            .into(),
            SchemaObject {
                instance_type: Some(schemars::schema::InstanceType::String.into()),
                const_value: Some("NL/360".to_string().into()),
                ..Default::default()
            }
            .into(),
            SchemaObject {
                instance_type: Some(schemars::schema::InstanceType::String.into()),
                const_value: Some("NL/365".to_string().into()),
                ..Default::default()
            }
            .into(),
            SchemaObject {
                instance_type: Some(schemars::schema::InstanceType::String.into()),
                metadata: Some(Box::new(schemars::schema::Metadata {
                    description: Some(
                        "BD/252 convention. String must be in 'BD/252[{calendar}]' format"
                            .to_string(),
                    ),
                    ..Default::default()
                })),
                string: Some(Box::new(schemars::schema::StringValidation {
                    pattern: Some(r#"^BD/252\[.*\]$"#.to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            }
            .into(),
        ]);

        schema.into()
    }
}

//
// construction
//
impl FromStr for DayCountSymbol {
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const SUPPORTED_FMT: &[&str] = &[
            "ACT/360",
            "ACT/365F",
            "NL/360",
            "NL/365",
            "BD/252[{{calendar}}]",
        ];
        match s {
            "ACT/360" => Ok(DayCountSymbol::Act360),
            "ACT/365F" => Ok(DayCountSymbol::Act365f),
            "NL/360" => Ok(DayCountSymbol::Nl360),
            "NL/365" => Ok(DayCountSymbol::Nl365),
            s if s.starts_with("BD/252[") && s.ends_with(']') => {
                let cal = s[7..(s.len() - 1)]
                    .parse()
                    .context("for BD/252 daycount convention")?;
                Ok(DayCountSymbol::Bd252 { cal })
            }
            _ => Err(anyhow::anyhow!(
                "invalid day count symbol '{s}'. Expected one of {SUPPORTED_FMT:?}",
            )),
        }
    }
}

//
// methods
//
impl DayCountSymbol {
    #[inline]
    pub fn instantinate(
        &self,
        calsrc: &impl DataSrc<CalendarSymbol, Output = Calendar>,
    ) -> anyhow::Result<DayCount> {
        let res = match self {
            DayCountSymbol::Act360 => DayCount::Act360,
            DayCountSymbol::Act365f => DayCount::Act365f,
            DayCountSymbol::Nl360 => DayCount::Nl360,
            DayCountSymbol::Nl365 => DayCount::Nl365,
            DayCountSymbol::Bd252 { cal } => DayCount::Bd252(Bd252 {
                cal: calsrc.get(cal)?,
            }),
        };
        Ok(res)
    }
}

// -----------------------------------------------------------------------------
// DayCountSrc
//
#[derive(Debug, Clone, PartialEq, Eq, DebugTree)]
#[debug_tree(desc = "day count source")]
pub struct DayCountSrc<Cal> {
    #[debug_tree(subtree)]
    cal: Cal,
}

//
// construction
//
impl<Cal> DayCountSrc<Cal> {
    #[inline]
    pub fn new(cal: Cal) -> Self {
        Self { cal }
    }
}

//
// methods
//
impl<Cal> DayCountSrc<Cal> {
    #[inline]
    pub fn inner(&self) -> &Cal {
        &self.cal
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut Cal {
        &mut self.cal
    }

    #[inline]
    pub fn into_inner(self) -> Cal {
        self.cal
    }
}

impl<Cal> DataSrc<DayCountSymbol> for DayCountSrc<Cal>
where
    Cal: DataSrc<CalendarSymbol, Output = Calendar>,
{
    type Output = DayCount;

    #[inline]
    fn get(&self, req: &DayCountSymbol) -> anyhow::Result<Self::Output> {
        req.instantinate(self.inner())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use mockall::mock;
    use qrs_datasrc::{DataSrc, DebugTree, TreeInfo};
    use rstest::rstest;

    use super::*;

    mock! {
        CalSrc {}

        impl DebugTree for CalSrc {
            fn desc(&self) -> String;
            fn debug_tree(&self) -> TreeInfo;
        }

        impl DataSrc<CalendarSymbol> for CalSrc {
            type Output = Calendar;
            fn get(&self, req: &CalendarSymbol) -> anyhow::Result<Calendar>;
        }
    }

    #[rstest]
    #[case("ACT/360", DayCountSymbol::Act360)]
    #[case("ACT/365F", DayCountSymbol::Act365f)]
    #[case("NL/360", DayCountSymbol::Nl360)]
    #[case("NL/365", DayCountSymbol::Nl365)]
    #[case("BD/252[TKY]", DayCountSymbol::Bd252 {
        cal: "TKY".parse().unwrap()
    })]
    #[case("BD/252[LDN|TKY]", DayCountSymbol::Bd252 {
        cal: "LDN|TKY".parse().unwrap()
    })]
    fn test_string(#[case] s: &str, #[case] o: DayCountSymbol) {
        // from_str
        let res = DayCountSymbol::from_str(s).unwrap();

        assert_eq!(res, o);

        // to_string
        let res = o.to_string();

        assert_eq!(res, s);
    }

    #[rstest]
    #[case(DayCountSymbol::Act360, DayCount::Act360, false)]
    #[case(DayCountSymbol::Act365f, DayCount::Act365f, false)]
    #[case(DayCountSymbol::Nl360, DayCount::Nl360, false)]
    #[case(DayCountSymbol::Nl365, DayCount::Nl365, false)]
    #[case(DayCountSymbol::Bd252 {
        cal: "TKY".parse().unwrap()
    }, DayCount::Bd252(Bd252 {
        cal: Calendar::default()
    }), true)]
    fn test_instantinate(
        #[case] sym: DayCountSymbol,
        #[case] exp: DayCount,
        #[case] call_calsrc: bool,
    ) {
        let mut calsrc = MockCalSrc::new();
        calsrc
            .expect_get()
            .returning(|_| Ok(Calendar::default()))
            .times(if call_calsrc { 1 } else { 0 });

        let res = sym.instantinate(&calsrc).unwrap();

        assert_eq!(res, exp);
        calsrc.checkpoint();
    }
}
