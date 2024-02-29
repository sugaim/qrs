use std::{
    collections::BTreeSet,
    fmt::Display,
    ops::BitXor,
    sync::{Arc, Mutex, Weak},
};

#[cfg(feature = "serde")]
use serde::Serialize;
use uuid::Uuid;

// -----------------------------------------------------------------------------
// StateId
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct StateId(Uuid);

//
// display, serde
//
impl Display for StateId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

//
// construction
//
impl StateId {
    /// Generate a new unique state id.
    ///
    /// Internally, this uses a generating method of [`uuid::Uuid`].
    /// Hence, when multiple instances are created, these may have the different value
    /// even though this function is nullary.
    #[inline]
    pub fn gen() -> Self {
        StateId(Uuid::new_v4())
    }
}

//
// methods
//
impl BitXor for StateId {
    type Output = StateId;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        StateId(Uuid::from_u128(self.0.as_u128() ^ rhs.0.as_u128()))
    }
}

impl BitXor for &StateId {
    type Output = StateId;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        *self ^ *rhs
    }
}

impl BitXor<&StateId> for StateId {
    type Output = StateId;

    #[inline]
    fn bitxor(self, rhs: &StateId) -> Self::Output {
        self ^ *rhs
    }
}

impl BitXor<StateId> for &StateId {
    type Output = StateId;

    #[inline]
    fn bitxor(self, rhs: StateId) -> Self::Output {
        *self ^ rhs
    }
}

// -----------------------------------------------------------------------------
// TreeInfo
//

/// Debug information of a tree of data source.
///
/// Data source may depend on other data sources.
/// This struct represents the tree of the data sources.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize),
    serde(rename_all = "snake_case", tag = "type")
)]
pub enum TreeInfo {
    /// Leaf node of the data source. In other words, the data source does not depend on any other data sources.
    Leaf {
        /// Description of the data source.
        #[cfg_attr(feature = "serde", serde(rename = "description"))]
        desc: String,
        /// Type of the data source.
        #[cfg_attr(feature = "serde", serde(rename = "object_type"))]
        tp: String,
    },
    /// A wrapper of some data source. In other words, the data source depends on a child data source.
    Wrap {
        /// Description of the data source.
        #[cfg_attr(feature = "serde", serde(rename = "description"))]
        desc: String,
        /// Type of the data source.
        #[cfg_attr(feature = "serde", serde(rename = "object_type"))]
        tp: String,
        /// Child of the data source.
        child: Box<TreeInfo>,
    },
    /// A branch of some data sources. In other words, the data source depends on multiple child data sources.
    Branch {
        /// Description of the data source.
        #[cfg_attr(feature = "serde", serde(rename = "description"))]
        desc: String,
        /// Type of the data source.
        #[cfg_attr(feature = "serde", serde(rename = "object_type"))]
        tp: String,
        /// Children of the data source.
        children: BTreeSet<TreeInfo>,
    },
}

// -----------------------------------------------------------------------------
// DebugTree
//
pub trait DebugTree {
    /// Get the description of the data source.
    fn desc(&self) -> String;

    /// Get the tree structure of the data source.
    fn debug_tree(&self) -> TreeInfo;
}

impl<N: ?Sized + DebugTree> DebugTree for Mutex<N> {
    #[inline]
    fn desc(&self) -> String {
        self.lock().unwrap().desc()
    }

    #[inline]
    fn debug_tree(&self) -> TreeInfo {
        self.lock().unwrap().debug_tree()
    }
}

impl<N: ?Sized + DebugTree> DebugTree for Arc<N> {
    #[inline]
    fn desc(&self) -> String {
        self.as_ref().desc()
    }

    #[inline]
    fn debug_tree(&self) -> TreeInfo {
        self.as_ref().debug_tree()
    }
}

// -----------------------------------------------------------------------------
// Subject
//
pub trait Subject: DebugTree {
    /// Accept an observer to be notified when the state of the subject is changed.
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>);

    /// Remove an observer from the list of the observers.
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>);
}

impl<S: ?Sized + Subject> Subject for Mutex<S> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.get_mut().unwrap().reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.get_mut().unwrap().rm_observer(observer);
    }
}

impl<S: ?Sized + Subject> Subject for Arc<Mutex<S>> {
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.lock().unwrap().reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.lock().unwrap().rm_observer(observer);
    }
}

// -----------------------------------------------------------------------------
// Observer
//
pub trait Observer: 'static + Send + Sync {
    fn receive(&mut self, new_state: &StateId);
}

// =============================================================================
#[cfg(test)]
mod tests {
    use uuid::Uuid;

    #[test]
    fn test_stateid_display() {
        let id = super::StateId(Uuid::from_u128(u128::MAX));
        assert_eq!(format!("{}", id), format!("{}", id.0));

        let id = super::StateId(Uuid::from_u128(u128::MIN));
        assert_eq!(format!("{}", id), format!("{}", id.0));

        let id = super::StateId(Uuid::from_u128(u128::MAX / 2));
        assert_eq!(format!("{}", id), format!("{}", id.0));
    }

    #[test]
    fn test_stateid_bitxor() {
        let id1 = super::StateId(Uuid::from_u128(u128::MAX));
        let id2 = super::StateId(Uuid::from_u128(u128::MIN));
        let id3 = super::StateId(Uuid::from_u128(u128::MAX / 2));

        for lhs in &[id1, id2, id3] {
            // self is inverse of itself
            assert_eq!(lhs ^ lhs, super::StateId(Uuid::from_u128(0)));
            for rhs in &[id1, id2, id3] {
                // value check
                assert_eq!((lhs ^ rhs).0.as_u128(), (lhs.0.as_u128() ^ rhs.0.as_u128()));

                // commutative
                assert_eq!(lhs ^ rhs, rhs ^ lhs);

                for mid in &[id1, id2, id3] {
                    // associative
                    assert_eq!((lhs ^ rhs) ^ mid, lhs ^ (rhs ^ mid));
                }
            }
        }
    }
}
