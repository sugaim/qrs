use crate::{Observer, StateId};

// -----------------------------------------------------------------------------
// _OnMsg
//
pub struct _OnChange<F> {
    f: F,
}

//
// construction
//
impl<F> _OnChange<F> {
    #[inline]
    pub fn new(f: F) -> Self {
        _OnChange { f }
    }
}

impl<F> Observer for _OnChange<F>
where
    F: 'static + Send + Sync + Fn(&StateId),
{
    #[inline]
    fn receive(&mut self, new_state: &StateId) {
        (self.f)(new_state);
    }
}
