use std::collections::HashSet;

pub mod runner;

pub trait Model: Sized {
    type Op: Clone + std::fmt::Debug;
    type Value: Clone + std::fmt::Debug;

    fn initial() -> Self;
    fn apply(&self, op: &Self::Op) -> (Self, Self::Value);
}

#[derive(Debug)]
pub enum Event<M: Model> {
    Invoke { op: M::Op, ret_event: usize },
    Ret { val: M::Value },
}

#[derive(Debug)]
pub struct Node<M: Model> {
    ev: Option<Event<M>>, // None if it's a begin or end sentinel node
    next: usize,
    prev: usize,
}

#[derive(Debug)]
pub struct Hist<M: Model> {
    events: Vec<Node<M>>,
}

impl<M: Model> Hist<M> {
    const BEGIN: usize = 0;
    const END: usize = 0;

    pub fn with_capacity(cap: usize) -> Self {
        let mut events = Vec::with_capacity(cap + 2);
        events.push(Node {
            ev: None,
            next: Self::END,
            prev: Self::BEGIN,
        });
        events.push(Node {
            ev: None,
            next: Self::END,
            prev: Self::BEGIN,
        });

        Self { events }
    }

    pub fn push_back(&mut self, ev: Event<M>) -> usize {
        let news_pos = self.events.len();
        let old_last = {
            // update the rear sentinel
            let last = self.events.get_mut(Self::END).unwrap();
            let old_last = last.prev;
            last.prev = news_pos;
            old_last
        };
        {
            // update the previous last node
            let last = self.events.get_mut(old_last).unwrap();
            last.next = news_pos;
        }
        // add the new node
        self.events.push(Node {
            ev: Some(ev),
            next: Self::END,
            prev: old_last,
        });
        news_pos
    }

    pub fn get_mut_from_eid(&mut self, eid: usize) -> &mut Event<M> {
        let node = self.events.get_mut(eid).unwrap();
        node.ev.as_mut().unwrap()
    }

    pub fn iter(&self) -> Iter<'_, M> {
        Iter { hist: self, eid: 0 }
    }

    fn lift(&mut self, eid: usize) {
        let (prev, next) = {
            let node = self.events.get(eid).unwrap();
            (node.prev, node.next)
        };
        self.events.get_mut(prev).unwrap().next = next;
        self.events.get_mut(next).unwrap().next = prev;
    }

    fn unlift(&mut self, eid: usize) {
        let (prev, next) = {
            let node = self.events.get(eid).unwrap();
            (node.prev, node.next)
        };
        self.events.get_mut(prev).unwrap().next = eid;
        self.events.get_mut(next).unwrap().next = eid;
    }

    pub fn len(&self) -> usize {
        self.events.len() - 2
    }
}

pub struct Iter<'a, M: Model> {
    hist: &'a Hist<M>,
    eid: usize,
}

impl<'a, M> Iterator for Iter<'a, M>
where
    M: Model,
{
    type Item = &'a Event<M>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_eid = self.hist.events.get(self.eid).unwrap().next;
        let next = self.hist.events.get(next_eid).unwrap();

        self.eid = next_eid;
        next.ev.as_ref()
    }
}

impl<'a, M: Model> IntoIterator for &'a Hist<M> {
    type Item = &'a Event<M>;

    type IntoIter = Iter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

type Linbits = bit_vec::BitVec<u32>; // TODO: use u64

pub struct Checker<M: Model> {
    hist: Hist<M>,
    lin: Linbits,
    calls: Vec<usize>,
    cache: HashSet<(Linbits, M)>,
}

impl<M> Checker<M>
where
    M: Model,
{
    pub fn new(hist: Hist<M>) -> Self {
        let len = hist.len();
        Self {
            hist,
            lin: Linbits::from_elem(len / 2, false),
            calls: Vec::new(),
            cache: HashSet::new(),
        }
    }

    pub fn check_linearizability(&mut self) -> bool {
        true
    }
}
