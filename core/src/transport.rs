use crate::node::NodeId;

// TODO make an async trait
pub trait Transport<B, P> {
    fn unicast(&mut self, node_id: NodeId, payload: B);

    fn mutlicast(&mut self, &[(NodeId, B)]);

    fn accept(&mut self) -> (NodeId, P);
}