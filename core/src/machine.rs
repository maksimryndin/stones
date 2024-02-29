// TODO make an async trait, return result? Or is it an implementor responsibility?
pub trait StateMachine<C> {
    fn apply(&mut self, command: C);
}
