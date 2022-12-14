use std::time::Instant;

use hedgehog::runner;
use rand::prelude::Distribution;

#[derive(Debug, Clone)]
enum FlurryOp {
    Get(u64),
    Set(u64, u64),
    Rm(u64),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct FlurryModel {
    inner: rpds::RedBlackTreeMap<u64, u64>,
}

impl hedgehog::Model for FlurryModel {
    type Op = FlurryOp;

    type Value = Option<u64>;

    fn initial() -> Self {
        Self {
            inner: rpds::RedBlackTreeMap::new(),
        }
    }

    fn apply(&self, op: &Self::Op) -> (Self, Self::Value) {
        match op {
            FlurryOp::Get(key) => {
                let res = self.inner.get(key);
                (self.clone(), res.map(|x| *x))
            }
            FlurryOp::Set(key, val) => {
                let res = self.inner.insert(*key, *val);
                (Self { inner: res }, None)
            }
            FlurryOp::Rm(key) => {
                let res = self.inner.remove(key);
                (Self { inner: res }, None)
            }
        }
    }
}

#[derive(Clone)]
struct FlurrySystem {
    inner: flurry::HashMap<u64, u64>,
}

struct FlurryOpDist;

impl Distribution<FlurryOp> for FlurryOpDist {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> FlurryOp {
        let kind: u8 = rng.gen_range(0..3);
        let key = rng.gen_range(0..10);

        match kind {
            0 => {
                let val = rng.gen_range(0..1000000);
                FlurryOp::Set(key, val)
            }
            1 => FlurryOp::Get(key),
            2 => FlurryOp::Rm(key),
            _ => unreachable!(),
        }
    }
}

impl hedgehog::runner::System<FlurryModel> for FlurrySystem {
    type OpDist = FlurryOpDist;

    fn new_op_distr() -> Self::OpDist {
        FlurryOpDist
    }

    fn initial() -> Self {
        Self {
            inner: flurry::HashMap::new(),
        }
    }

    fn apply(&mut self, op: FlurryOp) -> Option<u64> {
        match op {
            FlurryOp::Get(key) => {
                let guard = self.inner.guard();
                self.inner.get(&key, &guard).map(|x| *x)
            }
            FlurryOp::Set(key, val) => {
                let guard = self.inner.guard();
                self.inner.insert(key, val, &guard);
                None
            }
            FlurryOp::Rm(key) => {
                let guard = self.inner.guard();
                self.inner.remove(&key, &guard);
                None
            }
        }
    }
}

fn main() {
    for _ in 0..100 {
        let runner: runner::Runner<FlurryModel, FlurrySystem> =
            hedgehog::runner::Runner::new(5, 50_000);

        let begin = Instant::now();
        let hist = runner.produce_history();

        println!("History produced in {:?}", begin.elapsed());

        // for ev in &hist {
        //     println!("{:?}", ev);
        // }

        // println!("The history {:#?}", &hist);

        let checking = Instant::now();

        let mut checker = hedgehog::Checker::new(hist);

        let res = checker.check_linearizability();

        println!("Linearizability checked in {:?}", checking.elapsed());

        println!("\n\nWas this history linearizable? {}", res);
    }
}
