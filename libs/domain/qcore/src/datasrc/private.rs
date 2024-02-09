use maplit::btreeset;

use super::{Node, NodeId, NodeInfo, NodeStateId, StateRecorder, Tree};

// -----------------------------------------------------------------------------
// _Node
//
#[derive(Debug)]
pub(super) struct _UnaryNode<S> {
    pub(super) src: S,
    pub(super) states: StateRecorder<NodeStateId>,
    pub(super) info: NodeInfo,
}

//
// methods
//
impl<S: Node> Node for _UnaryNode<S> {
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
        let state = self.states.get_or_gen_unwrapped(state);
        self.info.set_state(state);
        self.info.notify_all();
    }
}
