// TODO make an async trait
pub trait StateMachine<C> {
    fn apply(&mut self, command: C) {
        todo!()
    }
}
