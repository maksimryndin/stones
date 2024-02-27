

pub struct Request<C> {
    command: C,
}

pub enum Response {
    Commited,
    Aborted,
    Redirect(NodeId),
}