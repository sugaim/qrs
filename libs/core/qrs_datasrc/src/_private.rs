use std::sync::{Mutex, Weak};

use crate::{Observer, StateId};

// -----------------------------------------------------------------------------
// _PassThroughUnary
//
#[derive(Debug)]
pub struct _PassThroughUnary {
    state: StateId,
    obs: Vec<Weak<Mutex<dyn Observer>>>,
    pass_state: bool,
}

//
// construction
//
impl Default for _PassThroughUnary {
    #[inline]
    fn default() -> Self {
        _PassThroughUnary {
            state: StateId::gen(),
            obs: Vec::new(),
            pass_state: false,
        }
    }
}

impl _PassThroughUnary {
    #[inline]
    pub fn pass_state(mut self) -> Self {
        self.pass_state = true;
        self
    }
}

//
// methods
//
impl _PassThroughUnary {
    #[inline]
    pub fn reg_observer(&mut self, observer: Weak<Mutex<dyn Observer>>) {
        self.obs.retain(|o| o.upgrade().is_some());
        if observer.upgrade().is_none() {
            return;
        }
        self.obs.push(observer);
    }

    #[inline]
    pub fn rm_observer(&mut self, observer: &Weak<Mutex<dyn Observer>>) {
        self.obs
            .retain(|o| !o.ptr_eq(observer) && o.upgrade().is_some());
    }
}

impl Observer for _PassThroughUnary {
    fn receive(&mut self, subject_state: &StateId) {
        let state = if self.pass_state {
            *subject_state
        } else {
            subject_state ^ self.state
        };
        self.obs.iter().filter_map(|o| o.upgrade()).for_each(|o| {
            o.lock().unwrap().receive(&state);
        });
    }
}
