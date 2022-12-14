use std::{
    io::Write,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Instant,
};

use hedgehog::runner;
use rand::prelude::Distribution;

#[derive(Debug, Clone)]
enum CounterOp {
    Incr,
    Read,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct CounterModel {
    inner: u32,
}

impl hedgehog::Model for CounterModel {
    type Op = CounterOp;

    type Value = Option<u32>;

    fn initial() -> Self {
        Self { inner: 0 }
    }

    fn apply(&self, op: &Self::Op) -> (Self, Self::Value) {
        match op {
            CounterOp::Incr => (
                Self {
                    inner: self.inner + 1,
                },
                None,
            ),
            CounterOp::Read => (self.clone(), Some(self.inner)),
        }
    }
}

#[derive(Clone)]
struct CounterSystem {
    inner: Arc<AtomicU32>,
}

struct CounterOpDist;

impl Distribution<CounterOp> for CounterOpDist {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> CounterOp {
        let kind: u8 = rng.gen_range(0..2);

        match kind {
            0 => CounterOp::Incr,
            1 => CounterOp::Read,
            _ => unreachable!(),
        }
    }
}

impl hedgehog::runner::System<CounterModel> for CounterSystem {
    type OpDist = CounterOpDist;

    fn new_op_distr() -> Self::OpDist {
        CounterOpDist
    }

    fn initial() -> Self {
        Self {
            inner: AtomicU32::new(0).into(),
        }
    }

    fn apply(&mut self, op: CounterOp) -> Option<u32> {
        match op {
            CounterOp::Incr => {
                let old = self.inner.load(Ordering::Relaxed);
                self.inner.store(old + 1, Ordering::Relaxed);
                None
            }
            CounterOp::Read => Some(self.inner.load(Ordering::Relaxed)),
        }
    }
}

fn main() {
    let count = 100;
    let thread_count = 5;
    let event_count = 10_000;
    for _ in 0..count {
        print!(".");
        std::io::stdout().flush().unwrap();

        let runner: runner::Runner<CounterModel, CounterSystem> =
            hedgehog::runner::Runner::new(thread_count, event_count);

        // let begin = Instant::now();
        let hist = runner.produce_history();

        // println!("History produced in {:?}", begin.elapsed());

        // for ev in &hist {
        //     println!("{:?}", ev);
        // }

        // println!("The history {:#?}", &hist);

        // let checking = Instant::now();

        let mut checker = hedgehog::Checker::new(hist);

        let res = checker.check_linearizability();

        if !res {
            println!("\nFound a non-linearizable history!");
        }

        // println!("Linearizability checked in {:?}", checking.elapsed());

        // println!("\n\nWas this history linearizable? {}", res);
    }
}
