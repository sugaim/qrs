use core::f64;
use std::{collections::BTreeMap, convert::Infallible};

use qmath::num::Real;

use crate::{Error, Expr};

use super::{grads::Grads, Graph};

// -----------------------------------------------------------------------------
// _Node
// -----------------------------------------------------------------------------
#[derive(Debug)]
enum _Node<V> {
    // nullary
    Leaf { value: V, index: usize },

    // unary
    Neg { value: V, index: usize },
    AddL { value: V, lhs: usize, rhs: V },
    AddR { value: V, lhs: V, rhs: usize },
    SubL { value: V, lhs: usize, rhs: V },
    SubR { value: V, lhs: V, rhs: usize },
    MulL { value: V, lhs: usize, rhs: V },
    MulR { value: V, lhs: V, rhs: usize },
    DivL { value: V, lhs: usize, rhs: V },
    DivR { value: V, lhs: V, rhs: usize },
    Exp { value: V, index: usize },
    Log { value: V, index: usize },
    Erf { value: V, index: usize },
    Sqrt { value: V, index: usize },
    Powi { value: V, index: usize, exp: i32 },

    // binary
    Add { value: V, lhs: usize, rhs: usize },
    Sub { value: V, lhs: usize, rhs: usize },
    Mul { value: V, lhs: usize, rhs: usize },
    Div { value: V, lhs: usize, rhs: usize },

    // multi-ary
    Compressed { value: V, grads: Vec<V> },
}

impl<V> _Node<V> {
    #[inline]
    fn value(&self) -> &V {
        match self {
            _Node::Leaf { value, .. }
            | _Node::Neg { value, .. }
            | _Node::AddL { value, .. }
            | _Node::AddR { value, .. }
            | _Node::SubL { value, .. }
            | _Node::SubR { value, .. }
            | _Node::MulL { value, .. }
            | _Node::MulR { value, .. }
            | _Node::DivL { value, .. }
            | _Node::DivR { value, .. }
            | _Node::Exp { value, .. }
            | _Node::Log { value, .. }
            | _Node::Erf { value, .. }
            | _Node::Sqrt { value, .. }
            | _Node::Powi { value, .. }
            | _Node::Add { value, .. }
            | _Node::Sub { value, .. }
            | _Node::Mul { value, .. }
            | _Node::Div { value, .. }
            | _Node::Compressed { value, .. } => value,
        }
    }
}

// -----------------------------------------------------------------------------
// Node
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub(crate) struct Node<K, V> {
    graph: Graph<K, V>,
    index: usize,
}

impl<K, V> Clone for Node<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        self.graph.0.borrow_mut().tape._incl_refcnt(self.index);
        Self {
            index: self.index,
            graph: self.graph.clone(),
        }
    }
}

impl<K, V> Drop for Node<K, V> {
    #[inline]
    fn drop(&mut self) {
        self.graph.0.borrow_mut().tape._decl_refcnt(self.index);
    }
}

//
// methods
//
impl<K, V> Node<K, V> {
    #[inline]
    pub(crate) fn _indirectly_read<R>(&self, f: impl FnOnce(&V) -> R) -> R {
        f(self.graph.0.borrow().tape._cell(self.index).value())
    }

    #[inline]
    pub(crate) fn _key(&self) -> Option<K>
    where
        K: Clone,
    {
        self.graph.0.borrow().tape._key(self.index).cloned()
    }

    #[inline]
    pub(crate) fn _graph(&self) -> &Graph<K, V> {
        &self.graph
    }

    #[inline]
    pub(crate) fn _grads(&self) -> Grads<K, V>
    where
        V: Real,
    {
        let mut internal = self.graph.0.borrow_mut();
        let internal = &mut *internal;
        internal.grad_pool._calc_grad(
            &mut internal.workspace,
            &internal.tape,
            self.index,
            self.graph.clone(),
        )
    }

    #[inline]
    pub(crate) fn _varidx(&self) -> Option<usize> {
        let internal = self.graph.0.borrow();
        match internal.tape._cell(self.index) {
            _Node::Leaf { index, .. } => Some(*index),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn _compress(&self) -> Expr<K, V>
    where
        V: Real,
    {
        let node = _Node::Compressed {
            value: self._indirectly_read(|val| val.clone()),
            grads: self._grads().collect_mapped(|_, v| v),
        };
        Node {
            index: self.graph.0.borrow_mut().tape._reg_node(node),
            graph: self.graph.clone(),
        }
        .into()
    }

    #[inline]
    pub(crate) fn _dotize(&self) -> GraphvizBuilder<K, V, (), ()>
    where
        K: Clone,
        V: Real,
    {
        let mut collector = _GraphvizGraph::default();
        {
            let mut internal = self.graph.0.borrow_mut();
            let internal = &mut *internal;
            internal
                .workspace
                ._back_prop(&internal.tape, self.index, &mut collector)
                .unwrap();
        }

        let resolve_node = |node: &_GraphvizNodeIdx| match node {
            _GraphvizNodeIdx::Cell(idx) => collector.cell2node[idx],
            _GraphvizNodeIdx::Node(idx) => *idx,
        };
        let mut edges: Vec<_> = collector
            .edges
            .into_iter()
            .map(|(src, dst, label)| (resolve_node(&src), resolve_node(&dst), label))
            .collect();
        edges.sort();
        GraphvizBuilder {
            name: "GradientGraph".to_string(),
            nodes: collector.nodes,
            edges,
            graph_global_settings: Default::default(),
            node_global_settings: Default::default(),
            key_fmt: Default::default(),
            value_fmt: Default::default(),
        }
    }
}

impl<K, V> Node<K, V> {
    #[inline]
    pub(super) fn _create_var(graph: &Graph<K, V>, key: K, value: V) -> Result<Self, Error<K>>
    where
        K: Eq,
    {
        let index = graph.0.borrow_mut().tape._reg_var(key, value)?;
        Ok(Self {
            index,
            graph: graph.clone(),
        })
    }
}

// -----------------------------------------------------------------------------
// _BackPropPostProcess
// _BackPropWorkSpace
// -----------------------------------------------------------------------------
#[allow(unused)]
pub(super) trait _BackProp<K, V> {
    type Error;

    #[inline]
    fn _on_var(
        &mut self,
        cell_idx: usize,
        var_idx: usize,
        key: &K,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_neg(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_addl(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_addr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_subl(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_subr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_mull(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_mulr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_divl(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_divr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_exp(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_log(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_erf(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_sqrt(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_powi(
        &mut self,
        cell_idx: usize,
        arg: usize,
        exp: i32,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_add(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_sub(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_mul(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_div(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn _on_compressed(
        &mut self,
        cell_idx: usize,
        grads: &[V],
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct _BackPropWorkSpace<V> {
    refcount: Vec<usize>,
    visited: Vec<bool>,
    grads_memo: Vec<V>,
    next_nodes: Vec<usize>,
}

impl<V> Default for _BackPropWorkSpace<V> {
    #[inline]
    fn default() -> Self {
        Self {
            refcount: Vec::new(),
            visited: Vec::new(),
            grads_memo: Vec::new(),
            next_nodes: Vec::new(),
        }
    }
}

impl<V> _BackPropWorkSpace<V> {
    /// Count the number of references to each node in the computation graph.
    /// Note that this method is a preparation for gradient calculation
    /// and please does not call for other purposes.
    fn _count_ref<K>(&mut self, tape: &_Tape<K, V>, root: usize)
    where
        V: Real,
    {
        let refcount = &mut self.refcount;
        refcount.clear();
        refcount.resize(tape.cells.len(), 0);

        let visited = &mut self.visited;
        visited.clear();
        visited.resize(tape.cells.len(), false);

        let stack = &mut self.next_nodes;
        stack.clear();
        stack.push(root);

        while let Some(idx) = stack.pop() {
            // skip if already visited
            // this is necessary to avoid overcounting.
            // For example, when we have a calculation graph like
            // ```text
            // y = x0 * x1
            // z = exp(y)
            // w = y * z
            // ```
            // there are 2 paths from 'x0' to 'w' but we need to count
            // the reference of 'x0' as 1 because 'x0' depends on 'w' through 'y'.
            // In the above example, we expects correct reference counts are
            // x0=1, x1=1, y=2, z=1, w=1.
            refcount[idx] += 1;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            let node = tape._cell(idx);
            match node {
                // variables
                _Node::Leaf { .. } => {}
                // unary
                _Node::Neg { index, .. }
                | _Node::AddL { lhs: index, .. }
                | _Node::AddR { rhs: index, .. }
                | _Node::SubL { lhs: index, .. }
                | _Node::SubR { rhs: index, .. }
                | _Node::MulL { lhs: index, .. }
                | _Node::MulR { rhs: index, .. }
                | _Node::DivL { lhs: index, .. }
                | _Node::DivR { rhs: index, .. }
                | _Node::Exp { index, .. }
                | _Node::Log { index, .. }
                | _Node::Erf { index, .. }
                | _Node::Sqrt { index, .. }
                | _Node::Powi { index, .. } => stack.push(*index),
                // binary
                _Node::Add { lhs, rhs, .. }
                | _Node::Sub { lhs, rhs, .. }
                | _Node::Mul { lhs, rhs, .. }
                | _Node::Div { lhs, rhs, .. } => {
                    stack.push(*lhs);
                    stack.push(*rhs);
                }
                // multi-ary
                _Node::Compressed { grads, .. } => {
                    for cell_idx in (0..grads.len()).map(|idx| tape.vars[idx].cell_idx) {
                        refcount[cell_idx] += 1;
                    }
                }
            }
        }
    }

    pub(super) fn _back_prop<K, Proccesor>(
        &mut self,
        tape: &_Tape<K, V>,
        root: usize,
        proc: &mut Proccesor,
    ) -> Result<(), Proccesor::Error>
    where
        V: Real,
        Proccesor: _BackProp<K, V>,
    {
        // Naive implementation which is like depth-first search is simple but
        // some redandant calculation occurs.
        // For example, when we have a calculation graph like
        // ```text
        // y = x0 * x1
        // z = exp(y)
        // w = y * z
        // ```
        // depth-first search calculates the gradient of `y` twice
        // through `y` and `z`.
        //
        // To avoid this redundant calculation, we wait until all the upstream
        // finishes to propagate the gradient.
        // In the above example, we propagate the gradient of `y` to `x0` and `x1`
        // only after the gradient is propaged from 'w' to 'y' and 'z' to 'y'.
        //
        // We counts the number of references to each node in the computation graph
        // to implement this waiting mechanism.
        // For each propagation, we decrement the reference count of the node
        // and if the reference count becomes zero, we propagate the gradient to
        // the further downstream nodes.
        //
        // For the above example, reference counts of the initial nodes are
        // x0=1, x1=1, y=2, z=1, w=1.
        // At the first, we set gradient of `w` to 1 and propagate it to `y` and `z`.
        // Then, reference counts become x0=1, x1=1, y=1, z=0, w=0.
        // Since the reference count of `z` becomes zero, we propagate the gradient to `y`
        // and update the reference counts to x0=1, x1=1, y=0, z=0, w=0.
        // Finally, we propagate the gradient of `y` to `x0` and `x1` because the reference
        // counts of 'y' becomes zero.

        // preparation phase
        self._count_ref(tape, root);
        let refcount = &mut self.refcount;

        let stack = &mut self.next_nodes;
        stack.clear();
        stack.push(root);

        let grads_memo = &mut self.grads_memo;
        grads_memo.clear();
        grads_memo.resize(tape.cells.len(), V::zero());
        grads_memo[root] = V::one();

        // calculation phase
        let _decl_refcnt = |idx: usize, rc: &mut Vec<usize>, next: &mut Vec<usize>| {
            rc[idx] -= 1;
            if rc[idx] == 0 {
                next.push(idx);
            }
        };

        while let Some(tgt) = stack.pop() {
            let node = tape._cell(tgt);
            let seed = grads_memo[tgt].clone();
            match node {
                // variables
                _Node::Leaf { value, index } => {
                    proc._on_var(tgt, *index, &tape.vars[*index].key, value, &seed)?;
                }
                // unary arithmetic
                _Node::Neg { value, index } => {
                    proc._on_neg(tgt, *index, value, &seed)?;
                    grads_memo[*index] -= &seed;
                    _decl_refcnt(*index, refcount, stack);
                }
                _Node::AddL { value, lhs, rhs } => {
                    proc._on_addl(tgt, *lhs, rhs, value, &seed)?;
                    grads_memo[*lhs] += &seed;
                    _decl_refcnt(*lhs, refcount, stack);
                }
                _Node::AddR { value, lhs, rhs } => {
                    proc._on_addr(tgt, lhs, *rhs, value, &seed)?;
                    grads_memo[*rhs] += &seed;
                    _decl_refcnt(*rhs, refcount, stack);
                }
                _Node::SubL { value, lhs, rhs } => {
                    proc._on_subl(tgt, *lhs, rhs, value, &seed)?;
                    grads_memo[*lhs] += &seed;
                    _decl_refcnt(*lhs, refcount, stack);
                }
                _Node::SubR { value, lhs, rhs } => {
                    proc._on_subr(tgt, lhs, *rhs, value, &seed)?;
                    grads_memo[*rhs] -= &seed;
                    _decl_refcnt(*rhs, refcount, stack);
                }
                _Node::MulL { value, lhs, rhs } => {
                    proc._on_mull(tgt, *lhs, rhs, value, &seed)?;
                    grads_memo[*lhs] += &(seed * rhs);
                    _decl_refcnt(*lhs, refcount, stack);
                }
                _Node::MulR { value, lhs, rhs } => {
                    proc._on_mulr(tgt, lhs, *rhs, value, &seed)?;
                    grads_memo[*rhs] += &(seed * lhs);
                    _decl_refcnt(*rhs, refcount, stack);
                }
                _Node::DivL { value, lhs, rhs } => {
                    proc._on_divl(tgt, *lhs, rhs, value, &seed)?;
                    grads_memo[*lhs] += &(seed / rhs);
                    _decl_refcnt(*lhs, refcount, stack);
                }
                _Node::DivR { value, lhs, rhs } => {
                    proc._on_divr(tgt, lhs, *rhs, value, &seed)?;
                    let rhs_val = tape._cell(*rhs).value();
                    grads_memo[*rhs] -= &(seed * lhs / rhs_val / rhs_val);
                    _decl_refcnt(*rhs, refcount, stack);
                }
                // unary elementary functions
                _Node::Exp { value, index } => {
                    proc._on_exp(tgt, *index, value, &seed)?;
                    grads_memo[*index] += &(seed * value);
                    _decl_refcnt(*index, refcount, stack);
                }
                _Node::Log { value, index } => {
                    proc._on_log(tgt, *index, value, &seed)?;
                    let val = tape._cell(*index).value();
                    grads_memo[*index] += &(seed / val);
                    _decl_refcnt(*index, refcount, stack);
                }
                _Node::Erf { value, index } => {
                    proc._on_erf(tgt, *index, value, &seed)?;
                    let coeff = V::nearest_value_of_f64(2.0 / f64::consts::PI.sqrt());
                    let arg = tape._cell(*index).value();

                    grads_memo[*index] += &(seed * coeff * (-arg.clone() * arg).exp());
                    _decl_refcnt(*index, refcount, stack);
                }
                _Node::Sqrt { value, index } => {
                    proc._on_sqrt(tgt, *index, value, &seed)?;
                    let coeff = V::nearest_value_of_f64(0.5);
                    grads_memo[*index] += &(seed * &coeff / value);
                    _decl_refcnt(*index, refcount, stack);
                }
                _Node::Powi { value, index, exp } => {
                    proc._on_powi(tgt, *index, *exp, value, &seed)?;
                    let coeff = V::nearest_value_of_f64(*exp as f64);
                    let val = tape._cell(*index).value();
                    grads_memo[*index] += &(seed * &val.clone().powi(*exp - 1) * &coeff);
                    _decl_refcnt(*index, refcount, stack);
                }
                // binary arithmetic
                _Node::Add { value, lhs, rhs } => {
                    proc._on_add(tgt, *lhs, *rhs, value, &seed)?;
                    grads_memo[*lhs] += &seed;
                    grads_memo[*rhs] += &seed;
                    _decl_refcnt(*lhs, refcount, stack);
                    _decl_refcnt(*rhs, refcount, stack);
                }
                _Node::Sub { value, lhs, rhs } => {
                    proc._on_sub(tgt, *lhs, *rhs, value, &seed)?;
                    grads_memo[*lhs] += &seed;
                    grads_memo[*rhs] -= &seed;
                    _decl_refcnt(*lhs, refcount, stack);
                    _decl_refcnt(*rhs, refcount, stack);
                }
                _Node::Mul { value, lhs, rhs } => {
                    proc._on_mul(tgt, *lhs, *rhs, value, &seed)?;
                    let lhs_val = tape._cell(*lhs).value();
                    let rhs_val = tape._cell(*rhs).value();
                    grads_memo[*lhs] += &(seed.clone() * rhs_val);
                    grads_memo[*rhs] += &(seed * lhs_val);
                    _decl_refcnt(*lhs, refcount, stack);
                    _decl_refcnt(*rhs, refcount, stack);
                }
                _Node::Div { value, lhs, rhs } => {
                    proc._on_div(tgt, *lhs, *rhs, value, &seed)?;
                    let lhs_val = tape._cell(*lhs).value();
                    let rhs_val = tape._cell(*rhs).value();
                    grads_memo[*lhs] += &(seed.clone() / rhs_val);
                    grads_memo[*rhs] -= &(seed * lhs_val / rhs_val / rhs_val);
                    _decl_refcnt(*lhs, refcount, stack);
                    _decl_refcnt(*rhs, refcount, stack);
                }
                // multi-ary
                _Node::Compressed { value, grads } => {
                    proc._on_compressed(tgt, grads, value, &seed)?;
                    for (idx, grad) in grads.iter().enumerate() {
                        let idx = tape.vars[idx].cell_idx;
                        grads_memo[idx] += &(seed.clone() * grad);
                        _decl_refcnt(idx, refcount, stack);
                    }
                }
            }
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// _VarIdx
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct _VarIdx<K> {
    pub(super) cell_idx: usize,
    pub(super) key: K,
}

// -----------------------------------------------------------------------------
// _TapeCell
// _Tape
// -----------------------------------------------------------------------------
#[derive(Debug)]
struct _TapeCell<V> {
    node: _Node<V>,
    refcnt: usize,
}

/// Design note:
///
/// We want to use AAD under monte carlo simulation and need to implement
/// 1. cleaning up unused nodes
/// 2. random access to nodes
///
/// The first one is necessary to avoid memory error.
/// If we can't clean up unused nodes, the amount of memory will increase linearly
/// with the number of simulation.
///
/// The second one is necessary for calculation performance.
/// Monte carlo simulation involves heavy loops and searching algorithm like binary search
/// should be avoided as much as possible.
///
/// To achieve the first one, we counts the number of references to each node.
/// Although rust standard library has [`std::rc::Rc`], we implement the reference counting
/// manually because creating [`std::rc::Rc`] instance needs some search algorithm
/// on available heap memory.
/// Instead of using [`std::rc::Rc`], we store reference counts of nodes in a [`Vec`].
/// [`Vec`] also needs to searching for memory allocation, but it is not so frequent
/// and we can reuse the allocated memory after cleaning up unused nodes.
///
/// Node with zero reference count can be cleaned up.
/// There are some ways to implement this 'cleaning up' process.
///
/// The simplest idea is to use [`Vec::retain`] method.
/// However, it is not so easy to implement this because retain changes
/// indices of elements, nodes of computation graph, and we need to keep updating
/// the indices of nodes consistently including the references.
/// So, with this idea, we may need to share the indices of nodes between all the references
/// but we need something like [`std::rc::Rc`] to do this and it causes heap memory allocation,
/// which leads to the bad performance.
///
/// The second idea is to overwrite the node with zero reference count without any change of indices.
/// However, we need to search the node with zero reference count and it is not so efficient.
///
/// So, we use the third idea.
/// Basic strategy is the same as the second idea, but we also manage
/// the indices of nodes with zero reference count in a separate [`Vec`].
/// Then, we can get the index of the node with zero reference count in O(1) time
/// with [`Vec::pop`] and overwrite the node with zero reference count.
///
/// Currently, variables which are differentiable nodes are treated in a special way.
/// Even when nothing refers to the variable, it is not cleaned up.
#[derive(Debug)]
pub(super) struct _Tape<K, V> {
    cells: Vec<_TapeCell<V>>,
    vacancy: Vec<usize>,
    vars: Vec<_VarIdx<K>>,
    next_nodes: Vec<usize>,
}

impl<K, V> Default for _Tape<K, V> {
    #[inline]
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            vacancy: Vec::new(),
            vars: Vec::new(),
            next_nodes: Vec::new(),
        }
    }
}

impl<K, V> _Tape<K, V> {
    #[inline]
    pub(super) fn _vars(&self) -> &[_VarIdx<K>] {
        &self.vars
    }
}

impl<K, V> _Tape<K, V> {
    #[inline]
    fn _cell(&self, idx: usize) -> &_Node<V> {
        &self
            .cells
            .get(idx)
            .expect("Tape must manage this expr")
            .node
    }

    #[inline]
    fn _key(&self, idx: usize) -> Option<&K> {
        match self._cell(idx) {
            _Node::Leaf { index, .. } => self.vars.get(*index).map(|_VarIdx { key, .. }| key),
            _ => None,
        }
    }

    fn _reg_var(&mut self, key: K, value: V) -> Result<usize, Error<K>>
    where
        K: Eq,
    {
        if self.vars.iter().any(|_VarIdx { key: k, .. }| k == &key) {
            return Err(Error::VarAlreadyExists(key));
        }

        let index = self._reg_node(_Node::Leaf {
            value,
            index: self.vars.len(),
        });
        self.vars.push(_VarIdx {
            cell_idx: index,
            key,
        });
        Ok(index)
    }

    #[inline]
    fn _reg_node(&mut self, node: _Node<V>) -> usize {
        let cell = _TapeCell { node, refcnt: 1 };
        let index = if let Some(idx) = self.vacancy.pop() {
            let elem = self.cells.get_mut(idx).expect("Tape must manage this expr");
            *elem = cell;
            idx
        } else {
            self.cells.push(cell);
            self.cells.len() - 1
        };
        index
    }

    #[inline]
    fn _incl_refcnt(&mut self, idx: usize) {
        let cell = self.cells.get_mut(idx).expect("Tape must manage this expr");
        cell.refcnt += 1;
    }

    fn _decl_refcnt(&mut self, idx: usize) {
        // to avoid stack overflow, we use iterative approach rather than recursive one
        let stack = &mut self.next_nodes;
        stack.clear();
        stack.push(idx);

        while let Some(idx) = stack.pop() {
            let Some(cell) = self.cells.get_mut(idx) else {
                continue;
            };
            if cell.refcnt == 0 {
                continue;
            }
            cell.refcnt -= 1; // 0 < refcnt is assumed
            if cell.refcnt != 0 {
                continue;
            }

            match &cell.node {
                // variable
                _Node::Leaf { .. } => {
                    // variables are treated in a special way
                    // to enable random access on gradients, we don't clean up variables
                }

                // unary
                _Node::Neg { index, .. }
                | _Node::AddL { lhs: index, .. }
                | _Node::SubL { lhs: index, .. }
                | _Node::MulL { lhs: index, .. }
                | _Node::DivL { lhs: index, .. }
                | _Node::AddR { rhs: index, .. }
                | _Node::SubR { rhs: index, .. }
                | _Node::MulR { rhs: index, .. }
                | _Node::DivR { rhs: index, .. }
                | _Node::Exp { index, .. }
                | _Node::Log { index, .. }
                | _Node::Erf { index, .. }
                | _Node::Sqrt { index, .. }
                | _Node::Powi { index, .. } => {
                    self.vacancy.push(idx);
                    stack.push(*index);
                }

                // binary arithmetic
                _Node::Add { lhs, rhs, .. }
                | _Node::Sub { lhs, rhs, .. }
                | _Node::Mul { lhs, rhs, .. }
                | _Node::Div { lhs, rhs, .. } => {
                    self.vacancy.push(idx);
                    stack.push(*lhs);
                    stack.push(*rhs);
                }

                // unary elementary functions

                // multi-ary
                _Node::Compressed { .. } => {
                    self.vacancy.push(idx);
                    // we don't need to decrement the reference count of the children
                    // because they are variables and they are not cleaned up
                }
            }
        }
    }

    #[inline]
    fn _make_unary<F>(&mut self, index: usize, f: F) -> usize
    where
        F: FnOnce(&V) -> _Node<V>,
    {
        let val = self._cell(index).value();
        let node = f(val);
        self._incl_refcnt(index);
        self._reg_node(node)
    }

    #[inline]
    fn _make_binary<F>(&mut self, lhs: usize, rhs: usize, f: F) -> usize
    where
        F: FnOnce(&V, &V) -> _Node<V>,
    {
        let lhs_val = self._cell(lhs).value();
        let rhs_val = self._cell(rhs).value();
        let node = f(lhs_val, rhs_val);
        self._incl_refcnt(lhs);
        self._incl_refcnt(rhs);
        self._reg_node(node)
    }
}

// -----------------------------------------------------------------------------
// Scalar
// -----------------------------------------------------------------------------
pub(crate) struct Scalar<V>(pub V);

//
// arithmetic operations
//
impl<K, V> std::ops::Neg for &Node<K, V>
where
    V: Clone + std::ops::Neg<Output = V>,
{
    type Output = Node<K, V>;

    #[inline]
    fn neg(self) -> Self::Output {
        let mut internal = self.graph.0.borrow_mut();
        Node {
            index: internal.tape._make_unary(self.index, |val| _Node::Neg {
                value: std::ops::Neg::neg(val.clone()),
                index: self.index,
            }),
            graph: self.graph.clone(),
        }
    }
}

macro_rules! _define_arithmetic_binary {
    ($tr:ident, $fn:ident, $node:ident, $node_l:ident, $node_r:ident) => {
        impl<K, V> std::ops::$tr<&Node<K, V>> for &Node<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Node<K, V>;

            #[inline]
            fn $fn(self, rhs: &Node<K, V>) -> Self::Output {
                if !Graph::ptr_eq(&self.graph, &rhs.graph) {
                    panic!(
                        "Cannot {} nodes from different tapes: lhs.tape={:?}, rhs.tape={:?}",
                        stringify!($tr),
                        self.graph._debug_ptr(),
                        rhs.graph._debug_ptr()
                    );
                }
                let mut internal = self.graph.0.borrow_mut();
                Node {
                    index: internal
                        .tape
                        ._make_binary(self.index, rhs.index, |lval, rval| _Node::$node {
                            value: std::ops::$tr::$fn(lval.clone(), rval),
                            lhs: self.index,
                            rhs: rhs.index,
                        }),
                    graph: self.graph.clone(),
                }
            }
        }
        impl<K, V> std::ops::$tr<Scalar<&V>> for &Node<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Node<K, V>;

            #[inline]
            fn $fn(self, rhs: Scalar<&V>) -> Self::Output {
                let mut internal = self.graph.0.borrow_mut();
                Node {
                    index: internal
                        .tape
                        ._make_unary(self.index, |lval| _Node::$node_l {
                            value: std::ops::$tr::$fn(lval.clone(), rhs.0),
                            lhs: self.index,
                            rhs: rhs.0.clone(),
                        }),
                    graph: self.graph.clone(),
                }
            }
        }
        impl<K, V> std::ops::$tr<&Node<K, V>> for Scalar<V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Node<K, V>;

            #[inline]
            fn $fn(self, rhs: &Node<K, V>) -> Self::Output {
                let mut internal = rhs.graph.0.borrow_mut();
                Node {
                    index: internal.tape._make_unary(rhs.index, |rval| _Node::$node_r {
                        value: std::ops::$tr::$fn(self.0.clone(), rval),
                        lhs: self.0,
                        rhs: rhs.index,
                    }),
                    graph: rhs.graph.clone(),
                }
            }
        }
        impl<K, V> std::ops::$tr<&Node<K, V>> for Scalar<&V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Node<K, V>;

            #[inline]
            fn $fn(self, rhs: &Node<K, V>) -> Self::Output {
                let mut internal = rhs.graph.0.borrow_mut();
                Node {
                    index: internal.tape._make_unary(rhs.index, |rval| _Node::$node_r {
                        value: std::ops::$tr::$fn(self.0.clone(), rval),
                        lhs: self.0.clone(),
                        rhs: rhs.index,
                    }),
                    graph: rhs.graph.clone(),
                }
            }
        }
    };
}

_define_arithmetic_binary!(Add, add, Add, AddL, AddR);
_define_arithmetic_binary!(Sub, sub, Sub, SubL, SubR);
_define_arithmetic_binary!(Mul, mul, Mul, MulL, MulR);
_define_arithmetic_binary!(Div, div, Div, DivL, DivR);

//
// unary elementary functions
//
macro_rules! _define_elementary_unary {
    ($tr:ident, $fn:ident, $node:ident) => {
        impl<K, V> qmath::num::$tr for Node<K, V>
        where
            V: Clone + qmath::num::$tr<Output = V>,
        {
            type Output = Node<K, V>;

            #[inline]
            fn $fn(self) -> Self::Output {
                let mut internal = self.graph.0.borrow_mut();
                Node {
                    index: internal.tape._make_unary(self.index, |val| _Node::$node {
                        value: qmath::num::$tr::$fn(val.clone()),
                        index: self.index,
                    }),
                    graph: self.graph.clone(),
                }
            }
        }
    };
}

_define_elementary_unary!(Exp, exp, Exp);
_define_elementary_unary!(Log, log, Log);
_define_elementary_unary!(Erf, erf, Erf);
_define_elementary_unary!(Sqrt, sqrt, Sqrt);

impl<K, V> qmath::num::Powi for Node<K, V>
where
    V: Clone + qmath::num::Powi<Output = V>,
{
    type Output = Node<K, V>;

    #[inline]
    fn powi(self, exp: i32) -> Self::Output {
        let mut internal = self.graph.0.borrow_mut();
        Node {
            index: internal.tape._make_unary(self.index, |val| _Node::Powi {
                value: qmath::num::Powi::powi(val.clone(), exp),
                index: self.index,
                exp,
            }),
            graph: self.graph.clone(),
        }
    }
}

// -----------------------------------------------------------------------------
// _GraphvizNodeIdx
// _GraphvizNode
// _GraphvizGraph
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
enum _GraphvizNodeIdx {
    Node(usize),
    Cell(usize),
}

#[derive(Debug, Clone)]
enum _GraphvizNode<K, V> {
    Const { value: V },
    Var { key: K, value: V, grad: V },
    Node { op: String, value: V, grad: V },
}

struct _GraphvizGraph<K, V> {
    cell2node: BTreeMap<usize, usize>,
    nodes: Vec<_GraphvizNode<K, V>>,
    edges: Vec<(_GraphvizNodeIdx, _GraphvizNodeIdx, Option<String>)>,
}

impl<K, V> Default for _GraphvizGraph<K, V> {
    #[inline]
    fn default() -> Self {
        Self {
            cell2node: BTreeMap::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl<K, V> _GraphvizGraph<K, V> {
    fn _unary(
        &mut self,
        op: &str,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
        edge: Option<String>,
    ) where
        V: Clone,
    {
        self.nodes.push(_GraphvizNode::Node {
            op: op.to_string(),
            value: value.clone(),
            grad: grad.clone(),
        });
        self.cell2node.insert(cell_idx, self.nodes.len() - 1);
        self.edges.push((
            _GraphvizNodeIdx::Cell(arg),
            _GraphvizNodeIdx::Node(self.nodes.len() - 1),
            edge,
        ));
    }
    #[allow(clippy::too_many_arguments)]
    fn _binary_partial(
        &mut self,
        op: &str,
        cell_idx: usize,
        cnst: &V,
        arg: usize,
        value: &V,
        grad: &V,
        arg_edge: Option<String>,
        cnst_edge: Option<String>,
    ) where
        V: Clone,
    {
        self.nodes.push(_GraphvizNode::Node {
            op: op.to_string(),
            value: value.clone(),
            grad: grad.clone(),
        });
        self.nodes.push(_GraphvizNode::Const {
            value: cnst.clone(),
        });
        self.cell2node.insert(cell_idx, self.nodes.len() - 2);
        self.edges.push((
            _GraphvizNodeIdx::Node(self.nodes.len() - 1),
            _GraphvizNodeIdx::Node(self.nodes.len() - 2),
            cnst_edge,
        ));
        self.edges.push((
            _GraphvizNodeIdx::Cell(arg),
            _GraphvizNodeIdx::Node(self.nodes.len() - 2),
            arg_edge,
        ));
    }

    #[allow(clippy::too_many_arguments)]
    fn _binary(
        &mut self,
        op: &str,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
        lhs_edge: Option<String>,
        rhs_edge: Option<String>,
    ) where
        V: Clone,
    {
        self.nodes.push(_GraphvizNode::Node {
            op: op.to_string(),
            value: value.clone(),
            grad: grad.clone(),
        });
        self.cell2node.insert(cell_idx, self.nodes.len() - 1);
        self.edges.push((
            _GraphvizNodeIdx::Cell(lhs),
            _GraphvizNodeIdx::Node(self.nodes.len() - 1),
            lhs_edge,
        ));
        self.edges.push((
            _GraphvizNodeIdx::Cell(rhs),
            _GraphvizNodeIdx::Node(self.nodes.len() - 1),
            rhs_edge,
        ));
    }
}

impl<K, V> _BackProp<K, V> for _GraphvizGraph<K, V>
where
    K: Clone,
    V: Clone,
{
    type Error = Infallible;

    #[inline]
    fn _on_var(
        &mut self,
        cell_idx: usize,
        _: usize,
        key: &K,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self.nodes.push(_GraphvizNode::Var {
            key: key.clone(),
            value: value.clone(),
            grad: grad.clone(),
        });
        self.cell2node.insert(cell_idx, self.nodes.len() - 1);
        Ok(())
    }

    #[inline]
    fn _on_neg(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._unary("-", cell_idx, arg, value, grad, None);
        Ok(())
    }

    #[inline]
    fn _on_addl(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "+",
            cell_idx,
            rhs,
            lhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_addr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "+",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "R".to_string().into(),
            "L".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_subl(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "-",
            cell_idx,
            rhs,
            lhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_subr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "-",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "R".to_string().into(),
            "L".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_mull(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "*",
            cell_idx,
            rhs,
            lhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_mulr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "*",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "R".to_string().into(),
            "L".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_divl(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: &V,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "/",
            cell_idx,
            rhs,
            lhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_divr(
        &mut self,
        cell_idx: usize,
        lhs: &V,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary_partial(
            "/",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "R".to_string().into(),
            "L".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_exp(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._unary("exp", cell_idx, arg, value, grad, None);
        Ok(())
    }

    #[inline]
    fn _on_log(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._unary("log", cell_idx, arg, value, grad, None);
        Ok(())
    }

    #[inline]
    fn _on_erf(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._unary("erf", cell_idx, arg, value, grad, None);
        Ok(())
    }

    #[inline]
    fn _on_sqrt(
        &mut self,
        cell_idx: usize,
        arg: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._unary("sqrt", cell_idx, arg, value, grad, None);
        Ok(())
    }

    #[inline]
    fn _on_add(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary(
            "+",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_sub(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary(
            "-",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_mul(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary(
            "*",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }

    #[inline]
    fn _on_div(
        &mut self,
        cell_idx: usize,
        lhs: usize,
        rhs: usize,
        value: &V,
        grad: &V,
    ) -> Result<(), Self::Error> {
        self._binary(
            "/",
            cell_idx,
            lhs,
            rhs,
            value,
            grad,
            "L".to_string().into(),
            "R".to_string().into(),
        );
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// GraphvizBuilder
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct GraphvizBuilder<K, V, KeyFmt, ValFmt> {
    nodes: Vec<_GraphvizNode<K, V>>,
    edges: Vec<(usize, usize, Option<String>)>,
    name: String,
    graph_global_settings: BTreeMap<String, String>,
    node_global_settings: BTreeMap<String, String>,
    key_fmt: KeyFmt,
    value_fmt: ValFmt,
}

impl<K, V, KF> GraphvizBuilder<K, V, KF, ()> {
    #[inline]
    pub fn with_value_formatter<VF>(self, val_fmt: VF) -> GraphvizBuilder<K, V, KF, VF>
    where
        VF: Fn(&V) -> String,
    {
        GraphvizBuilder {
            nodes: self.nodes,
            edges: self.edges,
            name: self.name,
            graph_global_settings: self.graph_global_settings,
            node_global_settings: self.node_global_settings,
            key_fmt: self.key_fmt,
            value_fmt: val_fmt,
        }
    }
}

impl<K, V, VF> GraphvizBuilder<K, V, (), VF> {
    #[inline]
    pub fn with_key_formatter<KF>(self, key_fmt: KF) -> GraphvizBuilder<K, V, KF, VF>
    where
        KF: Fn(&K) -> String,
    {
        GraphvizBuilder {
            nodes: self.nodes,
            edges: self.edges,
            name: self.name,
            graph_global_settings: self.graph_global_settings,
            node_global_settings: self.node_global_settings,
            key_fmt,
            value_fmt: self.value_fmt,
        }
    }
}

impl<K, V, KeyFmt, ValFmt> GraphvizBuilder<K, V, KeyFmt, ValFmt> {
    /// Set the name of the graph.
    #[inline]
    pub fn with_name(&mut self, name: &str) -> &mut Self {
        self.name = name.to_string();
        self
    }

    /// Set a global setting for the graph.
    #[inline]
    pub fn with_graph_setting(&mut self, key: &str, value: &str) -> &mut Self {
        self.graph_global_settings
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Set a global setting for the node.
    #[inline]
    pub fn with_node_setting(&mut self, key: &str, value: &str) -> &mut Self {
        self.node_global_settings
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Generate a dot file.
    pub fn gen_dot(&self) -> String
    where
        KeyFmt: Fn(&K) -> String,
        ValFmt: Fn(&V) -> String,
    {
        let mut buf = String::new();

        buf += &format!("digraph {} {{\n", self.name);

        // graph settings
        buf += "  graph [\n";
        for (key, value) in &self.graph_global_settings {
            buf.push_str(&format!("    {}={};\n", key, value));
        }
        buf += "  ];\n\n";

        // node settings
        buf += "  node [\n";
        for (key, value) in &self.node_global_settings {
            buf.push_str(&format!("    {}={};\n", key, value));
        }
        buf += "  ];\n\n";

        // nodes
        buf += "  // nodes\n";
        for (idx, node) in self.nodes.iter().enumerate() {
            match node {
                _GraphvizNode::Const { value } => {
                    let annotations = format!(
                        "label=\"{{value={}}}\", shape=record",
                        (self.value_fmt)(value)
                    );
                    buf.push_str(&format!("  {idx} [{annotations}];\n"));
                }
                _GraphvizNode::Var { key, value, grad } => {
                    let annotations = format!("label=\"{key}|{{value={value}|grad={grad}}}\", shape=record, style=\"diagonals\"",
                        key = (self.key_fmt)(key),
                        value = (self.value_fmt)(value),
                        grad = (self.value_fmt)(grad),
                    );
                    buf.push_str(&format!("  {idx} [{annotations}];\n"));
                }
                _GraphvizNode::Node { op, value, grad } => {
                    let annotations = format!(
                        "label=\"{op}|{{value={value}|grad={grad}}}\", shape=record",
                        op = op,
                        value = (self.value_fmt)(value),
                        grad = (self.value_fmt)(grad),
                    );
                    buf.push_str(&format!("  {idx} [{annotations}];\n"));
                }
            }
        }
        buf += "\n";

        // edges
        buf += "  // edges\n";
        for (src, dst, label) in &self.edges {
            let edge = match label {
                Some(label) => format!("  {src} -> {dst} [label=\"{label}\"];\n"),
                None => format!("  {src} -> {dst};\n"),
            };
            buf.push_str(&edge);
        }

        buf.push_str("}\n");
        buf
    }
}

#[cfg(test)]
mod tests {
    use qmath::num::{Erf, Exp, Log, Powi, Sqrt};

    use super::*;

    //
    // memory management related tests.
    // alghough this module only has private functions,
    // memory management is critical for the correctness of the library
    // so we test it here.
    //
    #[test]
    fn test_refcnt_var() {
        let graph = Graph::new();

        {
            let x1 = graph.create_var("42", 4.2f64).unwrap();
            let x2 = graph.create_var("43", 4.3f64).unwrap();
            let x3 = x1.clone();

            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.vars.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 1);
            assert_eq!(graph.0.borrow().tape.vars[0].cell_idx, 0);
            assert_eq!(graph.0.borrow().tape.vars[0].key, "42");
            assert_eq!(graph.0.borrow().tape.vars[1].cell_idx, 1);
            assert_eq!(graph.0.borrow().tape.vars[1].key, "43");
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x2, x3);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.vars.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vars[0].cell_idx, 0);
        assert_eq!(graph.0.borrow().tape.vars[0].key, "42");
        assert_eq!(graph.0.borrow().tape.vars[1].cell_idx, 1);
        assert_eq!(graph.0.borrow().tape.vars[1].key, "43");
        // even though the variables are already dropped,
        // their cells are not cleaned up because they are variables
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
    }

    #[test]
    fn test_refcnt_neg() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = -x1.as_ref();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_addl() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref() + Expr::from(3.0);
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_addr() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = Expr::from(3.0) + x1.as_ref();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_subl() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref() - Expr::from(3.0);
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_subr() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = Expr::from(3.0) - x1.as_ref();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_mull() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref() * Expr::from(3.0);
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_mulr() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = Expr::from(3.0) * x1.as_ref();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_divl() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref() / Expr::from(3.0);
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_divr() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = Expr::from(3.0) / x1.as_ref();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_exp() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref().clone().exp();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_log() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref().clone().log();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_erf() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref().clone().erf();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_sqrt() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref().clone().sqrt();
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_powi() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        {
            let x2 = x1.as_ref().clone().powi(5);
            let x3 = x2.clone();
            let x4 = x2.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 2);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x3, x4);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 2);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 1);
    }

    #[test]
    fn test_refcnt_add() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        let x2 = graph.create_var("43", 4.3f64).unwrap();
        {
            let x3 = x1.as_ref() + x2.as_ref();
            let x4 = x3.clone();
            let x5 = x3.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 3);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x4, x5);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 3);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 2);
    }

    #[test]
    fn test_refcnt_sub() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        let x2 = graph.create_var("43", 4.3f64).unwrap();
        {
            let x3 = x1.as_ref() - x2.as_ref();
            let x4 = x3.clone();
            let x5 = x3.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 3);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x4, x5);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 3);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 2);
    }

    #[test]
    fn test_refcnt_mul() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        let x2 = graph.create_var("43", 4.3f64).unwrap();
        {
            let x3 = x1.as_ref() * x2.as_ref();
            let x4 = x3.clone();
            let x5 = x3.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 3);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x4, x5);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 3);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 2);
    }

    #[test]
    fn test_refcnt_div() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        let x2 = graph.create_var("43", 4.3f64).unwrap();
        {
            let x3 = x1.as_ref() / x2.as_ref();
            let x4 = x3.clone();
            let x5 = x3.clone();
            assert_eq!(graph.0.borrow().tape.cells.len(), 3);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x4, x5);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 3);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 1);
        assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 2);
    }

    #[test]
    fn test_refcnt_recursive_decl() {
        let graph = Graph::new();

        {
            let x1 = graph.create_var("42", 4.2f64).unwrap();
            let x2 = x1.as_ref() + x1.as_ref();
            let x3 = x2.clone() + x2;
            let x4 = x3.clone();
            let x5 = x3.clone();

            assert_eq!(graph.0.borrow().tape.cells.len(), 3);
            assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 2);
            assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 3);
            assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
            let _ = (x4, x5);
        }

        assert_eq!(graph.0.borrow().tape.cells.len(), 3);
        assert_eq!(graph.0.borrow().tape.cells[0].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.cells[1].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 0);
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 2);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 2);
        assert_eq!(graph.0.borrow().tape.vacancy[1], 1);
    }

    #[test]
    fn test_reuse_vacant_cell() {
        let graph = Graph::new();

        let x1 = graph.create_var("42", 4.2f64).unwrap();
        let x2 = graph.create_var("43", 4.3f64).unwrap();
        {
            let x3 = x1.as_ref() + x2.as_ref();
            let _ = x3;
        }
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 1);
        assert_eq!(graph.0.borrow().tape.vacancy[0], 2);

        let x3 = x1.as_ref() - x2.as_ref();

        // vacancy is popped and cells[2] is reused
        assert_eq!(graph.0.borrow().tape.cells.len(), 3);
        assert_eq!(graph.0.borrow().tape.cells[2].refcnt, 1);
        approx::assert_abs_diff_eq!(
            graph.0.borrow().tape.cells[2].node.value(),
            &-0.1f64,
            epsilon = 1e-8
        );
        assert_eq!(graph.0.borrow().tape.vacancy.len(), 0);
        let _ = x3;
    }
}
