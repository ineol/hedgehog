use std::{collections::HashSet, hash::Hash};

pub mod bitvec;
pub mod runner;

pub trait Model: Sized {
    type Op: Clone + std::fmt::Debug;
    type Value: Clone + Eq + std::fmt::Debug;

    fn initial() -> Self;
    fn apply(&self, op: &Self::Op) -> (Self, Self::Value);
}

#[derive(Debug)]
pub enum Event<M: Model> {
    Invoke {
        op: M::Op,
        ret_event: usize,
        call_id: usize,
    },
    Ret {
        val: M::Value,
    },
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

    pub fn get_from_eid(&self, eid: usize) -> &Event<M> {
        let node = self.events.get(eid).expect("invalid eid");
        node.ev.as_ref().expect("cannot get sentinel node")
    }

    pub fn get_mut_from_eid(&mut self, eid: usize) -> &mut Event<M> {
        let node = self.events.get_mut(eid).expect("invalid eid");
        node.ev.as_mut().expect("cannot get sentinel node")
    }

    pub fn next_eid(&self, eid: usize) -> Option<usize> {
        let next = self.events[eid].next;
        if next != Self::END {
            Some(next)
        } else {
            None
        }
    }

    pub fn iter(&self) -> Iter<'_, M> {
        Iter { hist: self, eid: 0 }
    }

    fn lift_event(&mut self, eid: usize) {
        let (prev, next) = {
            let node = self.events.get(eid).unwrap();
            (node.prev, node.next)
        };
        self.events[prev].next = next;
        self.events[next].prev = prev;
    }

    fn lift(&mut self, eid: usize) {
        let eid_ret = if let Event::Invoke { ret_event, .. } = self.get_from_eid(eid) {
            *ret_event
        } else {
            unreachable!("lift called with the eid of a return event")
        };
        self.lift_event(eid);
        self.lift_event(eid_ret);
    }

    fn unlift_event(&mut self, eid: usize) {
        let (prev, next) = {
            let node = self.events.get(eid).unwrap();
            (node.prev, node.next)
        };
        self.events[prev].next = eid;
        self.events[next].prev = eid;
    }

    fn unlift(&mut self, eid: usize) {
        let eid_ret = if let Event::Invoke { ret_event, .. } = self.get_from_eid(eid) {
            *ret_event
        } else {
            unreachable!("unlift called with the eid of a return event")
        };
        self.unlift_event(eid_ret);
        self.unlift_event(eid);
    }

    pub fn len(&self) -> usize {
        self.events.len() - 2
    }

    pub fn first_eid(&self) -> Option<usize> {
        self.next_eid(Self::BEGIN)
    }

    pub fn first_event(&self) -> Option<&Event<M>> {
        let oeid = self.first_eid();
        oeid.map(|eid| self.get_from_eid(eid))
    }

    pub fn empty(&self) -> bool {
        self.first_eid().is_none()
    }

    fn get_call_id(&self, eid: usize) -> usize {
        let event = self.get_from_eid(eid);
        if let Event::Invoke {
            call_id: inv_id, ..
        } = event
        {
            *inv_id
        } else {
            unreachable!()
        }
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

type Linbits = crate::bitvec::BitVec; // TODO: use u64

pub struct Checker<M: Model> {
    hist: Hist<M>,
    lined: Linbits,
    calls: Vec<(usize, M)>,
    cache: HashSet<(Linbits, M)>,
}

impl<M> Checker<M>
where
    M: Model + Clone + Eq + Hash + std::fmt::Debug,
{
    pub fn new(hist: Hist<M>) -> Self {
        let len = hist.len();
        Self {
            hist,
            lined: Linbits::from_elem(false, len / 2),
            calls: Vec::new(),
            cache: HashSet::new(),
        }
    }

    fn apply(&self, s: &M, eid: usize) -> (bool, usize, M) {
        let event = self.hist.get_from_eid(eid);
        if let Event::Invoke {
            op,
            ret_event,
            call_id: inv_id,
        } = event
        {
            let (s2, res) = s.apply(op);
            if let Event::Ret { val } = self.hist.get_from_eid(*ret_event) {
                (val == &res, *inv_id, s2)
            } else {
                unreachable!("Invoke event")
            }
        } else {
            unreachable!("Return event")
        }
    }

    pub fn check_linearizability(&mut self) -> bool {
        let mut eid = if let Some(eid) = self.hist.first_eid() {
            eid
        } else {
            return true;
        };

        let mut s = M::initial();

        let mut iteration_count: u64 = 0;
        loop {
            iteration_count += 1;
            if matches!(self.hist.get_from_eid(eid), Event::Invoke { .. }) {
                let next_eid = self.hist.next_eid(eid).unwrap();

                let (lin, call_id, s2) = self.apply(&s, eid);

                if lin {
                    let mut lined2 = self.lined.clone();
                    lined2.set(call_id, true);
                    let unseen = self.cache.insert((lined2, s2.clone()));

                    if unseen {
                        self.calls.push((eid, s));
                        s = s2;
                        self.lined.set(call_id, true);
                        self.hist.lift(eid);
                        if let Some(next_eid) = self.hist.first_eid() {
                            eid = next_eid;
                        } else {
                            break;
                        }
                    } else {
                        eid = next_eid;
                    }
                } else {
                    eid = next_eid;
                };
            } else {
                match self.calls.pop() {
                    None => return false,
                    Some((eid2, s2)) => {
                        let call_id = self.hist.get_call_id(eid2);

                        self.lined.set(call_id, false);
                        self.hist.unlift(eid2);

                        let next_eid = self.hist.next_eid(eid2).unwrap();
                        eid = next_eid;
                        s = s2;
                    }
                }
            }
        }
        true
    }
}
