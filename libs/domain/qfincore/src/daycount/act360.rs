use std::convert::Infallible;

use qchrono::{
    ext::chrono::Datelike,
    timepoint::{Date, DateTime},
};

use super::{StateLessYearFrac, YearFrac};

// -----------------------------------------------------------------------------
// Act360
// -----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Act360;

//
// ser/de
//
impl serde::Serialize for Act360 {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        _serializer.serialize_str("Act360")
    }
}

impl<'de> serde::Deserialize<'de> for Act360 {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        let s: &str = serde::Deserialize::deserialize(_deserializer)?;
        if s == "act360" {
            Ok(Act360)
        } else {
            Err(serde::de::Error::custom(
                "Day count fraction string must be 'act360'",
            ))
        }
    }
}

impl schemars::JsonSchema for Act360 {
    fn schema_name() -> String {
        "Act360".to_string()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qfincore::daycount::Act360".into()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            format: Some("act360".to_string()),
            ..Default::default()
        }
        .into()
    }
}

//
// behavior
//
impl<D: Datelike> StateLessYearFrac<D> for Act360 where Act360: YearFrac<D> {}

impl YearFrac for Act360 {
    type Error = Infallible;

    #[inline]
    fn year_frac(&self, start: &Date, end: &Date) -> Result<f64, Self::Error> {
        let days = (*end - *start).num_days() as f64;
        Ok(days / 360.0)
    }
}

impl YearFrac<DateTime> for Act360 {
    type Error = Infallible;

    #[inline]
    fn year_frac(&self, start: &DateTime, end: &DateTime) -> Result<f64, Self::Error> {
        let days = (end - start).approx_secs();
        Ok(days / (360.0 * 24.0 * 60.0 * 60.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn ymd(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap()
    }

    #[rstest]
    #[case(ymd(2021, 1, 1), ymd(2021, 1, 2), 1. / 360.)]
    #[case(ymd(2021, 1, 1), ymd(2021, 2, 1), 31. / 360.)]
    #[case(ymd(2021, 1, 1), ymd(2022, 1, 1), 365. / 360.)]
    #[case(ymd(2024, 1, 1), ymd(2025, 1, 1), 366. / 360.)]
    #[case(ymd(2021, 7, 13), ymd(2021, 7, 25), 12. / 360.)]
    fn test_year_fraction(#[case] start: Date, #[case] end: Date, #[case] expected: f64) {
        let dcf = Act360.year_frac(&start, &end).unwrap();
        let rev = Act360.year_frac(&end, &start).unwrap();

        approx::assert_abs_diff_eq!(dcf, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(dcf, -rev, epsilon = 1e-10);
    }

    #[rstest]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 1. / 360.)]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-02-01T00:00:00Z".parse().unwrap(), 31. / 360.)]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2022-01-01T00:00:00Z".parse().unwrap(), 365. / 360.)]
    #[case("2024-01-01T00:00:00Z".parse().unwrap(), "2025-01-01T00:00:00Z".parse().unwrap(), 366. / 360.)]
    #[case("2021-07-13T00:00:00Z".parse().unwrap(), "2021-07-25T00:00:00Z".parse().unwrap(), 12. / 360.)]
    #[case("2021-01-01T09:22:33Z".parse().unwrap(), "2021-01-01T11:31:55Z".parse().unwrap(), (22. + 9. * 60. + 2. * 3600.) / 24. / 60. / 60. / 360.)]
    #[case("2021-01-01T09:22:33+09:00".parse().unwrap(), "2021-01-01T11:31:55+09:00".parse().unwrap(), (22. + 9. * 60. + 2. * 3600.) / 24. / 60. / 60. / 360.)]
    #[case("2021-01-01T09:22:33+09:00".parse().unwrap(), "2021-01-01T11:01:55-05:30".parse().unwrap(), (22. + 9. * 60. + 16. * 3600.) / 24. / 60. / 60. / 360.)]
    fn test_year_fraction_datetime(
        #[case] start: DateTime,
        #[case] end: DateTime,
        #[case] expected: f64,
    ) {
        let dcf = Act360.year_frac(&start, &end).unwrap();
        let rev = Act360.year_frac(&end, &start).unwrap();

        approx::assert_abs_diff_eq!(dcf, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(dcf, -rev, epsilon = 1e-10);
    }

    #[test]
    fn test_ser() {
        let act360 = Act360;

        let ser = serde_json::to_string(&act360).unwrap();

        assert_eq!(ser, "\"Act360\"");
    }

    #[test]
    fn test_de() {
        let ser = "\"act360\"";

        let act360: Act360 = serde_json::from_str(ser).unwrap();

        assert_eq!(act360, Act360);
    }

    #[rstest]
    #[case("\"Act360\"")]
    #[case("\"act365f\"")]
    #[case("\" act360\"")]
    #[case("\"act360 \"")]
    fn test_de_err(#[case] ser: &str) {
        let act360: Result<Act360, _> = serde_json::from_str(ser);

        assert!(act360.is_err());
    }
}
