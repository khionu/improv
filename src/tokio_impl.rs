use std::{
    any::TypeId,
    collections::HashMap,
    sync::{
        Arc, atomic::{AtomicBool, AtomicU16, Ordering},
        RwLock,
    },
    time::Instant,
};

use futures::channel::mpsc::unbounded;
use futures::StreamExt;

use crate::{Actor, ActorErr, ActorOk, ActorRef, ActorResult, ActorState, ActorSystemDriver};

/// ActorSystemDriver implementation that uses the user's
/// Tokio runtime to spawn Actors and run them asynchronously
pub struct TokioActorDriver {
    sf_epoch: Instant,
    sf_increment: AtomicU16,
    is_running: Arc<AtomicBool>,
}

impl TokioActorDriver {
    // TODO: Move to generic utility
    fn generate_snowflake(&self) -> u64 {
        let inc = self.sf_increment.fetch_add(1, Ordering::Acquire);
        let dur = Instant::now().duration_since(self.sf_epoch).as_millis() as u64;

        ((dur << 16) | inc as u64)
    }
}

impl Default for TokioActorDriver {
    fn default() -> Self {
        Self {
            sf_epoch: Instant::now(),
            sf_increment: AtomicU16::new(0),
            is_running: Arc::new(Default::default()),
        }
    }
}

impl ActorSystemDriver for TokioActorDriver {
    fn register<T>(&self, mut actor: T) -> (ActorRef<T>, Option<T::Err>) where
        T: Actor + 'static
    {
        let id = self.generate_snowflake();

        let (tx, mut rx) = unbounded::<T::Msg>();

        let (state, err) = match actor.start() {
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

        if *state.read().unwrap() == ActorState::Healthy {
            let running = self.is_running.clone();
            tokio::spawn(async move {
                loop {
                    if !running.load(Ordering::Relaxed) { break; }

                    if let Some(msg) = rx.next().await {
                        let mut state = state.write()
                            .expect("poisoned actor_state, report to dev");

                        if *state != ActorState::Healthy {
                            break;
                        }

                        match actor.handle(msg) {
                            Ok(ok) => {
                                if ok == ActorOk::GracefulEnd {
                                    *state = ActorState::Stopped;
                                }
                            }
                            Err(err) => {
                                if let ActorErr::Crashing(e) = err {
                                    *state = ActorState::Crashed;
                                }
                            }
                        }
                    } else { break; }
                }
            });
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
