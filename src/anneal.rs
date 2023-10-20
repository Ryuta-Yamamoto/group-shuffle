use std::{collections::{HashMap, HashSet}, hash::Hash, mem};

use rand::prelude::{SliceRandom};
use rand::rngs::{SmallRng};
use itertools::Itertools;

use crate::model::entity::{Id, Tag, Member};
use crate::model::group::{Group, Table};
use crate::model::condition::{RelationPenalty, Constraint, Condition, Score};
use crate::action::{Action, GroupAction, Position, ActionResult, ActionError, Index};
use crate::cache::TableCache;


struct Params {
    temperature: f64,
    cooling_rate: f64,
    max_iterations: usize,
}

struct State {
    table: Table,
    n_iterations: usize,
    temperature: f64,
}

struct SwapGenerator {
    sizes: Vec<Index>,
    candidates: Vec<Position>,
    rng: SmallRng,
}

impl SwapGenerator {
    fn init(&mut self) {
        assert!(self.sizes.len() > 1);
        assert!(self.sizes.iter().all(|size| *size > 0));
        self.candidates = self.sizes
            .iter().enumerate()
            .flat_map(
                |(group_index, size)| {
                    (0..*size).map(move |member_index| Position { group_index, member_index })
                }
            )
            .collect();
    }
    fn next(&mut self) -> Action {
        let pos1 = match self.candidates.pop() {
            Some(pos) => pos,
            None => {
                self.init();
                return self.next()
            }
        };
        loop {
            let pos2 = self.candidates.pop().unwrap_or_else(|| {
                self.init();
                self.candidates.pop().unwrap()
            });
            if pos1.group_index != pos2.group_index {
                return Action::Swap(pos1, pos2)
            }
        }
    }
}
