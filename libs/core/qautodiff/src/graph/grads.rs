use std::convert::Infallible;

use qmath::num::Real;

use crate::{Error, Var};

use super::{
    tape::{_BackProp, _BackPropWorkSpace, _Tape},
    Graph,
};

// -----------------------------------------------------------------------------
// _GradCollect
// -----------------------------------------------------------------------------
struct _GradCollect<'a, V> {
    grads: &'a mut Vec<V>,
}

impl<'a, K, V> _BackProp<K, V> for _GradCollect<'a, V>
where
    V: Real,
{
    type Error = Infallible;

    #[inline]
    fn _on_var(
        &mut self,
        _: usize,
        var_idx: usize,
        _: &K,
        _: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        if self.grads.len() <= var_idx {
            self.grads.resize(var_idx + 1, V::zero());
        }
        self.grads[var_idx] += grad;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// _GradBuf
// _GradPool
// Grads
// -----------------------------------------------------------------------------
/// Reusable buffer for gradients.
///
/// [_GradPool] stores and manages [_GradBuf]s.
/// Although this struct can be shared via multipe [Grads] instances,
/// [Grads] controls side effects of the buffer to other shared instances.
#[derive(Debug)]
struct _GradBuf<V> {
    grads: Vec<V>,
    refcnt: usize,
}

/// Pool of gradient buffers.
///
/// The buffers are reused to reduce the number of allocations
/// and this struct manages the buffers and their reference counts.
/// When reference count becomes zero, the index of the buffer is stored in `vacancy`
/// and used for the next request.
#[derive(Debug)]
pub(super) struct _GradPool<V> {
    grads: Vec<_GradBuf<V>>,
    vacancy: Vec<usize>,
}

impl<V> Default for _GradPool<V> {
    #[inline]
    fn default() -> Self {
        Self {
            grads: Vec::new(),
            vacancy: Vec::new(),
        }
    }
}

impl<V> _GradPool<V> {
    #[inline]
    pub(super) fn _calc_grad<K>(
        &mut self,
        ws: &mut _BackPropWorkSpace<V>,
        tape: &_Tape<K, V>,
        node: usize,
        graph: Graph<K, V>,
    ) -> Grads<K, V>
    where
        V: Real,
    {
        let index = self.vacancy.pop().unwrap_or_else(|| {
            self.grads.push(_GradBuf {
                grads: Vec::new(),
                refcnt: 0,
            });
            self.grads.len() - 1
        });
        self.grads[index].refcnt = 1;
        self.grads[index].grads.clear();

        let mut process = _GradCollect {
            grads: &mut self.grads[index].grads,
        };
        ws._back_prop(tape, node, &mut process).unwrap();
        Grads { graph, index }
    }
}

/// Gradients of the computation graph.
///
/// This is a flyweight object to access gradients stored in the computation graph.
/// So data which this instance refers to is shared among multiple instances
/// but mutable methods are not provided to avoid side effects.
#[derive(Debug)]
pub struct Grads<K, V> {
    graph: Graph<K, V>,
    index: usize,
}

impl<K, V> Drop for Grads<K, V> {
    #[inline]
    fn drop(&mut self) {
        let mut internal = self.graph.0.borrow_mut();
        internal.grad_pool.grads[self.index].refcnt -= 1;
        if internal.grad_pool.grads[self.index].refcnt == 0 {
            internal.grad_pool.vacancy.push(self.index);
        }
    }
}

impl<K, V> Clone for Grads<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        self.graph.0.borrow_mut().grad_pool.grads[self.index].refcnt += 1;
        Self {
            graph: self.graph.clone(),
            index: self.index,
        }
    }
}

impl<K, V> Grads<K, V> {
    /// Return the gradient of the variable.
    ///
    /// If the variable does not belong to the same graph as this instance,
    /// this method returns zero.
    #[inline]
    pub fn wrt(&self, var: &Var<K, V>) -> V
    where
        V: Real,
    {
        if !Graph::ptr_eq(&self.graph, var._node()._graph()) {
            return V::zero();
        }
        let varidx = var._node()._varidx().expect("Variable returns its index");
        let internal = self.graph.0.borrow();
        let grads = &internal.grad_pool.grads[self.index].grads;
        grads.get(varidx).cloned().unwrap_or_else(V::zero)
    }

    /// Collect gradients stored in the computation graph.
    #[inline]
    pub fn collect_mapped<F, X, R>(&self, mut f: F) -> R
    where
        V: Real,
        F: FnMut(&K, V) -> X,
        R: FromIterator<X>,
    {
        let internal = self.graph.0.borrow();
        let vars = internal.tape._vars().iter().enumerate();
        let grads = &internal.grad_pool.grads[self.index].grads;

        vars.map(|(i, idx)| {
            let grad = grads.get(i).cloned().unwrap_or_else(V::zero);
            f(&idx.key, grad)
        })
        .collect()
    }

    #[inline]
    pub fn collect<R>(&self) -> R
    where
        K: Clone,
        V: Real,
        R: FromIterator<(K, V)>,
    {
        self.collect_mapped(|k, v| (k.clone(), v))
    }
}

// -----------------------------------------------------------------------------
// GradsAccum
// -----------------------------------------------------------------------------
/// Gradient accumulator.
///
/// [Grads] is immutable object.
/// Hence, we can not accumulate
#[derive(Debug, Clone)]
pub struct GradsAccum<K, V> {
    graph: Graph<K, V>,
    grads: Vec<V>,
}

//
// ctor
//
impl<K, V> GradsAccum<K, V> {
    /// Create a new gradient aggregator.
    #[inline]
    pub(super) fn new(graph: Graph<K, V>) -> Self {
        Self {
            graph,
            grads: Vec::new(),
        }
    }
}

//
// methods
//
impl<K, V> GradsAccum<K, V> {
    /// Returns the gradient of the variable.
    ///
    /// If the variable belongs to different graph, this method returns zero.
    #[inline]
    pub fn wrt(&self, var: &Var<K, V>) -> V
    where
        V: Real,
    {
        if !Graph::ptr_eq(&self.graph, var._node()._graph()) {
            return V::zero();
        }
        let varidx = var._node()._varidx().expect("Variable returns its index");
        self.grads.get(varidx).cloned().unwrap_or_else(V::zero)
    }

    #[inline]
    pub fn collect<R>(&self) -> R
    where
        K: Clone,
        V: Real,
        R: FromIterator<(K, V)>,
    {
        let internal = self.graph.0.borrow();
        let vars = internal.tape._vars();
        vars.iter()
            .enumerate()
            .map(|(i, idx)| {
                (
                    idx.key.clone(),
                    self.grads.get(i).cloned().unwrap_or_else(V::zero),
                )
            })
            .collect()
    }

    /// Accumulate gradients to this instance.
    ///
    /// This method accumulates gradients passed by the argument with the closure `f`.
    /// The closure `f` is called with two arguments:
    /// the first argument is a mutable reference to data already accumulated in this instance,
    /// and the second argument is a reference to the gradient to be accumulated.
    /// Note that
    pub fn accum<F>(&mut self, grads: &Grads<K, V>, f: F) -> Result<(), Error<K>>
    where
        V: Real,
        F: Fn(&mut V, &V),
    {
        if !Graph::ptr_eq(&self.graph, &grads.graph) {
            return Err(Error::DifferentGraphs("gradient aggregation"));
        }
        let internal = grads.graph.0.borrow();
        let grads = &internal.grad_pool.grads[grads.index].grads;
        if self.grads.len() < grads.len() {
            self.grads.resize(grads.len(), V::zero());
        }
        for (i, grad) in grads.iter().enumerate() {
            f(&mut self.grads[i], grad);
        }
        Ok(())
    }
}
