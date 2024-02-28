// pub struct NodeMeta {
//     connection_string: &str,
//     id: Id,
//     algorithm_specific:
// }

// trait Node {
//     fn nodes(&self) -> &[NodeMeta]

//     async fn unicast

//     async fn broadcast

//     async fn cache

//     async fn store

//     async fn shutdown (graceful shutdown)
// }

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct NodeId(String);
