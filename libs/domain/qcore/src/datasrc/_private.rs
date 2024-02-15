use std::sync::{Arc, Mutex, Weak};

use super::{Listener, NodeId, Notifier, PublisherState, StateId};

// -----------------------------------------------------------------------------
// _Node
//

/// Typical node implementation which does not have any state changing event.
#[derive(Debug)]
pub(super) struct _UnaryPassThroughNode {
    info: PublisherState,
    src_id: NodeId,
    self_state: StateId, // invariant becuase this node itself does not have state changing event
}

//
// construction
//
impl _UnaryPassThroughNode {
    /// Create a new instance and register it to the source.
    pub(super) fn new_and_reg<S: Notifier>(
        desc: impl Into<String>,
        src: &mut S,
    ) -> Arc<Mutex<Self>> {
        let self_state = StateId::gen();
        let res = Arc::new(Mutex::new(Self {
            info: PublisherState::new(desc),
            src_id: src.id(),
            self_state,
        }));
        let subsc = Arc::downgrade(&res);
        let state = src.accept_listener(subsc) ^ self_state;
        res.lock().unwrap().info.set_state(state);
        res
    }
}

//
// methods
//
impl _UnaryPassThroughNode {
    #[inline]
    pub(super) fn desc(&self) -> String {
        self.info.desc().into()
    }
    #[inline]
    pub(super) fn state(&self) -> StateId {
        self.info.state()
    }
    #[inline]
    pub(super) fn accept_subscriber(&mut self, subsc: Weak<Mutex<dyn Listener>>) -> StateId {
        self.info.accept_listener(subsc)
    }
    #[inline]
    pub(super) fn remove_subscriber(&mut self, id: &NodeId) {
        self.info.remove_listener(id);
    }
}

impl Listener for _UnaryPassThroughNode {
    #[inline]
    fn id(&self) -> NodeId {
        self.info.id()
    }
    #[inline]
    fn listen(&mut self, id: &NodeId, state: &StateId) {
        if id != &self.src_id {
            return;
        }
        self.info.set_state(state ^ self.self_state);
    }
}
