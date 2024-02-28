use stones_core::NodeId;

pub struct ClientRequest<C> {
    command: C,
}

pub enum ClientResponse {
    Commited,
    Aborted,
    Redirect(NodeId),
}
