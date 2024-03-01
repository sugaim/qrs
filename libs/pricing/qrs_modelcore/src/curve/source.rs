use std::{
    collections::HashMap,
    sync::{Mutex, Weak},
};

use qrs_datasrc::{ext::DebugTree, CacheSize, DataSrc, Observer, PassThroughNode, Subject};

use super::{CompositeCurve, WeightedCurve, YieldCurve};

// -----------------------------------------------------------------------------
// FetchError
//
#[derive(Debug, thiserror::Error)]
pub enum FetchError<D, C> {
    #[error("curve definition source error: {0}")]
    DefSrc(D),
    #[error("curve source error: {0}")]
    CurveSrc(C),
}

// -----------------------------------------------------------------------------
// CurveSrc
//
/// Combine a curve definition source and a curve source into a composite curve source.
#[derive(Debug, DebugTree)]
#[debug_tree(desc_field = "desc")]
pub struct CurveSrc<DefSrc, CrvSrc, C> {
    desc: String,
    #[debug_tree(subtree)]
    def_src: DefSrc,
    #[debug_tree(subtree)]
    crv_src: CrvSrc,
    node: PassThroughNode<String, C>,
}

//
// construction
//
impl<DefSrc, CrvSrc, C> CurveSrc<DefSrc, CrvSrc, C>
where
    DefSrc: DataSrc<Output = HashMap<String, f64>>,
    CrvSrc: DataSrc<Key = str, Output = C>,
    C: 'static + Send,
{
    /// Create a new curve source.
    /// Cache is disabled when `cache_size` is `None`.
    pub fn new(mut def_src: DefSrc, mut crv_src: CrvSrc, cache_size: Option<CacheSize>) -> Self {
        let (node, mut detectors) = PassThroughNode::new(2, cache_size);
        def_src.reg_observer(detectors.pop().unwrap());
        crv_src.reg_observer(detectors.pop().unwrap());
        Self {
            desc: "curve source".to_string(),
            def_src,
            crv_src,
            node,
        }
    }

    /// Add a description
    #[inline]
    pub fn with_desc(self, desc: impl Into<String>) -> Self {
        Self {
            desc: desc.into(),
            ..self
        }
    }
}

impl<DefSrc, CrvSrc, C> Clone for CurveSrc<DefSrc, CrvSrc, C>
where
    DefSrc: DataSrc<Output = HashMap<String, f64>> + Clone,
    CrvSrc: DataSrc<Key = str, Output = C> + Clone,
    C: 'static + Send,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(
            self.def_src.clone(),
            self.crv_src.clone(),
            self.node.cache_size(),
        )
        .with_desc(&self.desc)
    }
}

//
// methods
//
impl<DefSrc, CrvSrc, C> CurveSrc<DefSrc, CrvSrc, C> {
    #[inline]
    pub fn def_src(&self) -> &DefSrc {
        &self.def_src
    }

    #[inline]
    pub fn crv_src(&self) -> &CrvSrc {
        &self.crv_src
    }

    #[inline]
    pub fn inner(&self) -> (&DefSrc, &CrvSrc) {
        (&self.def_src, &self.crv_src)
    }

    #[inline]
    pub fn inner_mut(&mut self) -> (&mut DefSrc, &mut CrvSrc) {
        (&mut self.def_src, &mut self.crv_src)
    }

    #[inline]
    pub fn into_inner(self) -> (DefSrc, CrvSrc) {
        (self.def_src, self.crv_src)
    }
}

impl<DefSrc, CrvSrc, C> Subject for CurveSrc<DefSrc, CrvSrc, C>
where
    C: 'static + Send,
{
    #[inline]
    fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.node.reg_observer(observer);
    }

    #[inline]
    fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.node.rm_observer(observer);
    }
}

impl<DefSrc, CrvSrc, C> DataSrc for CurveSrc<DefSrc, CrvSrc, C>
where
    DefSrc: DataSrc<Output = HashMap<String, f64>>,
    CrvSrc: DataSrc<Key = str, Output = C>,
    C: 'static + Send + YieldCurve + Clone,
{
    type Key = DefSrc::Key;
    type Output = CompositeCurve<C>;
    type Err = FetchError<DefSrc::Err, CrvSrc::Err>;

    #[inline]
    fn req(&self, key: &Self::Key) -> Result<Self::Output, Self::Err> {
        let def = self.def_src.req(key).map_err(FetchError::DefSrc)?;
        let mut components = Vec::with_capacity(def.len());
        for (name, weight) in def {
            let curve = match self.node.get_from_cache(&name) {
                Some(c) => c,
                None => {
                    let c = self.crv_src.req(&name).map_err(FetchError::CurveSrc)?;
                    if self.node.is_caching() {
                        self.node.push_to_cache(name.clone(), c.clone());
                    }
                    c
                }
            };
            components.push(WeightedCurve { weight, curve });
        }
        Ok(CompositeCurve { components })
    }
}
