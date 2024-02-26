use futures::channel::oneshot;

pub(crate) async fn generate_timeout() -> u64 {
    let (sender, receiver) = oneshot::channel::<u64>();
    let _ = sender.send(3);
    receiver.await.unwrap()
}
