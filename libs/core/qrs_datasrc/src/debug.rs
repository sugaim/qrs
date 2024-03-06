use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

// -----------------------------------------------------------------------------
// TreeInfo
//

/// Debug information of a data source tree
///
/// Data source may depend on other data sources.
/// This struct represents the tree of the data sources.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, schemars::JsonSchema),
    serde(rename_all = "snake_case", tag = "type"),
    schemars(description = "Debug information of a data source tree")
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
        children: BTreeMap<String, TreeInfo>,
    },
}

// -----------------------------------------------------------------------------
// DebugTree
//

/// Generate debug information of a data source tree.
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

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_desc() {
        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc, desc = "desc")]
        struct Leaf;

        let desc = Leaf.desc();

        assert_eq!(desc, "desc");
    }

    #[test]
    fn test_derive_desc_field() {
        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc, desc_field = "int")]
        struct Leaf {
            int: i32,
        }

        let desc = Leaf { int: 42 }.desc();

        assert_eq!(desc, "42");
    }

    #[test]
    fn test_derive_desc_func() {
        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc, desc_func = "desc_func")]
        struct Leaf;

        fn desc_func(_: &Leaf) -> String {
            "desc_func".to_owned()
        }

        let desc = Leaf.desc();

        assert_eq!(desc, "desc_func");
    }

    #[test]
    fn test_derive_leaf() {
        #[derive(qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc)]
        struct Leaf;

        let tree = Leaf.debug_tree();

        let TreeInfo::Leaf { desc, .. } = tree else {
            panic!("Expected a leaf node, but got {:?}", tree);
        };
        assert_eq!(desc, "no description");
    }

    #[test]
    fn test_derive_wrap() {
        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc, desc = "wrapped")]
        struct Leaf;

        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc)]
        struct Wrap {
            #[debug_tree(subtree)]
            child: Leaf,
        }

        let tree = Wrap::default().debug_tree();

        let TreeInfo::Wrap { desc, child, .. } = tree else {
            panic!("Expected a wrap node, but got {:?}", tree);
        };
        assert_eq!(desc, "no description");
        let TreeInfo::Leaf { desc, .. } = *child else {
            panic!("Expected a leaf node, but got {:?}", child);
        };
        assert_eq!(desc, "wrapped");
    }

    #[test]
    fn test_derive_tuple_wrap() {
        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc, desc = "wrapped")]
        struct Leaf;

        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc)]
        struct Wrap(#[debug_tree(subtree)] Leaf);

        let tree = Wrap::default().debug_tree();

        let TreeInfo::Wrap { desc, child, .. } = tree else {
            panic!("Expected a wrap node, but got {:?}", tree);
        };
        assert_eq!(desc, "no description");
        let TreeInfo::Leaf { desc, .. } = *child else {
            panic!("Expected a leaf node, but got {:?}", child);
        };
        assert_eq!(desc, "wrapped");
    }

    #[test]
    fn test_derive_branch() {
        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc)]
        struct Leaf;

        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc, desc = "branch")]
        struct Branch {
            #[debug_tree(subtree)]
            child1: Leaf,

            _int: i32,

            #[debug_tree(subtree)]
            child2: Leaf,
        }

        let tree = Branch::default().debug_tree();

        let TreeInfo::Branch {
            desc, mut children, ..
        } = tree
        else {
            panic!("Expected a branch node, but got {:?}", tree);
        };
        assert_eq!(desc, "branch");
        assert_eq!(children.len(), 2);
        assert_eq!(
            children.pop_first().unwrap(),
            ("child1".to_owned(), Leaf.debug_tree())
        );
        assert_eq!(
            children.pop_first().unwrap(),
            ("child2".to_owned(), Leaf.debug_tree())
        );
    }

    #[test]
    fn test_derive_tuple_branch() {
        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc)]
        struct Leaf;

        #[derive(Default, qrs_datasrc_derive::DebugTree)]
        #[debug_tree(_use_from_qrs_datasrc, desc = "branch")]
        struct Branch(
            #[debug_tree(subtree)] Leaf,
            i32,
            #[debug_tree(subtree)] Leaf,
        );

        let tree = Branch::default().debug_tree();

        let TreeInfo::Branch {
            desc, mut children, ..
        } = tree
        else {
            panic!("Expected a branch node, but got {:?}", tree);
        };
        assert_eq!(desc, "branch");
        assert_eq!(children.len(), 2);
        assert_eq!(
            children.pop_first().unwrap(),
            ("0".to_owned(), Leaf.debug_tree())
        );
        assert_eq!(
            children.pop_first().unwrap(),
            ("2".to_owned(), Leaf.debug_tree())
        );
    }
}
