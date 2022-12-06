use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use crossbeam_queue::ArrayQueue;
use rand::{distributions::Distribution, SeedableRng};

use crate::Hist;
use crate::Model;

pub trait System<M: Model>
where
    Self: Sized,
{
    type OpDist: Distribution<M::Op>;

    fn new_op_distr() -> Self::OpDist;

    fn initial() -> Self;
    fn apply(&self, op: M::Op) -> M::Value;
}

#[derive(Debug)]
enum Event<M: Model> {
    Invoke { op: M::Op, tid: u32 },
    Ret { val: M::Value, tid: u32 },
}

pub struct Runner<M: Model, S: System<M>> {
    events: ArrayQueue<Event<M>>,
    system: S,
    thread_count: u32,
    events_per_thread: u32,
}

impl<M, S> Runner<M, S>
where
    M: Model,
    S: System<M> + Send + Sync,
    M::Value: Send,
    M::Op: Send,
    M::Op: std::fmt::Debug,
    M::Value: std::fmt::Debug,
    M: std::fmt::Debug,
{
    pub fn new(thread_count: u32, events_per_thread: u32) -> Self {
        Self {
            events: ArrayQueue::new(thread_count as usize * events_per_thread as usize * 2),
            thread_count,
            events_per_thread,
            system: S::initial(),
        }
    }

    fn run(&self) {
        let start = AtomicBool::new(false);
        let start_ref = &start;

        std::thread::scope(|s| {
            for tid in 0..self.thread_count {
                s.spawn(move || {
                    let dist = S::new_op_distr();
                    let mut rng = rand::rngs::SmallRng::from_entropy();

                    // Spin so all threads start at the same time
                    while !start_ref.load(Ordering::Relaxed) {}

                    for _ in 0..self.events_per_thread {
                        let op = dist.sample(&mut rng);
                        self.events
                            .push(Event::Invoke {
                                op: op.clone(),
                                tid,
                            })
                            .unwrap();
                        let res = self.system.apply(op);
                        self.events.push(Event::Ret { val: res, tid }).unwrap();
                    }
                });
            }
            thread::sleep(Duration::from_millis(100));
            start.store(true, Ordering::Release);
        });
        // probably unnecessary
        thread::sleep(Duration::from_millis(100));
    }

    pub fn produce_history(self) -> Hist<M> {
        self.run();

        let mut hist = Hist::with_capacity(self.events.capacity());

        const INVALID: usize = usize::MAX;

        let mut pending: Vec<usize> = std::iter::repeat(INVALID)
            .take(self.thread_count as usize)
            .collect();

        for event in self.events {
            match event {
                Event::Invoke { op, tid } => {
                    let pos = hist.push_back(crate::Event::Invoke {
                        op,
                        ret_event: INVALID,
                    });
                    debug_assert_eq!(pending[tid as usize], INVALID);
                    pending[tid as usize] = pos;
                }
                Event::Ret { val, tid } => {
                    let pos = hist.push_back(crate::Event::Ret { val });
                    let inv = pending[tid as usize];
                    debug_assert_ne!(inv, INVALID);
                    if let crate::Event::Invoke { ret_event, .. } = hist.get_mut_from_eid(inv) {
                        *ret_event = pos;
                    } else {
                        unreachable!();
                    }
                    pending[tid as usize] = INVALID;
                }
            }
        }
        hist
    }
}
