use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::{Error, Var};

use super::{
    grads::{GradsAccum, _GradPool},
    tape::{_BackPropWorkSpace, _Tape},
    Node,
};

// -----------------------------------------------------------------------------
// _Graph
// Graph
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub(super) struct _Graph<K, V> {
    pub(super) tape: _Tape<K, V>,
    pub(super) workspace: _BackPropWorkSpace<V>,
    pub(super) grad_pool: _GradPool<V>,
}

#[derive(Debug)]
pub struct Graph<K, V>(pub(super) Rc<RefCell<_Graph<K, V>>>);

impl<K, V> Clone for Graph<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// ctor
//
impl<K, V> Default for Graph<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Graph<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(_Graph {
            tape: _Tape::default(),
            workspace: _BackPropWorkSpace::default(),
            grad_pool: _GradPool::default(),
        })))
    }
}

// methods
impl<K, V> Graph<K, V> {
    /// Create a new variable which belongs to this tape.
    #[inline]
    pub fn create_var(&self, key: K, value: V) -> Result<Var<K, V>, Error<K>>
    where
        K: Debug + Eq,
    {
        Node::_create_var(self, key, value).map(Into::into).map(Var)
    }

    /// Check that two tapes are the same instance.
    ///
    /// Note that this comparison is not based on the contents of the tapes.
    #[inline]
    pub fn ptr_eq(lhs: &Self, rhs: &Self) -> bool {
        Rc::ptr_eq(&lhs.0, &rhs.0)
    }

    #[inline]
    pub fn gen_grads_accum(&self) -> GradsAccum<K, V> {
        GradsAccum::new(self.clone())
    }
}

// impls
impl<K, V> Graph<K, V> {
    #[inline]
    pub(super) fn _debug_ptr(&self) -> impl std::fmt::Debug {
        self.0.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_var() {
        let graph = Graph::new();

        let var = graph.create_var("x", 4.2).unwrap();

        assert_eq!(var.key(), "x");
        assert_eq!(var.value(), 4.2);
    }

    #[test]
    fn test_create_var_err_already_exists() {
        let graph = Graph::new();

        graph.create_var("x", 4.2).unwrap();
        let err = graph.create_var("x", 5.2);

        assert!(err.is_err());
        assert_eq!(err.unwrap_err(), Error::VarAlreadyExists("x"));
    }

    #[test]
    fn test_ptr_eq() {
        let graph1 = Graph::<&'static str, f64>::new();
        let graph2 = graph1.clone();

        assert!(Graph::ptr_eq(&graph1, &graph2));
    }

    #[test]
    fn test_ptr_neq() {
        let graph1 = Graph::<&'static str, f64>::new();
        let graph2 = Graph::<&'static str, f64>::new();

        assert!(!Graph::ptr_eq(&graph1, &graph2));
    }
}
