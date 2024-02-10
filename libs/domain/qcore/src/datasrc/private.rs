use maplit::btreeset;

use super::{Node, NodeId, NodeInfo, Tree};

// -----------------------------------------------------------------------------
// _Node
//

/// Typical node implementation which does not have any state changing event.
///
/// Since this does not have state changing, this node uses
/// a state id of downstream node as its own state id.
#[derive(Debug)]
pub(super) struct _UnaryPassThroughNode<S> {
    pub(super) src: S,
    pub(super) info: NodeInfo,
}

//
// methods
//
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
        self.info.set_state(*state);
        self.info.notify_all();
    }
}
