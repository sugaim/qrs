use std::{fmt::Display, ops::Deref};

use qmath::{
    ext::num::{One, Zero},
    num::{FloatBased, Powi, Real},
};

use crate::{
    graph::{Grads, Node, Scalar},
    GraphvizBuilder,
};

// -----------------------------------------------------------------------------
// _Expr
// -----------------------------------------------------------------------------
#[derive(Debug)]
enum _Expr<K, V> {
    Const(V),
    Node(Node<K, V>),
}

impl<K, V> Clone for _Expr<K, V>
where
    V: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        match self {
            _Expr::Const(v) => _Expr::Const(v.clone()),
            _Expr::Node(node) => _Expr::Node(node.clone()),
        }
    }
}

impl<K, V> _Expr<K, V> {
    #[inline]
    fn _indirectly_read<R>(&self, f: impl FnOnce(&V) -> R) -> R {
        match self {
            _Expr::Const(v) => f(v),
            _Expr::Node(node) => node._indirectly_read(f),
        }
    }
}

// -----------------------------------------------------------------------------
// Var
// -----------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Var<K, V>(pub(crate) Expr<K, V>);

impl<K, V> Clone for Var<K, V>
where
    V: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K, V> AsRef<Expr<K, V>> for Var<K, V> {
    #[inline]
    fn as_ref(&self) -> &Expr<K, V> {
        &self.0
    }
}
impl<K, V> Deref for Var<K, V> {
    type Target = Expr<K, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> Var<K, V> {
    /// Get the key of the variable
    #[inline]
    pub fn key(&self) -> K
    where
        K: Clone + Eq + std::hash::Hash,
    {
        self.0
            .key()
            .expect("Tape must generate and register this var")
    }
}

// impls
impl<K, V> Var<K, V> {
    #[inline]
    pub(crate) fn _node(&self) -> &Node<K, V> {
        match &self.0 .0 {
            _Expr::Node(node) => node,
            _ => unreachable!(),
        }
    }
}

// -----------------------------------------------------------------------------
// Expr
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct Expr<K, V>(_Expr<K, V>);

impl<K, V> Clone for Expr<K, V>
where
    V: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

//
// ser/de
//
impl<K, V> Display for Expr<K, V>
where
    V: Display,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0._indirectly_read(|v| write!(f, "{}", v))
    }
}

//
// cmp
//
impl<K1, K2, V> PartialEq<Expr<K1, V>> for Expr<K2, V>
where
    V: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Expr<K1, V>) -> bool {
        self.0
            ._indirectly_read(|v1| other.0._indirectly_read(|v2| v1.eq(v2)))
    }
}

impl<K1, V> Eq for Expr<K1, V> where V: Eq {}

impl<K1, K2, V> PartialOrd<Expr<K1, V>> for Expr<K2, V>
where
    V: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &Expr<K1, V>) -> Option<std::cmp::Ordering> {
        self.0
            ._indirectly_read(|v1| other.0._indirectly_read(|v2| v1.partial_cmp(v2)))
    }
}
impl<K1, V> Ord for Expr<K1, V>
where
    V: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            ._indirectly_read(|v1| other.0._indirectly_read(|v2| v1.cmp(v2)))
    }
}

//
// conversion
//
impl<K, V> From<Node<K, V>> for Expr<K, V> {
    #[inline]
    fn from(node: Node<K, V>) -> Self {
        Self(_Expr::Node(node))
    }
}

impl<K, V> From<V> for Expr<K, V> {
    #[inline]
    fn from(v: V) -> Self {
        Self(_Expr::Const(v))
    }
}

impl<K, V> From<Var<K, V>> for Expr<K, V> {
    #[inline]
    fn from(var: Var<K, V>) -> Self {
        var.0
    }
}

//
// methods
//
impl<K, V> Expr<K, V> {
    /// Get the value of the expression
    #[inline]
    pub fn value(&self) -> V
    where
        V: Clone,
    {
        match &self.0 {
            _Expr::Const(v) => v.clone(),
            _Expr::Node(node) => node._indirectly_read(Clone::clone),
        }
    }

    /// Calculate gradients if this expression is not a constant
    #[inline]
    pub fn grads(&self) -> Option<Grads<K, V>>
    where
        V: Real,
    {
        match &self.0 {
            _Expr::Const(_) => None,
            _Expr::Node(node) => Some(node._grads()),
        }
    }

    /// Get the key of the expression.
    /// Only available if this expression is a variable.
    #[inline]
    pub fn key(&self) -> Option<K>
    where
        K: Clone + Eq + std::hash::Hash,
    {
        match &self.0 {
            _Expr::Const(_) => None,
            _Expr::Node(node) => node._key(),
        }
    }

    /// Compress the expression to reduce memory usage, computation time, etc.
    #[inline]
    pub fn compress(self) -> Self
    where
        V: Real,
    {
        match self.0 {
            _Expr::Const(v) => Self::from(v),
            _Expr::Node(node) => node._compress(),
        }
    }

    /// Get the expression as a constant if possible
    #[inline]
    pub fn graphviz(&self) -> Option<GraphvizBuilder<K, V, (), ()>>
    where
        K: Clone,
        V: Real,
    {
        match &self.0 {
            _Expr::Const(_) => None,
            _Expr::Node(node) => Some(node._dotize()),
        }
    }
}

//
// numeric
//
impl<K, V> FloatBased for Expr<K, V>
where
    V: FloatBased,
{
    type BaseFloat = V::BaseFloat;

    fn nearest_base_float_of_f64(v: f64) -> Self::BaseFloat {
        V::nearest_base_float_of_f64(v)
    }
}

impl<K, V> Zero for Expr<K, V>
where
    V: Clone + Zero + for<'a> std::ops::Add<&'a V, Output = V>,
{
    #[inline]
    fn zero() -> Self {
        Self::from(V::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0._indirectly_read(Zero::is_zero)
    }
}

impl<K, V> One for Expr<K, V>
where
    V: Clone + One + for<'a> std::ops::Mul<&'a V, Output = V>,
{
    #[inline]
    fn one() -> Self {
        Self::from(V::one())
    }
}

impl<K, V> std::ops::Neg for Expr<K, V>
where
    V: Clone + std::ops::Neg<Output = V>,
{
    type Output = Expr<K, V>;

    #[inline]
    fn neg(self) -> Self::Output {
        match self.0 {
            _Expr::Const(v) => (-v).into(),
            _Expr::Node(node) => (-&node).into(),
        }
    }
}

impl<K, V> std::ops::Neg for &Expr<K, V>
where
    V: Clone + std::ops::Neg<Output = V>,
{
    type Output = Expr<K, V>;

    #[inline]
    fn neg(self) -> Self::Output {
        -self.clone()
    }
}

macro_rules! _define_arithmetic_binary {
    ($tr:ident, $fn:ident, $ass_tr:ident, $ass_fn: ident) => {
        impl<K, V> std::ops::$tr<Expr<K, V>> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: Expr<K, V>) -> Self::Output {
                std::ops::$tr::$fn(self, &rhs)
            }
        }
        impl<K, V> std::ops::$tr<V> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: V) -> Self::Output {
                std::ops::$tr::$fn(self, &rhs)
            }
        }
        impl<K, V> std::ops::$tr<&Expr<K, V>> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: &Expr<K, V>) -> Self::Output {
                match (self.0, &rhs.0) {
                    (_Expr::Const(lhs), _Expr::Const(rhs)) => std::ops::$tr::$fn(lhs, &rhs).into(),
                    (_Expr::Const(lhs), _Expr::Node(rhs)) => {
                        std::ops::$tr::$fn(Scalar(lhs), &rhs).into()
                    }
                    (_Expr::Node(lhs), _Expr::Const(rhs)) => {
                        std::ops::$tr::$fn(&lhs, Scalar(rhs)).into()
                    }
                    (_Expr::Node(lhs), _Expr::Node(rhs)) => std::ops::$tr::$fn(&lhs, rhs).into(),
                }
            }
        }
        impl<K, V> std::ops::$tr<&V> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: &V) -> Self::Output {
                match self.0 {
                    _Expr::Const(lhs) => std::ops::$tr::$fn(lhs, &rhs).into(),
                    _Expr::Node(lhs) => std::ops::$tr::$fn(&lhs, Scalar(rhs)).into(),
                }
            }
        }
        impl<K, V> std::ops::$tr<&Expr<K, V>> for &Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: &Expr<K, V>) -> Self::Output {
                std::ops::$tr::$fn(self.clone(), rhs)
            }
        }
        impl<K, V> std::ops::$tr<&V> for &Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: &V) -> Self::Output {
                std::ops::$tr::$fn(self.clone(), rhs)
            }
        }
        impl<K, V> std::ops::$tr<Expr<K, V>> for &Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: Expr<K, V>) -> Self::Output {
                std::ops::$tr::$fn(self.clone(), rhs)
            }
        }
        impl<K, V> std::ops::$tr<V> for &Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self, rhs: V) -> Self::Output {
                std::ops::$tr::$fn(self.clone(), rhs)
            }
        }
        impl<K, V> std::ops::$ass_tr<Expr<K, V>> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            #[inline]
            fn $ass_fn(&mut self, rhs: Expr<K, V>) {
                *self = std::ops::$tr::$fn(self.clone(), rhs);
            }
        }
        impl<K, V> std::ops::$ass_tr<V> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            #[inline]
            fn $ass_fn(&mut self, rhs: V) {
                *self = std::ops::$tr::$fn(self.clone(), rhs);
            }
        }
        impl<K, V> std::ops::$ass_tr<&Expr<K, V>> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            #[inline]
            fn $ass_fn(&mut self, rhs: &Expr<K, V>) {
                *self = std::ops::$tr::$fn(self.clone(), rhs);
            }
        }
        impl<K, V> std::ops::$ass_tr<&V> for Expr<K, V>
        where
            V: Clone + for<'a> std::ops::$tr<&'a V, Output = V>,
        {
            #[inline]
            fn $ass_fn(&mut self, rhs: &V) {
                *self = std::ops::$tr::$fn(self.clone(), rhs);
            }
        }
    };
}

_define_arithmetic_binary!(Add, add, AddAssign, add_assign);
_define_arithmetic_binary!(Sub, sub, SubAssign, sub_assign);
_define_arithmetic_binary!(Mul, mul, MulAssign, mul_assign);
_define_arithmetic_binary!(Div, div, DivAssign, div_assign);

macro_rules! _define_elementary_unary {
    ($tr:ident, $fn:ident) => {
        impl<K, V> qmath::num::$tr for Expr<K, V>
        where
            V: Clone + qmath::num::$tr<Output = V>,
        {
            type Output = Expr<K, V>;

            #[inline]
            fn $fn(self) -> Self::Output {
                match self.0 {
                    _Expr::Const(v) => qmath::num::$tr::$fn(v).into(),
                    _Expr::Node(node) => qmath::num::$tr::$fn(node).into(),
                }
            }
        }
    };
}

_define_elementary_unary!(Exp, exp);
_define_elementary_unary!(Log, log);
_define_elementary_unary!(Erf, erf);
_define_elementary_unary!(Sqrt, sqrt);

impl<K, V> Powi for Expr<K, V>
where
    V: Clone + Powi<Output = V>,
{
    type Output = Expr<K, V>;

    #[inline]
    fn powi(self, n: i32) -> Self::Output {
        match self.0 {
            _Expr::Const(v) => v.powi(n).into(),
            _Expr::Node(node) => node.powi(n).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::f64;
    use std::collections::HashMap;

    use qmath::num::{Erf, Exp, Log, Sqrt};
    use rstest::rstest;

    use crate::Graph;

    use super::*;

    #[test]
    fn test_is_real() {
        static_assertions::assert_impl_all!(Expr<&str, f32>: Real);
        static_assertions::assert_impl_all!(Expr<&str, f64>: Real);
    }

    #[rstest]
    #[case(0.0)]
    #[case(1.0)]
    #[case(4.0)]
    #[case(-3.5)]
    fn test_fmt(#[case] input: f64) {
        let graph = Graph::new();
        let x: Expr<&str, f64> = graph.create_var("x", input).unwrap().into();

        assert_eq!(format!("{}", x), input.to_string());
    }

    #[rstest]
    #[case(0.0)]
    #[case(1.0)]
    #[case(4.0)]
    #[case(-3.5)]
    fn test_neg(#[case] input: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", input).unwrap();
        let x = x.as_ref();

        let y = -x;
        let grads: HashMap<_, _> = y.grads().unwrap().collect();

        assert_eq!(y.value(), -input);
        assert_eq!(grads.len(), 1);
        assert_eq!(grads[&"x"], -1.0);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_addl(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = Expr::from(rhs);
        let x = x.as_ref();

        let z = x + &y;
        let w = y + x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs + rhs);
        assert_eq!(w.value(), lhs + rhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"x"], 1.0);
        assert_eq!(wgrads[&"x"], 1.0);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_addr(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = Expr::from(lhs);
        let y = graph.create_var("y", rhs).unwrap();
        let y = y.as_ref();

        let z = x.clone() + y;
        let w = y + x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs + rhs);
        assert_eq!(w.value(), lhs + rhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"y"], 1.0);
        assert_eq!(wgrads[&"y"], 1.0);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_subl(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = Expr::from(rhs);
        let x = x.as_ref();

        let z = x - &y;
        let w = y - x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs - rhs);
        assert_eq!(w.value(), rhs - lhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"x"], 1.0);
        assert_eq!(wgrads[&"x"], -1.0);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_subr(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = Expr::from(lhs);
        let y = graph.create_var("y", rhs).unwrap();
        let y = y.as_ref();

        let z = x.clone() - y;
        let w = y - x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs - rhs);
        assert_eq!(w.value(), rhs - lhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"y"], -1.0);
        assert_eq!(wgrads[&"y"], 1.0);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_mull(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = Expr::from(rhs);
        let x = x.as_ref();

        let z = x * &y;
        let w = y * x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs * rhs);
        assert_eq!(w.value(), rhs * lhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"x"], rhs);
        assert_eq!(wgrads[&"x"], rhs);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_mulr(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = Expr::from(lhs);
        let y = graph.create_var("y", rhs).unwrap();
        let y = y.as_ref();

        let z = x.clone() * y;
        let w = y * x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs * rhs);
        assert_eq!(w.value(), rhs * lhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"y"], lhs);
        assert_eq!(wgrads[&"y"], lhs);
    }

    #[rstest]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_divl(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = Expr::from(rhs);
        let x = x.as_ref();

        let z = x / &y;
        let w = y / x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs / rhs);
        assert_eq!(w.value(), rhs / lhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"x"], 1.0 / rhs);
        assert_eq!(wgrads[&"x"], -rhs / (lhs * lhs));
    }

    #[rstest]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_divr(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = Expr::from(lhs);
        let y = graph.create_var("y", rhs).unwrap();
        let y = y.as_ref();

        let z = x.clone() / y;
        let w = y / x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs / rhs);
        assert_eq!(w.value(), rhs / lhs);
        assert_eq!(zgrads.len(), 1);
        assert_eq!(wgrads.len(), 1);
        assert_eq!(zgrads[&"y"], -lhs / (rhs * rhs));
        assert_eq!(wgrads[&"y"], 1.0 / lhs);
    }

    #[rstest]
    #[case(0.0)]
    #[case(1.0)]
    #[case(4.0)]
    #[case(-3.5)]
    fn test_exp(#[case] input: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", input).unwrap();
        let x = x.as_ref();

        let y = x.clone().exp();
        let grads: HashMap<_, _> = y.grads().unwrap().collect();

        assert_eq!(y.value(), input.exp());
        assert_eq!(grads.len(), 1);
        assert_eq!(grads[&"x"], input.exp());
    }

    #[rstest]
    #[case(0.5)]
    #[case(1.0)]
    #[case(4.0)]
    fn test_log(#[case] input: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", input).unwrap();
        let x = x.as_ref();

        let y = x.clone().log();
        let grads: HashMap<_, _> = y.grads().unwrap().collect();

        assert_eq!(y.value(), input.ln());
        assert_eq!(grads.len(), 1);
        assert_eq!(grads[&"x"], 1. / input);
    }

    #[rstest]
    #[case(0.5)]
    #[case(1.0)]
    #[case(4.0)]
    fn test_erf(#[case] input: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", input).unwrap();
        let x = x.as_ref();

        let y = x.clone().erf();
        let grads: HashMap<_, _> = y.grads().unwrap().collect();

        assert_eq!(y.value(), input.erf());
        assert_eq!(grads.len(), 1);
        assert_eq!(
            grads[&"x"],
            2. / f64::consts::PI.sqrt() * (-input * input).exp()
        );
    }

    #[rstest]
    #[case(0.5)]
    #[case(1.0)]
    #[case(4.0)]
    fn test_sqrt(#[case] input: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", input).unwrap();
        let x = x.as_ref();

        let y = x.clone().sqrt();
        let grads: HashMap<_, _> = y.grads().unwrap().collect();

        assert_eq!(y.value(), input.sqrt());
        assert_eq!(grads.len(), 1);
        assert_eq!(grads[&"x"], 1. / (2. * input.sqrt()));
    }

    #[rstest]
    #[case(0.5, 0)]
    #[case(1.0, 0)]
    #[case(4.0, 0)]
    #[case(3.5, 0)]
    #[case(0.5, 2)]
    #[case(1.0, 2)]
    #[case(4.0, 2)]
    #[case(3.5, 2)]
    #[case(0.5, -4)]
    #[case(1.0, -4)]
    #[case(4.0, -4)]
    #[case(3.5, -4)]
    fn test_powi(#[case] input: f64, #[case] exp: i32) {
        let graph = Graph::new();
        let x = graph.create_var("x", input).unwrap();
        let x = x.as_ref();

        let y = x.clone().powi(exp);
        let grads: HashMap<_, _> = y.grads().unwrap().collect();

        assert_eq!(y.value(), input.powi(exp));
        assert_eq!(grads.len(), 1);
        assert_eq!(grads[&"x"], exp as f64 * input.powi(exp - 1));
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_add(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = graph.create_var("y", rhs).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();

        let z = x + y;
        let w = y + x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs + rhs);
        assert_eq!(w.value(), rhs + lhs);
        assert_eq!(zgrads.len(), 2);
        assert_eq!(wgrads.len(), 2);
        assert_eq!(zgrads[&"x"], 1.0);
        assert_eq!(zgrads[&"y"], 1.0);
        assert_eq!(wgrads[&"x"], 1.0);
        assert_eq!(wgrads[&"y"], 1.0);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_sub(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = graph.create_var("y", rhs).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();

        let z = x - y;
        let w = y - x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs - rhs);
        assert_eq!(w.value(), rhs - lhs);
        assert_eq!(zgrads.len(), 2);
        assert_eq!(wgrads.len(), 2);
        assert_eq!(zgrads[&"x"], 1.0);
        assert_eq!(zgrads[&"y"], -1.0);
        assert_eq!(wgrads[&"x"], -1.0);
        assert_eq!(wgrads[&"y"], 1.0);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_mul(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = graph.create_var("y", rhs).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();

        let z = x * y;
        let w = y * x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs * rhs);
        assert_eq!(w.value(), rhs * lhs);
        assert_eq!(zgrads.len(), 2);
        assert_eq!(wgrads.len(), 2);
        assert_eq!(zgrads[&"x"], rhs);
        assert_eq!(zgrads[&"y"], lhs);
        assert_eq!(wgrads[&"x"], rhs);
        assert_eq!(wgrads[&"y"], lhs);
    }

    #[rstest]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_div(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = graph.create_var("y", rhs).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();

        let z = x / y;
        let w = y / x;
        let zgrads: HashMap<_, _> = z.grads().unwrap().collect();
        let wgrads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(z.value(), lhs / rhs);
        assert_eq!(w.value(), rhs / lhs);
        assert_eq!(zgrads.len(), 2);
        assert_eq!(wgrads.len(), 2);
        assert_eq!(zgrads[&"x"], 1.0 / rhs);
        assert_eq!(zgrads[&"y"], -lhs / (rhs * rhs));
        assert_eq!(wgrads[&"x"], -rhs / (lhs * lhs));
        assert_eq!(wgrads[&"y"], 1.0 / lhs);
    }

    #[rstest]
    #[case(0.0)]
    #[case(1.0)]
    #[case(4.0)]
    #[case(-3.5)]
    fn test_cneg(#[case] input: f64) {
        let x = Expr::<&str, f64>::from(input);

        let y = -x;
        let grads = y.grads();

        assert_eq!(y.value(), -input);
        assert!(grads.is_none());
    }

    #[rstest]
    #[case(0.0)]
    #[case(1.0)]
    #[case(4.0)]
    #[case(-3.5)]
    fn test_cexp(#[case] input: f64) {
        let x = Expr::<&str, f64>::from(input);

        let y = x.exp();
        let grads = y.grads();

        assert_eq!(y.value(), input.exp());
        assert!(grads.is_none());
    }

    #[rstest]
    #[case(0.5)]
    #[case(1.0)]
    #[case(4.0)]
    fn test_clog(#[case] input: f64) {
        let x = Expr::<&str, f64>::from(input);

        let y = x.log();
        let grads = y.grads();

        assert_eq!(y.value(), input.ln());
        assert!(grads.is_none());
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_cadd(#[case] lhs: f64, #[case] rhs: f64) {
        let x = Expr::<&str, f64>::from(lhs);
        let y = Expr::<&str, f64>::from(rhs);

        let z = x.clone() + &y;
        let w = y + x;
        let zgrads = z.grads();
        let wgrads = w.grads();

        assert_eq!(z.value(), lhs + rhs);
        assert_eq!(w.value(), rhs + lhs);
        assert!(zgrads.is_none());
        assert!(wgrads.is_none());
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_csub(#[case] lhs: f64, #[case] rhs: f64) {
        let x = Expr::<&str, f64>::from(lhs);
        let y = Expr::<&str, f64>::from(rhs);

        let z = x.clone() - &y;
        let w = y - x;
        let zgrads = z.grads();
        let wgrads = w.grads();

        assert_eq!(z.value(), lhs - rhs);
        assert_eq!(w.value(), rhs - lhs);
        assert!(zgrads.is_none());
        assert!(wgrads.is_none());
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_cmul(#[case] lhs: f64, #[case] rhs: f64) {
        let x = Expr::<&str, f64>::from(lhs);
        let y = Expr::<&str, f64>::from(rhs);

        let z = x.clone() * &y;
        let w = y * x;
        let zgrads = z.grads();
        let wgrads = w.grads();

        assert_eq!(z.value(), lhs * rhs);
        assert_eq!(w.value(), rhs * lhs);
        assert!(zgrads.is_none());
        assert!(wgrads.is_none());
    }

    #[rstest]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(-3.5, -3.5)]
    fn test_cdiv(#[case] lhs: f64, #[case] rhs: f64) {
        let x = Expr::<&str, f64>::from(lhs);
        let y = Expr::<&str, f64>::from(rhs);

        let z = x.clone() / &y;
        let w = y / x;
        let zgrads = z.grads();
        let wgrads = w.grads();

        assert_eq!(z.value(), lhs / rhs);
        assert_eq!(w.value(), rhs / lhs);
        assert!(zgrads.is_none());
        assert!(wgrads.is_none());
    }

    #[test]
    fn test_is_zero() {
        let graph = Graph::new();

        let x = graph.create_var("x", 0.0).unwrap();
        let x = x.as_ref();
        let y = x.clone() + 0.0;
        let grads: HashMap<_, _> = y.grads().unwrap().collect();

        assert!(x.is_zero());
        assert!(y.is_zero());
        assert_eq!(grads.len(), 1);
        assert_eq!(grads[&"x"], 1.0);
    }

    #[test]
    fn test_zero() {
        let zero = Expr::<&str, f64>::zero();
        let grads = zero.grads();

        assert_eq!(zero.value(), 0.0);
        assert!(grads.is_none());
    }

    #[test]
    fn test_one() {
        let one = Expr::<&str, f64>::one();
        let grads = one.grads();

        assert_eq!(one.value(), 1.0);
        assert!(grads.is_none());
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(f64::NAN, 0.0)]
    #[case(f64::INFINITY, 0.0)]
    #[case(f64::NEG_INFINITY, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(f64::NAN, 1.0)]
    #[case(f64::INFINITY, 1.0)]
    #[case(f64::NEG_INFINITY, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(f64::NAN, 4.0)]
    #[case(f64::INFINITY, 4.0)]
    #[case(f64::NEG_INFINITY, 4.0)]
    #[case(-3.5, -3.5)]
    #[case(f64::NAN, -3.5)]
    #[case(f64::INFINITY, -3.5)]
    #[case(f64::NEG_INFINITY, -3.5)]
    #[case(f64::NAN, f64::NAN)]
    #[case(f64::INFINITY, f64::NAN)]
    #[case(f64::NEG_INFINITY, f64::NAN)]
    #[case(f64::INFINITY, f64::INFINITY)]
    #[case(f64::NEG_INFINITY, f64::INFINITY)]
    #[case(f64::NEG_INFINITY, f64::NEG_INFINITY)]
    fn test_eq(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = graph.create_var("y", rhs).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();

        let z = x == y;
        let w = y == x;

        assert_eq!(z, lhs == rhs);
        assert_eq!(w, rhs == lhs);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(1.0, 0.0)]
    #[case(4.0, 0.0)]
    #[case(-3.5, 0.0)]
    #[case(f64::NAN, 0.0)]
    #[case(f64::INFINITY, 0.0)]
    #[case(f64::NEG_INFINITY, 0.0)]
    #[case(1.0, 1.0)]
    #[case(4.0, 1.0)]
    #[case(-3.5, 1.0)]
    #[case(f64::NAN, 1.0)]
    #[case(f64::INFINITY, 1.0)]
    #[case(f64::NEG_INFINITY, 1.0)]
    #[case(4.0, 4.0)]
    #[case(-3.5, 4.0)]
    #[case(f64::NAN, 4.0)]
    #[case(f64::INFINITY, 4.0)]
    #[case(f64::NEG_INFINITY, 4.0)]
    #[case(-3.5, -3.5)]
    #[case(f64::NAN, -3.5)]
    #[case(f64::INFINITY, -3.5)]
    #[case(f64::NEG_INFINITY, -3.5)]
    #[case(f64::NAN, f64::NAN)]
    #[case(f64::INFINITY, f64::NAN)]
    #[case(f64::NEG_INFINITY, f64::NAN)]
    #[case(f64::INFINITY, f64::INFINITY)]
    #[case(f64::NEG_INFINITY, f64::INFINITY)]
    #[case(f64::NEG_INFINITY, f64::NEG_INFINITY)]
    fn test_cmp(#[case] lhs: f64, #[case] rhs: f64) {
        let graph = Graph::new();
        let x = graph.create_var("x", lhs).unwrap();
        let y = graph.create_var("y", rhs).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();

        let z = x.partial_cmp(y);
        let w = y.partial_cmp(x);

        assert_eq!(z, lhs.partial_cmp(&rhs));
        assert_eq!(w, rhs.partial_cmp(&lhs));
    }

    #[test]
    fn test_compound_expr() {
        let graph = Graph::new();
        let x = graph.create_var("x", 1.0).unwrap();
        let y = graph.create_var("y", 2.0).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();
        let z = x - y;

        // w = (x - y) * exp(x - y) + (x * y)^3
        // dw/dx = (x - y + 1) * exp(x - y) + 3 * y * (x * y)^2
        // dw/dy = -(x - y + 1) * exp(x - y) + 3 * x * (x * y)^2
        let w = z.clone() * z.exp() + (x * y).powi(3);
        let val = w.value();
        let grads: HashMap<_, _> = w.grads().unwrap().collect();

        assert_eq!(val, -(-1f64).exp() + 8.);
        assert_eq!(grads.len(), 2);
        assert_eq!(grads[&"x"], 24.);
        assert_eq!(grads[&"y"], 12.);
    }

    #[test]
    fn test_compressed() {
        let graph = Graph::new();
        let x = graph.create_var("x", 1.0).unwrap();
        let y = graph.create_var("y", 2.0).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();
        let z = x + y;
        let w = z.clone() * z.exp() + x * y;
        let val = w.value();
        let grads: HashMap<_, _> = w.grads().unwrap().collect();

        let c = w.compress();
        let cgrads: HashMap<_, _> = c.grads().unwrap().collect();

        assert_eq!(c.value(), val);
        assert_eq!(cgrads, grads);
    }

    #[test]
    fn test_graphviz() {
        let graph = Graph::new();
        let x = graph.create_var("x", 1.0).unwrap();
        let y = graph.create_var("y", 2.0).unwrap();
        let x = x.as_ref();
        let y = y.as_ref();
        let z = x + y;
        let w = z.clone() * z.exp() + x * y * Expr::from(4.2);

        let mut dotbuilder = w
            .graphviz()
            .unwrap()
            .with_key_formatter(std::string::ToString::to_string)
            .with_value_formatter(|n| format!("{:.3}", n));
        let res = dotbuilder
            .with_graph_setting("splines", "spline")
            .with_graph_setting("bgcolor", "\"#ffffff\"")
            .with_graph_setting("rankdir", "BT")
            .with_name("HogeGraph")
            .with_node_setting("color", "7")
            .gen_dot();

        assert_eq!(
            res,
            r##"digraph HogeGraph {
  graph [
    bgcolor="#ffffff";
    rankdir=BT;
    splines=spline;
  ];

  node [
    color=7;
  ];

  // nodes
  0 [label="+|{value=68.657|grad=1.000}", shape=record];
  1 [label="*|{value=8.400|grad=1.000}", shape=record];
  2 [label="{value=4.200}", shape=record];
  3 [label="*|{value=2.000|grad=4.200}", shape=record];
  4 [label="*|{value=60.257|grad=1.000}", shape=record];
  5 [label="exp|{value=20.086|grad=3.000}", shape=record];
  6 [label="+|{value=3.000|grad=80.342}", shape=record];
  7 [label="y|{value=2.000|grad=84.542}", shape=record, style="diagonals"];
  8 [label="x|{value=1.000|grad=88.742}", shape=record, style="diagonals"];

  // edges
  1 -> 0 [label="R"];
  2 -> 1 [label="R"];
  3 -> 1 [label="L"];
  4 -> 0 [label="L"];
  5 -> 4 [label="R"];
  6 -> 4 [label="L"];
  6 -> 5;
  7 -> 3 [label="R"];
  7 -> 6 [label="R"];
  8 -> 3 [label="L"];
  8 -> 6 [label="L"];
}
"##
        )
    }
}
