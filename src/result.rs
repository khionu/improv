use std::error::Error;

pub type ActorResult<Err> = Result<ActorOk, ActorErr<Err>>;

#[repr(u8)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorOk {
    /// The actor is done with the current operation.
    Success,
    /// The actor finished doing what it was supposed to do.
    /// The actor should then be dropped.
    GracefulEnd,
}

#[derive(Debug)]
pub enum ActorErr<T> where
    T: Error + Send + Sync
{
    // TODO: Enable when adding monitors
//    /// The actor is reporting an error, but the actor should
//    /// be treated as though the error has been recovered.
//    Reporting(T),
    /// The actor has encountered an error that means the actor
    /// is no longer in a functioning state and should be killed.
    Crashing(T),
}
