use std::{
    collections::{BTreeSet, HashSet},
    fmt::Display,
    str::FromStr,
};

use anyhow::bail;
#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::Serialize;

// -----------------------------------------------------------------------------
// CalendarSymVariant
//
/// A variant of calendar symbol.
///
/// We forcus on holidays rather than business days.
/// Hence, we use [`CalendarSymVariant::AnyClosed`] and [`CalendarSymVariant::AllClosed`]
/// to represent combined calendars.
/// - [`CalendarSymVariant::Single`]: An atom of calendar symbol.
/// - [`CalendarSymVariant::AnyClosed`]: A union of calendar symbols. A day is a holiday if a day is a holiday in any of the symbols.
/// - [`CalendarSymVariant::AllClosed`]: An intersection of calendar symbols. A day is a holiday if a day is a holiday in all of the symbols.
///
/// Roughly speaking, [`CalendarSymVariant::AnyClosed`] is a logical OR and [`CalendarSymVariant::AllClosed`] is a logical AND.
///
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CalendarSymVariant {
    Single(String),
    AnyClosed(BTreeSet<CalendarSymbol>),
    AllClosed(BTreeSet<CalendarSymbol>),
}

//
// construction
//
impl From<CalendarSymbol> for CalendarSymVariant {
    fn from(sym: CalendarSymbol) -> Self {
        sym.0
    }
}

// -----------------------------------------------------------------------------
// CalendarSymbol
//

/// A symbol for a calendar.
///
/// We need some validation for calendar symbols.
/// This is reason why both of [`CalendarSymbol`] and [`CalendarSymVariant`] are implemented,
/// although [`CalendarSymVariant`] seems enough and useful.
///
/// Please use [`CalendarSymbol`] in your code,
/// but use [`CalendarSymVariant`] when you need to access the variant directly.
///
/// # Variants
/// - [`CalendarSymVariant::Single`]: An atom of calendar symbol.
/// - [`CalendarSymVariant::AnyClosed`]: A union of calendar symbols. A day is a holiday if a day is a holiday in any of the symbols.
/// - [`CalendarSymVariant::AllClosed`]: An intersection of calendar symbols. A day is a holiday if a day is a holiday in all of the symbols.
///
/// # String representation
/// - [`CalendarSymVariant::Single`]: Just use the symbol.
/// - [`CalendarSymVariant::AnyClosed`]: Use `|` to separate symbols. e.g. `TK|NY|LN`.
/// - [`CalendarSymVariant::AllClosed`]: Use `&` to separate symbols. e.g. `TK&NY&LN`.
///
/// Precedence of operators is `|` > `&`.
/// If you need to control the precedence, please use parentheses, e.g. `(TK|NY)&(LN|TK)`.
///
/// # Examples
/// ```
/// use qrs_chrono::CalendarSymbol;
/// use qrs_chrono::CalendarSymVariant;
///
/// let sym = CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap();
/// match sym.dispatch() {
///    CalendarSymVariant::AnyClosed(c) => {
///       assert_eq!(c.len(), 2);
///       assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
///       assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));
///    }
///    _ => unreachable!(),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarSymbol(CalendarSymVariant);

//
// display, serde
//
impl Display for CalendarSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            CalendarSymVariant::Single(s) => write!(f, "{}", s),
            CalendarSymVariant::AnyClosed(c) => {
                let mut iter = c.iter();
                write!(f, "{}", iter.next().unwrap())?;
                for sym in iter {
                    write!(f, "|{}", sym)?;
                }
                Ok(())
            }
            CalendarSymVariant::AllClosed(c) => {
                let mut iter = c.iter();
                let fst = iter.next().unwrap();
                match fst.dispatch() {
                    CalendarSymVariant::AnyClosed(_) => {
                        write!(f, "({})", fst)?;
                    }
                    _ => {
                        write!(f, "{}", fst)?;
                    }
                }
                for sym in iter {
                    match sym.dispatch() {
                        CalendarSymVariant::AnyClosed(_) => {
                            write!(f, "&({})", sym)?;
                        }
                        _ => {
                            write!(f, "&{}", sym)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for CalendarSymbol {
    #[inline]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CalendarSymbol {
    #[inline]
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CalendarSymbol::from_str(&s).map_err(serde::de::Error::custom)
    }
}
#[cfg(feature = "serde")]
impl JsonSchema for CalendarSymbol {
    fn schema_name() -> String {
        "CalendarSymbol".to_owned()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qrs_chrono::CalendarSymbol".into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut sch = <String as JsonSchema>::json_schema(gen).into_object();
        sch.metadata().description = Some(
            "A symbol for a calendar. Combined calendars are available. As an atom, only alphanumeric characters or '_' are allowed.".to_owned(),
        );
        sch.metadata().examples = vec![
            "TK".into(),
            "TK|NY".into(),
            "TK&NY".into(),
            "(TK|NY)&(LN|TK)".into(),
        ];
        sch.into()
    }
}

//
// construction
//
impl FromStr for CalendarSymbol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        peg::parser!( grammar cal_sym() for str {
            rule single() -> CalendarSymbol
                = sym:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_']+) {
                    CalendarSymbol::of_single(sym).expect("validated by peg")
                }
                / expected!("symbol for single calendar")

            rule _()
                = quiet!{[' ' | '\t' | '\n']*}

            pub(crate) rule parse() -> CalendarSymbol = precedence!{
                x:(@) _ "|" _ y:@ { x | y }
                --
                x:(@) _ "&" _ y:@ { x & y }
                --
                "(" _ v:parse() _ ")" { v }
                n:single() {n}
            }
        });
        let s = s.trim();
        cal_sym::parse(s).map_err(|e| anyhow::anyhow!(e))
    }
}

impl TryFrom<&str> for CalendarSymbol {
    type Error = anyhow::Error;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        CalendarSymbol::from_str(s)
    }
}

impl TryFrom<CalendarSymVariant> for CalendarSymbol {
    type Error = anyhow::Error;

    #[inline]
    fn try_from(v: CalendarSymVariant) -> Result<Self, Self::Error> {
        match v {
            CalendarSymVariant::Single(s) => Self::of_single(s),
            CalendarSymVariant::AnyClosed(c) => Self::of_any_closed(c.into_iter()),
            CalendarSymVariant::AllClosed(c) => Self::of_all_closed(c.into_iter()),
        }
    }
}

impl CalendarSymbol {
    /// Create a new [`CalendarSymbol`] from a string for a single calendar.
    /// Available characters are alphabets, integers and underscore.
    ///
    /// # Errors
    /// - If the given string is empty.
    /// - If the given string contains any non-alphanumeric characters other than underscore.
    ///
    /// # Examples
    /// ```
    /// use qrs_chrono::CalendarSymbol;
    /// use qrs_chrono::CalendarSymVariant;
    ///
    /// let sym = CalendarSymbol::of_single("TK");
    /// assert!(sym.is_ok());
    /// assert_eq!(sym.unwrap().dispatch(), &CalendarSymVariant::Single("TK".to_owned()));
    ///
    /// let sym = CalendarSymbol::of_single("TK|NY");
    /// assert!(sym.is_err());
    /// ```
    pub fn of_single(name: impl Into<String>) -> Result<Self, anyhow::Error> {
        let is_ok = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let name = name.into();
        if name.is_empty() {
            bail!("Empty calendar symbol");
        }
        if name.chars().all(is_ok) {
            Ok(Self(CalendarSymVariant::Single(name)))
        } else {
            bail!("Invalid calendar symbol: {}", name);
        }
    }

    /// Create a new [`CalendarSymbol`] from multiple symbols with any-closed strategy.
    ///
    /// # Errors
    /// - If the given iterator is empty.
    /// - If the given iterator contains any invalid calendar symbols.
    ///
    pub fn of_any_closed<T>(children: impl Iterator<Item = T>) -> Result<Self, anyhow::Error>
    where
        T: TryInto<Self>,
        anyhow::Error: From<T::Error>,
    {
        let mut set = BTreeSet::new();
        for child in children.map(T::try_into) {
            match child?.0 {
                CalendarSymVariant::AnyClosed(c) => {
                    set.extend(c);
                }
                sym => {
                    set.insert(Self(sym));
                }
            }
        }
        match set.len() {
            0 => bail!("Empty set of calendar symbols"),
            1 => Ok(set.into_iter().next().unwrap()),
            _ => Ok(Self(CalendarSymVariant::AnyClosed(set))),
        }
    }

    /// Create a new [`CalendarSymbol`] from multiple symbols with all-closed strategy.
    ///
    /// # Errors
    /// - If the given iterator is empty.
    /// - If the given iterator contains any invalid calendar symbols.
    ///
    pub fn of_all_closed<T>(children: impl Iterator<Item = T>) -> Result<Self, anyhow::Error>
    where
        T: TryInto<Self>,
        anyhow::Error: From<T::Error>,
    {
        let mut set = BTreeSet::new();
        for child in children.map(T::try_into) {
            match child?.0 {
                CalendarSymVariant::AllClosed(c) => {
                    set.extend(c);
                }
                sym => {
                    set.insert(Self(sym));
                }
            }
        }
        match set.len() {
            0 => bail!("Empty set of calendar symbols"),
            1 => Ok(set.into_iter().next().unwrap()),
            _ => Ok(Self(CalendarSymVariant::AllClosed(set))),
        }
    }
}

//
// methods
//
impl CalendarSymbol {
    /// Get the variant of the calendar symbol to access the variant directly.
    #[inline]
    pub fn dispatch(&self) -> &CalendarSymVariant {
        &self.0
    }

    /// Take the variant of the calendar symbol to access the variant directly.
    #[inline]
    pub fn take_dispatch(self) -> CalendarSymVariant {
        self.0
    }

    /// Collect all the leaves of the calendar symbol.
    ///
    /// Instead of returning a collection, this method takes a mutable reference to a set
    /// to reduce allocation costs because this operation can be necessary for multiple symbols.
    ///
    /// # Examples
    /// ```
    /// use std::collections::HashSet;
    /// use qrs_chrono::CalendarSymbol;
    ///
    /// let sym = CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter()).unwrap();
    /// let mut set = HashSet::new();
    /// sym.collect_leaves(&mut set);
    /// assert_eq!(set, ["TK", "NY", "LN"].into_iter().map(ToOwned::to_owned).collect::<HashSet<_>>());
    /// ```
    #[inline]
    pub fn collect_leaves(&self, set: &mut HashSet<String>) {
        match &self.0 {
            CalendarSymVariant::Single(s) => {
                set.insert(s.clone());
            }
            CalendarSymVariant::AnyClosed(c) | CalendarSymVariant::AllClosed(c) => {
                for sym in c {
                    sym.collect_leaves(set);
                }
            }
        }
    }
}

//
// operators
//
impl std::ops::BitOr for CalendarSymbol {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self::of_any_closed([self, rhs].into_iter())
            .expect("When valid symbols are given, the result should be valid")
    }
}

impl std::ops::BitAnd for CalendarSymbol {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self::of_all_closed([self, rhs].into_iter())
            .expect("When valid symbols are given, the result should be valid")
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_of_single() {
        let sym = CalendarSymbol::of_single("TK");
        assert_eq!(sym.unwrap(), CalendarSymbol::of_single("TK").unwrap());

        let sym = CalendarSymbol::of_single("TK|NY");
        assert!(sym.is_err());

        let sym = CalendarSymbol::of_single("(TK)");
        assert!(sym.is_err());

        let sym = CalendarSymbol::of_single("");
        assert!(sym.is_err());

        let sym = CalendarSymbol::of_single(" ");
        assert!(sym.is_err());

        let sym = CalendarSymbol::of_single("😃");
        assert!(sym.is_err());
    }

    #[test]
    fn test_of_all_closed() {
        let sym = CalendarSymbol::of_all_closed(["TK"].into_iter());
        assert_eq!(sym.unwrap(), CalendarSymbol::of_single("TK").unwrap());

        let sym = CalendarSymbol::of_all_closed(["TK", "NY"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AllClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));

        let sym = CalendarSymbol::of_all_closed(["TK", "NY", "LN"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AllClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("LN").unwrap()));

        let sym = CalendarSymbol::of_all_closed(["TK", "NY", "NY"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AllClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));

        let sym = CalendarSymbol::of_all_closed(["TK", "NY&LN"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AllClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("LN").unwrap()));

        let sym = CalendarSymbol::of_all_closed(["TK", "NY|LN"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AllClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_any_closed(["NY", "LN"].into_iter()).unwrap()));

        // error
        let sym = CalendarSymbol::of_all_closed(["", "TK"].into_iter());
        assert!(sym.is_err());

        let sym = CalendarSymbol::of_all_closed(Vec::<&str>::new().into_iter());
        assert!(sym.is_err());
    }

    #[test]
    fn test_of_any_closed() {
        let sym = CalendarSymbol::of_any_closed(["TK"].into_iter());
        assert_eq!(sym.unwrap(), CalendarSymbol::of_single("TK").unwrap());

        let sym = CalendarSymbol::of_any_closed(["TK", "NY"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AnyClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));

        let sym = CalendarSymbol::of_any_closed(["TK", "NY", "LN"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AnyClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("LN").unwrap()));

        let sym = CalendarSymbol::of_any_closed(["TK", "NY", "NY"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AnyClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));

        let sym = CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AnyClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_all_closed(["NY", "LN"].into_iter()).unwrap()));

        let sym = CalendarSymbol::of_any_closed(["TK", "NY|LN"].into_iter());
        assert!(sym.is_ok());
        let CalendarSymVariant::AnyClosed(c) = sym.unwrap().take_dispatch() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSymbol::of_single("TK").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("NY").unwrap()));
        assert!(c.contains(&CalendarSymbol::of_single("LN").unwrap()));

        // error
        let sym = CalendarSymbol::of_any_closed(["", "TK"].into_iter());
        assert!(sym.is_err());

        let sym = CalendarSymbol::of_any_closed(Vec::<&str>::new().into_iter());
        assert!(sym.is_err());
    }

    #[test]
    fn test_from_str() {
        // single
        let parsed = CalendarSymbol::from_str("TK");
        assert_eq!(parsed.unwrap(), CalendarSymbol::of_single("TK").unwrap());

        let parsed = CalendarSymbol::from_str("(NY)");
        assert_eq!(parsed.unwrap(), CalendarSymbol::of_single("NY").unwrap());

        // one binary operator
        let parsed = CalendarSymbol::from_str("TK|NY");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("TK&NY");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap()
        );

        // multiple bitor operators
        let parsed = CalendarSymbol::from_str("TK|NY|LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("NY|( TK|LN )");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("(TK |LN)|NY");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        // multiple bitand operators
        let parsed = CalendarSymbol::from_str("TK & NY & LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_all_closed(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("TK&(NY&LN)");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_all_closed(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        // mixed operators
        let parsed = CalendarSymbol::from_str("TK|NY&LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("TK&NY|LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["NY&TK", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("TK&NY|LN&TK");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["NY&TK", "LN&TK"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("TK|NY&LN|TK");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_any_closed(["TK", "NY&LN", "TK"].into_iter()).unwrap()
        );

        let parsed = CalendarSymbol::from_str("(TK|NY)&(LN|TK)");
        assert_eq!(
            parsed.unwrap(),
            CalendarSymbol::of_all_closed(["TK|NY", "LN|TK"].into_iter()).unwrap()
        );

        // error
        let parsed = CalendarSymbol::from_str("");
        assert!(parsed.is_err());

        let parsed = CalendarSymbol::from_str(" ");
        assert!(parsed.is_err());

        let parsed = CalendarSymbol::from_str("😃");
        assert!(parsed.is_err());

        let parsed = CalendarSymbol::from_str("<TK>");
        assert!(parsed.is_err());

        let parsed = CalendarSymbol::from_str("TK|NY&");
        assert!(parsed.is_err());

        let parsed = CalendarSymbol::from_str("T K");
        assert!(parsed.is_err());

        let parsed = CalendarSymbol::from_str("TK|NY&LN|");
        assert!(parsed.is_err());

        let parsed = CalendarSymbol::from_str("()");
        assert!(parsed.is_err());
    }

    #[test]
    fn test_conversion_between_variant() {
        // into variant
        let sym = CalendarSymbol::of_single("TK").unwrap();
        let var: CalendarSymVariant = sym.clone().into();
        assert_eq!(var, CalendarSymVariant::Single("TK".to_owned()));
        assert_eq!(sym.dispatch(), &var);
        assert_eq!(sym.take_dispatch(), var);

        let sym = CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap();
        let var: CalendarSymVariant = sym.clone().into();
        assert_eq!(
            var,
            CalendarSymVariant::AnyClosed(
                ["TK", "NY"]
                    .into_iter()
                    .map(|s| CalendarSymbol::of_single(s).unwrap())
                    .collect()
            )
        );
        assert_eq!(sym.dispatch(), &var);
        assert_eq!(sym.take_dispatch(), var);

        let sym = CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap();
        let var: CalendarSymVariant = sym.clone().into();
        assert_eq!(
            var,
            CalendarSymVariant::AllClosed(
                ["TK", "NY"]
                    .into_iter()
                    .map(|s| CalendarSymbol::of_single(s).unwrap())
                    .collect()
            )
        );
        assert_eq!(sym.dispatch(), &var);
        assert_eq!(sym.take_dispatch(), var);

        // try from variant
        let var = CalendarSymVariant::Single("TK".to_owned());
        let sym: CalendarSymbol = var.try_into().unwrap();
        assert_eq!(sym, CalendarSymbol::of_single("TK").unwrap());

        let var = CalendarSymVariant::AnyClosed(
            ["TK", "NY"]
                .into_iter()
                .map(|s| CalendarSymbol::of_single(s).unwrap())
                .collect(),
        );
        let sym: CalendarSymbol = var.try_into().unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap()
        );

        let var = CalendarSymVariant::AllClosed(
            ["TK", "NY"]
                .into_iter()
                .map(|s| CalendarSymbol::of_single(s).unwrap())
                .collect(),
        );
        let sym: CalendarSymbol = var.try_into().unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap()
        );

        // error
        let var = CalendarSymVariant::Single("TK|NY".to_owned());
        let sym: Result<CalendarSymbol, _> = var.try_into();
        assert!(sym.is_err());

        let var = CalendarSymVariant::Single("".to_owned());
        let sym: Result<CalendarSymbol, _> = var.try_into();
        assert!(sym.is_err());

        let var = CalendarSymVariant::Single(" ".to_owned());
        let sym: Result<CalendarSymbol, _> = var.try_into();
        assert!(sym.is_err());

        let var = CalendarSymVariant::Single("(TK)".to_owned());
        let sym: Result<CalendarSymbol, _> = var.try_into();
        assert!(sym.is_err());

        let var = CalendarSymVariant::AnyClosed(BTreeSet::new());
        let sym: Result<CalendarSymbol, _> = var.try_into();
        assert!(sym.is_err());

        let var = CalendarSymVariant::AllClosed(BTreeSet::new());
        let sym: Result<CalendarSymbol, _> = var.try_into();
        assert!(sym.is_err());
    }

    #[test]
    fn test_display() {
        let sym = CalendarSymbol::of_single("TK").unwrap();
        assert_eq!(sym.to_string(), "TK");

        let sym = CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap();
        assert_eq!(sym.to_string(), "NY|TK");

        let sym = CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap();
        assert_eq!(sym.to_string(), "NY&TK");

        let sym = CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter()).unwrap();
        assert_eq!(sym.to_string(), "TK|LN&NY");

        let sym = CalendarSymbol::of_all_closed(["TK|NY", "LN|TK"].into_iter()).unwrap();
        assert_eq!(sym.to_string(), "(LN|TK)&(NY|TK)");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde() {
        let sym = CalendarSymbol::of_single("TK").unwrap();
        let json = serde_json::to_string(&sym).unwrap();
        assert_eq!(json, "\"TK\"");

        let sym: CalendarSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(sym, CalendarSymbol::of_single("TK").unwrap());

        let sym = CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap();
        let json = serde_json::to_string(&sym).unwrap();
        assert_eq!(json, "\"NY|TK\"");

        let sym: CalendarSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap()
        );

        let sym = CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap();
        let json = serde_json::to_string(&sym).unwrap();
        assert_eq!(json, "\"NY&TK\"");

        let sym: CalendarSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap()
        );

        let sym = CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter()).unwrap();
        let json = serde_json::to_string(&sym).unwrap();
        assert_eq!(json, "\"TK|LN&NY\"");

        let sym: CalendarSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter()).unwrap()
        );

        let sym = CalendarSymbol::of_all_closed(["TK|NY", "LN|TK"].into_iter()).unwrap();
        let json = serde_json::to_string(&sym).unwrap();
        assert_eq!(json, "\"(LN|TK)&(NY|TK)\"");

        let sym: CalendarSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_all_closed(["TK|NY", "LN|TK"].into_iter()).unwrap()
        );
    }

    #[test]
    fn test_bitwise() {
        let sym1 = CalendarSymbol::of_single("TK").unwrap();
        let sym2 = CalendarSymbol::of_single("NY").unwrap();

        let sym = sym1.clone() | sym2.clone();
        assert_eq!(
            sym,
            CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap()
        );

        let sym = sym1 & sym2;
        assert_eq!(
            sym,
            CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap()
        );
    }

    #[test]
    fn test_leaves() {
        let sym = CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter()).unwrap();
        let mut set = HashSet::new();
        sym.collect_leaves(&mut set);
        assert_eq!(set.len(), 3);
        assert!(set.contains("TK"));
        assert!(set.contains("NY"));
        assert!(set.contains("LN"));
    }

    #[test]
    fn test_bitor() {
        let sym = CalendarSymbol::of_single("TK").unwrap();
        let sym = sym | CalendarSymbol::of_single("NY").unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_any_closed(["TK", "NY"].into_iter()).unwrap()
        );

        let sym = CalendarSymbol::of_single("TK").unwrap();
        let sym = sym | CalendarSymbol::of_any_closed(["NY", "LN"].into_iter()).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_any_closed(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        let sym = CalendarSymbol::of_single("TK").unwrap();
        let sym = sym | CalendarSymbol::of_all_closed(["NY", "LN"].into_iter()).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_any_closed(["TK", "NY&LN"].into_iter()).unwrap()
        );
    }

    #[test]
    fn test_bitand() {
        let sym = CalendarSymbol::of_single("TK").unwrap();
        let sym = sym & CalendarSymbol::of_single("NY").unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_all_closed(["TK", "NY"].into_iter()).unwrap()
        );

        let sym = CalendarSymbol::of_single("TK").unwrap();
        let sym = sym & CalendarSymbol::of_any_closed(["NY", "LN"].into_iter()).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_all_closed(["TK", "NY|LN"].into_iter()).unwrap()
        );

        let sym = CalendarSymbol::of_single("TK").unwrap();
        let sym = sym & CalendarSymbol::of_all_closed(["NY", "LN"].into_iter()).unwrap();
        assert_eq!(
            sym,
            CalendarSymbol::of_all_closed(["TK", "NY", "LN"].into_iter()).unwrap()
        );
    }
}
