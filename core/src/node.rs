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

trait<C: ConnectInfo, B> Transport {
    async fn unicast(&mut self, conn: C, payload: B);

    async fn broadcast(&mut self, &[(conn: C, payload: B)]);

    async fn router(&mut self)
}

pub trait ConnectInfo {}