use std::sync::Arc;

use maplit::btreeset;

use super::{Node, NodeId, NodeInfo, NodeStateId, Tree};

// -----------------------------------------------------------------------------
// _Node
//

/// Typical node implementation which does not have any state changing event.
#[derive(Debug)]
pub(super) struct _UnaryPassThroughNode<S> {
    pub(super) src: S,
    info: NodeInfo,
    self_state: NodeStateId, // invariant becuase this node itself does not have state changing event
}

//
// construction
//
impl<S: Node> _UnaryPassThroughNode<S> {
    #[inline]
    pub(super) fn new(src: S, desc: impl Into<String>) -> Arc<Self> {
        let res = Arc::new(Self {
            src,
            info: NodeInfo::new(desc),
            self_state: NodeStateId::gen(),
        });
        let subsc = Arc::downgrade(&res);
        let downstream_state = res.src.accept_subscriber(subsc);
        res.info.set_state(downstream_state ^ res.self_state);
        res
    }

    #[inline]
    pub(super) fn desc(&self) -> &str {
        self.info.desc()
    }
}

//
// methods
//
impl<S> _UnaryPassThroughNode<S> {
    #[inline]
    pub(super) fn state(&self) -> NodeStateId {
        self.info.state()
    }
}

impl<S: Node> Node for _UnaryPassThroughNode<S> {
    #[inline]
    fn id(&self) -> NodeId {
        self.info.id()
    }

    #[inline]
    fn tree(&self) -> super::Tree {
        Tree::Branch {
            desc: self.info.desc().to_owned(),
            id: self.id(),
            state: self.info.state(),
            children: btreeset![self.src.tree()],
        }
    }

    #[inline]
    fn accept_subscriber(&self, subscriber: std::sync::Weak<dyn Node>) -> super::NodeStateId {
        self.info.accept_subscriber(subscriber)
    }

    #[inline]
    fn remove_subscriber(&self, subscriber: &NodeId) {
        self.info.remove_subscriber(subscriber)
    }

    #[inline]
    fn subscribe(&self, publisher: &super::NodeId, state: &super::NodeStateId) {
        if publisher != &self.src.id() {
            return;
        }
        self.info.set_state(state ^ self.self_state);
    }
}
