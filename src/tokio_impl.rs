use std::{
    any::TypeId,
    sync::{
        Arc, atomic::{AtomicBool, Ordering},
        RwLock,
    },
};

use async_trait::async_trait;
use futures::channel::mpsc::{
    unbounded,
    UnboundedReceiver,
};
use futures::StreamExt;

use crate::{Actor, ActorErr, ActorOk, ActorRef, ActorState, ActorSystemDriver};
use crate::utils::SnowflakeProducer;

/// ActorSystemDriver implementation that uses the user's
/// Tokio runtime to spawn Actors and run them asynchronously
#[derive(Default)]
pub struct TokioActorDriver {
    snowflakes: SnowflakeProducer,
    is_running: Arc<AtomicBool>,
}

#[async_trait]
impl ActorSystemDriver for TokioActorDriver {
    async fn register<T>(&self, mut actor: T) -> (ActorRef<T>, Option<T::Err>) where
        T: Actor + 'static
    {
        let id = self.snowflakes.produce();

        let (tx, rx) = unbounded::<T::Msg>();

        let (state, err) = match actor.start().await {
            Ok(ok) => {
                match ok {
                    ActorOk::Success => (ActorState::Healthy, None),
                    ActorOk::GracefulEnd => (ActorState::Stopped, None),
                }
            }
            Err(err) => {
                match err {
                    // TODO: Enable when adding Monitors
                    // ActorErr::Reporting(e) => (ActorState::Healthy, Some(e)),
                    ActorErr::Crashing(e) => (ActorState::Crashed, Some(e)),
                }
            }
        };

        let state = Arc::new(RwLock::new(state));

        let actor_ref = ActorRef {
            id,
            r#type: TypeId::of::<T>(),
            tx: Arc::new(tx),
            state: state.clone(),
        };

        let state_g = state.read().unwrap();

        if *state_g == ActorState::Healthy {
            let running = self.is_running.clone();

            drop(state_g);

            tokio::spawn(dequeue_for_actor(actor, state, rx, running));
        }

        (actor_ref, err)
    }

    fn is_running(&self) -> Arc<AtomicBool> {
        self.is_running.clone()
    }

    fn stop(&self) {
        self.is_running.swap(false, Ordering::Acquire);
    }
}

async fn dequeue_for_actor<T: Actor + 'static>(mut actor: T, state: Arc<RwLock<ActorState>>,
                                     mut rx: UnboundedReceiver<T::Msg>, is_running: Arc<AtomicBool>) {
    while is_running.load(Ordering::Relaxed) {
        if let Some(msg) = rx.next().await {
            {
                let state_g = state.read()
                    .expect("poisoned actor_state, report to dev");

                if *state_g != ActorState::Healthy {
                    break;
                }
            }

            let handle_result = actor.handle(msg).await;

            {
                let mut state_g = state.write()
                    .expect("poisoned actor_state, report to dev");

                match handle_result {
                    Ok(ok) => {
                        if ok == ActorOk::GracefulEnd {
                            *state_g = ActorState::Stopped;
                        }
                    }
                    Err(err) => {
                        if let ActorErr::Crashing(_e) = err {
                            *state_g = ActorState::Crashed;
                        }
                    }
                }
            }
        } else { break; }
    }
}
