use std::{
    any::TypeId,
    error::Error,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use async_trait::async_trait;
use futures::channel::mpsc::UnboundedSender;

pub use result::{ActorErr, ActorOk, ActorResult};

mod result;
mod utils;

#[cfg(feature = "tokio_impl")]
pub mod tokio_impl;

/// This is the reference that should be cloned and passed around.
/// Anything that needs to send to an Actor should have a clone of
/// the corresponding ActorRef<T>.
#[derive(Debug)]
pub struct ActorRef<T: Actor + 'static> {
    id: u64,
    r#type: TypeId, // TODO: Is this still needed?
    tx: Arc<UnboundedSender<T::Msg>>,
    state: Arc<RwLock<ActorState>>, // TODO: Should I use AtomicU8 instead?
}

/// State of the Actor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorState {
    /// The Actor is healthy, running, and listening.
    Healthy,
    /// The Actor is stopped, and did so without error.
    Stopped,
    /// The Actor is stopped, but did so as the result of
    /// an unrecoverable error.
    Crashed,
}

/// The Actor Trait.
#[async_trait]
pub trait Actor: Send + Sync {
    type Msg: Send + Sync;
    type Err: Error + Send + Sync;

    /// This is ran synchronously after an Actor is given
    /// to the ActorSystem.
    async fn start(&mut self) -> ActorResult<Self::Err> { Ok(ActorOk::Success) }

    /// The handle that the ActorSystem invokes when a message is
    /// sent to the Actor. This will get wrapped in an ActorFuture
    async fn handle(&mut self, msg: Self::Msg) -> ActorResult<Self::Err>;

    /// This is ran synchronously on request through the ActorSystem.
    /// It will be blocked by any current message handles.
    async fn stop(&mut self) -> ActorResult<Self::Err> { Ok(ActorOk::GracefulEnd) }
}

/// The internal driver for the ActorSystem. This defines threading
/// and storage implementations.
#[async_trait]
pub trait ActorSystemDriver {
    async fn register<T>(&self, mut actor: T) -> (ActorRef<T>, Option<T::Err>) where
        T: Actor + 'static;
    fn is_running(&self) -> Arc<AtomicBool>;
    fn stop(&self);
}

/// This is the API that should directly be consumed, rather than the
/// ActorSystemDriver. Implementation-agnostic details will be added
/// here.
pub struct ActorSystem<T: ActorSystemDriver + Sized> {
    is_running: Arc<AtomicBool>,
    inner: Arc<T>,
}

impl<T: Actor + 'static> ActorRef<T> {
    /// Send a message to the Actor to handle
    pub fn send(&self, msg: T::Msg) -> Result<(), ActorState> {
        let g = self.state.read()
            .expect("poisoned actor state guard, report to dev");

        if *g != ActorState::Healthy {
            return Err(g.clone());
        }

        self.tx.unbounded_send(msg)
            .expect("healthy actor has disconnected channel, report to dev");

        Ok(())
    }
}

impl<T: Actor + 'static> PartialEq for ActorRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Actor + 'static> Eq for ActorRef<T> {}

impl<T: ActorSystemDriver> ActorSystem<T> {
    pub fn new(driver: T) -> Self {
        ActorSystem {
            is_running: driver.is_running(),
            inner: Arc::new(driver)
        }
    }

    /// Stop the ActorSystem. Actors will stop processing
    /// messages and the system will be dead. This should
    /// only be used when the entire system is to be
    /// dropped
    pub fn stop(&self) {
        self.inner.stop()
    }

    /// Atomically checks if the ActorSystem is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Registers an Actor to the ActorSystem. This
    /// function will block until after Actor::start has
    /// completed.
    ///
    /// Returns the ActorRef handle and an Option with
    /// any error returned by Actor::start
    pub async fn register<A: Actor + 'static>(&self, actor: A) -> (ActorRef<A>, Option<A::Err>) {
        self.inner.register(actor).await
    }
}

impl<T: Actor + 'static> Clone for ActorRef<T> {
    fn clone(&self) -> Self {
        ActorRef {
            id: self.id,
            r#type: self.r#type,
            tx: self.tx.clone(),
            state: self.state.clone(),
        }
    }
}

impl<T: ActorSystemDriver> Default for ActorSystem<T> where
    T: Default
{
    fn default() -> Self {
        ActorSystem::new(T::default())
    }
}
