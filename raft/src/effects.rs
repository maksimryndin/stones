use core::ops::{Deref, DerefMut};
use futures::channel::mpsc;
use futures::channel::oneshot;

pub(crate) async fn generate_timeout() -> u64 {
    let (sender, receiver) = oneshot::channel::<u64>();
    let _ = sender.send(3);
    receiver.await.unwrap()
}

/// Persistent state
/// Wrapped type can only be accessed and
/// mutated via dereference which tracks
/// mutations
pub(crate) struct Persistence<T: Sized + Clone> {
    wrapped: T,
    tracker: mpsc::UnboundedSender<T>,
}

impl<T: Sized + Clone> Persistence<T> {
    pub(crate) fn new(wrapped: T, tracker: mpsc::UnboundedSender<T>) -> Self {
        Self { wrapped, tracker }
    }

    pub(crate) fn update<'a>(&'a mut self) -> Update<'a, T> {
        Update {
            wrapped: &mut self.wrapped,
            tracker: &mut self.tracker,
        }
    }
}

impl<T: Sized + Clone> Deref for Persistence<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.wrapped
    }
}

pub(crate) struct Update<'a, T: Clone> {
    wrapped: &'a mut T,
    tracker: &'a mut mpsc::UnboundedSender<T>,
}

impl<'a, T: Sized + Clone> Deref for Update<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.wrapped
    }
}

impl<'a, T: Sized + Clone> DerefMut for Update<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.wrapped
    }
}

impl<'a, T: Sized + Clone> Drop for Update<'a, T> {
    fn drop(&mut self) {
        self.tracker
            .unbounded_send(Clone::clone(self.wrapped))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[derive(Clone)]
    struct State {
        value: u8,
    }

    #[tokio::test]
    async fn mutation_of_persistent() {
        let (tracker_tx, mut tracker_rx) = mpsc::unbounded();
        let state = State { value: 0 };
        let mut persistent = Persistence::new(state, tracker_tx);
        assert_eq!(persistent.value, 0);
        persistent.update().value += 1;
        let changed = (tracker_rx.next().await).unwrap();
        assert_eq!(changed.value, 1);
    }
}
