use std::time::{Duration, Instant};

use hedgehog::runner;
use rand::prelude::Distribution;

use kvs::{KvStore, KvsEngine};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Clone)]
enum KvOp {
    Get(String),
    Set(String, String),
    Rm(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct KvModel {
    inner: rpds::RedBlackTreeMap<String, String>,
}

impl hedgehog::Model for KvModel {
    type Op = KvOp;

    type Value = Option<String>;

    fn initial() -> Self {
        Self {
            inner: rpds::RedBlackTreeMap::new(),
        }
    }

    fn apply(&self, op: &Self::Op) -> (Self, Self::Value) {
        match op {
            KvOp::Get(key) => {
                let res = self.inner.get(key);
                (self.clone(), res.cloned())
            }
            KvOp::Set(key, val) => {
                let res = self.inner.insert(key.clone(), val.clone());
                (Self { inner: res }, None)
            }
            KvOp::Rm(key) => {
                let res = self.inner.remove(key);
                (Self { inner: res }, None)
            }
        }
    }
}

struct KvSystem {
    inner: KvStore,
}

impl Clone for KvSystem {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.try_clone().unwrap(),
        }
    }
}

struct KvOpDist;

impl Distribution<KvOp> for KvOpDist {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> KvOp {
        let kind: u8 = rng.gen_range(0..3);
        let key = rng.gen_range(0..10);

        match kind {
            0 => {
                let val = rng.gen_range(0..1000000);
                KvOp::Set(key.to_string(), val.to_string())
            }
            1 => KvOp::Get(key.to_string()),
            2 => KvOp::Rm(key.to_string()),
            _ => unreachable!(),
        }
    }
}

impl hedgehog::runner::System<KvModel> for KvSystem {
    type OpDist = KvOpDist;

    fn new_op_distr() -> Self::OpDist {
        KvOpDist
    }

    fn initial() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.into_path();
        info!("path is {:?}", &path);
        Self {
            inner: KvStore::open(path).unwrap(),
        }
    }

    fn apply(&mut self, op: KvOp) -> Option<String> {
        match op {
            KvOp::Get(key) => self.inner.get(key).unwrap(),
            KvOp::Set(key, val) => {
                self.inner.set(key, val).unwrap();
                None
            }
            KvOp::Rm(key) => {
                let _ = self.inner.remove(key);
                None
            }
        }
    }
}

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let cpus = 1;
    println!("Using {} threads", cpus);

    std::thread::spawn(|| loop {
        if let Some(stats) = memory_stats::memory_stats() {
            if stats.physical_mem > 10_000_000_000 {
                eprintln!("Hedgehow exeeded the memory budget");
                std::process::exit(1);
            }
            std::thread::sleep(Duration::from_secs(1));
        } else {
            eprintln!("Could not read the memory stats, you're on your own");
            break;
        }
    });

    for _ in 0..100 / cpus {
        let mut hists = Vec::new();

        for _ in 0..cpus {
            let runner: runner::Runner<KvModel, KvSystem> =
                hedgehog::runner::Runner::new(4, 30_000);
            let begin = Instant::now();
            let hist = runner.produce_history();

            hists.push((hist, begin.elapsed()));
        }

        std::thread::scope(move |s| {
            for (hist, prod_dur) in hists {
                s.spawn(move || {
                    let checking = Instant::now();

                    let mut checker = hedgehog::Checker::new(hist);

                    let res = checker.check_linearizability();

                    println!(
                        "Trace produced in {:?} and checked in {:?}: {}",
                        prod_dur,
                        checking.elapsed(),
                        if res { "OK" } else { "NON-LINEARIZABLE" }
                    );
                });
            }
        });
    }
}
