use stones_core::NodeId;

pub struct ClientRequest<C> {
    pub command: C,
}

pub enum ClientResponse {
    Commited,
    Aborted,
    Redirect(NodeId),
}
