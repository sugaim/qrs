use std::{borrow::Borrow, collections::BTreeSet, fmt::Display, str::FromStr};

use anyhow::{bail, Context};
use qcollections::size_ensured::{NonEmpty, RequireMinSize};

// -----------------------------------------------------------------------------
// CalendarSymAtom
//
/// An atom of a calendar symbol.
///
/// This is just a string with some constraints.
/// - It should not be empty.
/// - It should consist of alphanumeric characters and underscore.
///
/// This is combined to create a [`CalendarSym`].
///
/// # Examples
/// ```
/// use qchrono::calendar::CalendarSymAtom;
///
/// let sym = CalendarSymAtom::new("TK");
/// assert!(sym.is_ok());
/// assert_eq!(sym.unwrap().as_str(), "TK");
///
/// let sym = CalendarSymAtom::new("TK+NY");
/// assert!(sym.is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarSymAtom(String);

impl Borrow<str> for CalendarSymAtom {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0
    }
}

//
// ser/de
//
impl FromStr for CalendarSymAtom {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for CalendarSymAtom {
    type Error = anyhow::Error;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        CalendarSymAtom::from_str(s)
    }
}

impl Display for CalendarSymAtom {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl serde::Serialize for CalendarSymAtom {
    #[inline]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for CalendarSymAtom {
    #[inline]
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

//
// ctors
//
impl CalendarSymAtom {
    /// Create a new [`CalendarSymAtom`] from a string.]
    ///
    /// # Errors
    /// - If the given string is empty.
    /// - If the given string contains any non-alphanumeric characters other than underscore.
    pub fn new(name: impl Into<String>) -> Result<Self, anyhow::Error> {
        let name: String = name.into();
        let is_ok = |c: char| c.is_ascii_alphanumeric() || c == '_';
        if name.is_empty() {
            bail!("Single calendar symbol should not be empty");
        }
        if name.chars().all(is_ok) {
            Ok(Self(name))
        } else {
            bail!("Invalid calendar symbol. Only alphanumeric characters and underscore are allowed: {name}")
        }
    }
}

impl From<CalendarSymAtom> for String {
    #[inline]
    fn from(s: CalendarSymAtom) -> Self {
        s.0
    }
}

//
// methods
//
impl CalendarSymAtom {
    /// Get the inner string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Take the inner string.
    #[inline]
    pub fn take(self) -> String {
        self.0
    }
}

// -----------------------------------------------------------------------------
// CalendarSym
//

/// A symbol for a calendar.
///
/// # Variants
/// - [`CalendarSym::Single`]: An atom of calendar symbol.
/// - [`CalendarSym::AnyClosed`]: A union of calendar symbols. A day is a holiday if a day is a holiday in any of the symbols.
/// - [`CalendarSym::AllClosed`]: An intersection of calendar symbols. A day is a holiday if a day is a holiday in all of the symbols.
///
/// # String representation
/// - [`CalendarSym::Single`]: String consists of alphanumeric characters and underscore.
/// - [`CalendarSym::AnyClosed`]: `|` separated single symbols. e.g. `TK|NY`.
/// - [`CalendarSym::AllClosed`]: `&` separated single symbols. e.g. `TK&NY`.
///
/// `&` is stronger than `|` like multiplication is stronger than addition.
/// If you need to control the precedence, please use parentheses, e.g. `(TK|NY)&(LN|TK)`.
///
/// # Examples
/// ```
/// use qchrono::calendar::CalendarSym;
///
/// let sym = CalendarSym::any_closed_of(["TK", "NY"]).unwrap();
/// match sym {
///    CalendarSym::AnyClosed(c) => {
///       assert_eq!(c.len(), 2);
///       assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
///       assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));
///    }
///    _ => unreachable!(),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CalendarSym {
    Single(CalendarSymAtom),
    AnyClosed(NonEmpty<BTreeSet<CalendarSym>>),
    AllClosed(NonEmpty<BTreeSet<CalendarSym>>),
}

//
// ser/de
//
impl FromStr for CalendarSym {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        peg::parser!( grammar cal_sym() for str {
            rule single() -> CalendarSym
                = sym:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_']+) {
                    CalendarSym::single_of(sym).expect("validated by peg")
                }
                / expected!("symbol for single calendar")

            rule _()
                = quiet!{[' ' | '\t' | '\n']*}

            pub(crate) rule parse() -> CalendarSym = precedence!{
                x:(@) _ "|" _ y:@ { x | y }
                --
                x:(@) _ "&" _ y:@ { x & y }
                --
                "(" _ v:parse() _ ")" { v }
                n:single() {n}
            }
        });
        let s = s.trim();
        cal_sym::parse(s)
            .map_err(|e| anyhow::anyhow!(e))
            .with_context(|| format!("Invalid calendar symbol: {}", s))
    }
}

impl TryFrom<&str> for CalendarSym {
    type Error = anyhow::Error;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        CalendarSym::from_str(s)
    }
}

impl Display for CalendarSym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Single(s) => write!(f, "{}", s),
            Self::AnyClosed(c) => {
                let mut iter = c.iter();
                write!(f, "{}", iter.next().unwrap())?;
                for sym in iter {
                    write!(f, "|{}", sym)?;
                }
                Ok(())
            }
            Self::AllClosed(c) => {
                let mut iter = c.iter();
                let fst = iter.next().unwrap();
                match fst {
                    Self::AnyClosed(_) => write!(f, "({})", fst)?,
                    _ => write!(f, "{}", fst)?,
                }
                for sym in iter {
                    match sym {
                        Self::AnyClosed(_) => write!(f, "&({})", sym)?,
                        _ => write!(f, "&{}", sym)?,
                    }
                }
                Ok(())
            }
        }
    }
}

impl serde::Serialize for CalendarSym {
    #[inline]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> serde::Deserialize<'de> for CalendarSym {
    #[inline]
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CalendarSym::from_str(&s).map_err(serde::de::Error::custom)
    }
}
impl schemars::JsonSchema for CalendarSym {
    fn schema_name() -> String {
        "CalendarSym".to_owned()
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        "qchrono::calendar::CalendarSym".into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut sch = <String as schemars::JsonSchema>::json_schema(gen).into_object();
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
// ctors
//
impl CalendarSym {
    /// Create a new [`CalendarSym`] from a string for a single calendar.
    /// Available characters are alphabets, integers and underscore.
    ///
    /// # Errors
    /// - If the given string is empty.
    /// - If the given string contains any non-alphanumeric characters other than underscore.
    ///
    /// # Examples
    /// ```
    /// use qchrono::calendar::CalendarSym;
    ///
    /// let sym = CalendarSym::single_of("TK");
    /// assert!(sym.is_ok());
    /// assert_eq!(sym.unwrap(), CalendarSym::Single("TK".parse().unwrap()));
    ///
    /// let sym = CalendarSym::single_of("TK|NY");
    /// assert!(sym.is_err());
    /// ```
    #[inline]
    pub fn single_of(name: impl Into<String>) -> Result<Self, anyhow::Error> {
        CalendarSymAtom::new(name).map(Self::Single)
    }

    /// Create a new [`CalendarSym`] from multiple symbols with any-closed strategy.
    ///
    /// # Errors
    /// - If the given iterator is empty.
    /// - If the given iterator contains any invalid calendar symbols.
    ///
    pub fn any_closed_of<T>(children: impl IntoIterator<Item = T>) -> Result<Self, anyhow::Error>
    where
        T: TryInto<Self>,
        anyhow::Error: From<T::Error>,
    {
        let mut set = BTreeSet::new();
        for child in children.into_iter().map(T::try_into) {
            match child? {
                Self::AnyClosed(c) => {
                    set.extend(c);
                }
                sym => {
                    set.insert(sym);
                }
            }
        }
        set.require_min_size()
            .map(Self::AnyClosed)
            .map_err(|_| anyhow::anyhow!("Empty set of calendar symbols"))
    }

    /// Create a new [`CalendarSym`] from multiple symbols with all-closed strategy.
    ///
    /// # Errors
    /// - If the given iterator is empty.
    /// - If the given iterator contains any invalid calendar symbols.
    ///
    pub fn all_closed_of<T>(children: impl IntoIterator<Item = T>) -> Result<Self, anyhow::Error>
    where
        T: TryInto<Self>,
        anyhow::Error: From<T::Error>,
    {
        let mut set = BTreeSet::new();
        for child in children.into_iter().map(T::try_into) {
            match child? {
                Self::AllClosed(c) => {
                    set.extend(c);
                }
                sym => {
                    set.insert(sym);
                }
            }
        }
        set.require_min_size()
            .map(Self::AllClosed)
            .map_err(|_| anyhow::anyhow!("Empty set of calendar symbols"))
    }
}

//
// methods
//
impl CalendarSym {
    /// Collect all the leaf symbols.
    ///
    /// # Examples
    /// ```
    /// use qchrono::calendar::{CalendarSym, CalendarSymAtom};
    ///
    /// let sym = CalendarSym::any_closed_of(["TK", "NY&LN"].into_iter()).unwrap();
    /// let set = sym.leaves();
    /// let mut set = set.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    /// set.sort();
    ///
    /// assert_eq!(&set, &["LN", "NY", "TK"]);
    /// ```
    #[inline]
    pub fn leaves(&self) -> BTreeSet<CalendarSymAtom> {
        let mut set = BTreeSet::new();
        self.collect_leaves(&mut set);
        set
    }

    #[inline]
    fn collect_leaves(&self, set: &mut BTreeSet<CalendarSymAtom>) {
        match &self {
            Self::Single(s) => {
                set.insert(s.clone());
            }
            Self::AnyClosed(c) | Self::AllClosed(c) => {
                for sym in c.iter() {
                    sym.collect_leaves(set);
                }
            }
        }
    }
}

//
// operators
//
impl std::ops::BitOr for CalendarSym {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self::any_closed_of([self, rhs])
            .expect("When valid symbols are given, the result should be valid")
    }
}

impl std::ops::BitAnd for CalendarSym {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self::all_closed_of([self, rhs])
            .expect("When valid symbols are given, the result should be valid")
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("TK", true)]
    #[case("TK_OSE", true)]
    #[case("TK|NY", false)]
    #[case("TK&NY", false)]
    #[case("(TK)", false)]
    #[case("", false)]
    #[case(" ", false)]
    #[case("😃", false)]
    fn test_atom_new(#[case] s: &str, #[case] ok: bool) {
        let sym = CalendarSymAtom::new(s);

        if ok {
            assert!(sym.is_ok());
            assert_eq!(sym.unwrap().as_str(), s);
        } else {
            assert!(sym.is_err());
        }
    }

    #[rstest]
    #[case("TK", Some(CalendarSym::single_of("TK").unwrap()))]
    #[case("TK_OSE", Some(CalendarSym::single_of("TK_OSE").unwrap()))]
    #[case("TK|NY", None)]
    #[case("TK&NY", None)]
    #[case("(TK)", None)]
    #[case("", None)]
    #[case(" ", None)]
    #[case("😃", None)]
    fn test_single_of(#[case] s: &str, #[case] expected: Option<CalendarSym>) {
        let sym = CalendarSym::single_of(s);

        match expected {
            Some(expected) => {
                assert_eq!(sym.unwrap(), expected);
            }
            None => {
                assert!(sym.is_err());
            }
        }
    }

    #[test]
    fn test_all_closed_of() {
        let sym = CalendarSym::all_closed_of(["TK"]);
        let CalendarSym::AllClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(
            c.into_inner(),
            [CalendarSym::single_of("TK").unwrap()]
                .into_iter()
                .collect::<BTreeSet<_>>()
        );

        let sym = CalendarSym::all_closed_of(["TK", "NY"]);
        assert!(sym.is_ok());
        let CalendarSym::AllClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));

        let sym = CalendarSym::all_closed_of(["TK", "NY", "LN"]);
        assert!(sym.is_ok());
        let CalendarSym::AllClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("LN").unwrap()));

        let sym = CalendarSym::all_closed_of(["TK", "NY", "NY"]);
        assert!(sym.is_ok());
        let CalendarSym::AllClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));

        let sym = CalendarSym::all_closed_of(["TK", "NY&LN"]);
        assert!(sym.is_ok());
        let CalendarSym::AllClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("LN").unwrap()));

        let sym = CalendarSym::all_closed_of(["TK", "NY|LN"]);
        assert!(sym.is_ok());
        let CalendarSym::AllClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::any_closed_of(["NY", "LN"].into_iter()).unwrap()));

        // error
        let sym = CalendarSym::all_closed_of(["", "TK"]);
        assert!(sym.is_err());

        let sym = CalendarSym::all_closed_of(Vec::<&str>::new());
        assert!(sym.is_err());
    }

    #[test]
    fn test_any_closed_of() {
        let sym = CalendarSym::any_closed_of(["TK"]);
        let CalendarSym::AnyClosed(sym) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(
            sym.into_inner(),
            [CalendarSym::single_of("TK").unwrap()]
                .into_iter()
                .collect::<BTreeSet<_>>()
        );

        let sym = CalendarSym::any_closed_of(["TK", "NY"]);
        let CalendarSym::AnyClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));

        let sym = CalendarSym::any_closed_of(["TK", "NY", "LN"]);
        assert!(sym.is_ok());
        let CalendarSym::AnyClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("LN").unwrap()));

        let sym = CalendarSym::any_closed_of(["TK", "NY", "NY"]);
        assert!(sym.is_ok());
        let CalendarSym::AnyClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));

        let sym = CalendarSym::any_closed_of(["TK", "NY&LN"]);
        assert!(sym.is_ok());
        let CalendarSym::AnyClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 2);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::all_closed_of(["NY", "LN"].into_iter()).unwrap()));

        let sym = CalendarSym::any_closed_of(["TK", "NY|LN"]);
        assert!(sym.is_ok());
        let CalendarSym::AnyClosed(c) = sym.unwrap() else {
            unreachable!();
        };
        assert_eq!(c.len(), 3);
        assert!(c.contains(&CalendarSym::single_of("TK").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("NY").unwrap()));
        assert!(c.contains(&CalendarSym::single_of("LN").unwrap()));

        // error
        let sym = CalendarSym::any_closed_of(["", "TK"]);
        assert!(sym.is_err());

        let sym = CalendarSym::any_closed_of(Vec::<&str>::new());
        assert!(sym.is_err());
    }

    #[test]
    fn test_from_str() {
        // single
        let parsed = CalendarSym::from_str("TK");
        assert_eq!(parsed.unwrap(), CalendarSym::single_of("TK").unwrap());

        let parsed = CalendarSym::from_str("(NY)");
        assert_eq!(parsed.unwrap(), CalendarSym::single_of("NY").unwrap());

        // one binary operator
        let parsed = CalendarSym::from_str("TK|NY");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["TK", "NY"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("TK&NY");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::all_closed_of(["TK", "NY"].into_iter()).unwrap()
        );

        // multiple bitor operators
        let parsed = CalendarSym::from_str("TK|NY|LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("NY|( TK|LN )");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("(TK |LN)|NY");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        // multiple bitand operators
        let parsed = CalendarSym::from_str("TK & NY & LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::all_closed_of(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("TK&(NY&LN)");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::all_closed_of(["TK", "NY", "LN"].into_iter()).unwrap()
        );

        // mixed operators
        let parsed = CalendarSym::from_str("TK|NY&LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["TK", "NY&LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("TK&NY|LN");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["NY&TK", "LN"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("TK&NY|LN&TK");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["NY&TK", "LN&TK"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("TK|NY&LN|TK");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::any_closed_of(["TK", "NY&LN", "TK"].into_iter()).unwrap()
        );

        let parsed = CalendarSym::from_str("(TK|NY)&(LN|TK)");
        assert_eq!(
            parsed.unwrap(),
            CalendarSym::all_closed_of(["TK|NY", "LN|TK"].into_iter()).unwrap()
        );

        // error
        let parsed = CalendarSym::from_str("");
        assert!(parsed.is_err());

        let parsed = CalendarSym::from_str(" ");
        assert!(parsed.is_err());

        let parsed = CalendarSym::from_str("😃");
        assert!(parsed.is_err());

        let parsed = CalendarSym::from_str("<TK>");
        assert!(parsed.is_err());

        let parsed = CalendarSym::from_str("TK|NY&");
        assert!(parsed.is_err());

        let parsed = CalendarSym::from_str("T K");
        assert!(parsed.is_err());

        let parsed = CalendarSym::from_str("TK|NY&LN|");
        assert!(parsed.is_err());

        let parsed = CalendarSym::from_str("()");
        assert!(parsed.is_err());
    }

    #[rstest]
    #[case("TK".parse().unwrap(), "TK")]
    #[case("TK|NY".parse().unwrap(), "NY|TK")]
    #[case("TK&NY".parse().unwrap(), "NY&TK")]
    #[case("(TK|NY)&(LN|TK)".parse().unwrap(), "(LN|TK)&(NY|TK)")]
    fn test_display(#[case] sym: CalendarSym, #[case] expected: &str) {
        let s = sym.to_string();

        assert_eq!(s, expected);
    }

    #[rstest]
    #[case("TK".parse().unwrap())]
    #[case("TK|NY".parse().unwrap())]
    #[case("TK&NY".parse().unwrap())]
    #[case("(TK|NY)&(LN|TK)".parse().unwrap())]
    fn test_serialize(#[case] sym: CalendarSym) {
        let ser = serde_json::to_string(&sym).unwrap();

        assert_eq!(ser, format!("\"{}\"", sym));
    }

    #[rstest]
    #[case("TK".parse().unwrap())]
    #[case("TK|NY".parse().unwrap())]
    #[case("TK&NY".parse().unwrap())]
    #[case("(TK|NY)&(LN|TK)".parse().unwrap())]
    fn test_deserialize(#[case] sym: CalendarSym) {
        let ser = serde_json::to_string(&sym).unwrap();

        let de: CalendarSym = serde_json::from_str(&ser).unwrap();

        assert_eq!(de, sym);
    }

    #[test]
    fn test_leaves() {
        let sym = CalendarSym::any_closed_of(["TK", "NY&LN"]).unwrap();
        let mut set = BTreeSet::new();
        sym.collect_leaves(&mut set);
        assert_eq!(set.len(), 3);
        assert!(set.contains("TK"));
        assert!(set.contains("NY"));
        assert!(set.contains("LN"));
    }

    #[rstest]
    #[case("TK", "NY", "TK|NY")]
    #[case("TK", "NY&LN", "TK|NY&LN")]
    #[case("TK", "NY|LN", "TK|NY|LN")]
    fn test_bitor(#[case] lhs: &str, #[case] rhs: &str, #[case] expected: &str) {
        let lhs: CalendarSym = lhs.parse().unwrap();
        let rhs: CalendarSym = rhs.parse().unwrap();
        let expected: CalendarSym = expected.parse().unwrap();

        let sym = lhs | rhs;

        assert_eq!(sym, expected);
    }

    #[rstest]
    #[case("TK", "NY", "TK&NY")]
    #[case("TK", "NY&LN", "TK&NY&LN")]
    #[case("TK", "NY|LN", "TK&(NY|LN)")]
    fn test_bitand(#[case] lhs: &str, #[case] rhs: &str, #[case] expected: &str) {
        let lhs: CalendarSym = lhs.parse().unwrap();
        let rhs: CalendarSym = rhs.parse().unwrap();
        let expected: CalendarSym = expected.parse().unwrap();

        let sym = lhs & rhs;

        assert_eq!(sym, expected);
    }
}
