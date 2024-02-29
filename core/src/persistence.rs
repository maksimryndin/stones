#[derive(Debug)]
pub struct FailedToSave;

// TODO make an async trait
pub trait PersistenceLayer<S> {
    fn load(&self) -> S;
    fn save(&mut self, data: S) -> Result<(), FailedToSave>;
}
