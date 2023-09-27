use std::{collections::{HashMap, HashSet}, hash::Hash, mem};

use itertools::Itertools;

use crate::model::entity::{Id, Tag, Member};
use crate::model::group::{Group, Table};
use crate::model::condition::{RelationPenalty, Constraint, Condition, Score};


impl Group {
    fn calc_score(&self, penalty: &RelationPenalty) -> Score {
        self.members.iter().combinations(2).map(|pair| {
            let ids = [pair[0].id, pair[1].id];
            penalty.get_pair(ids)
        }).sum()
    }
}

impl RelationPenalty {
    fn get_personal_score_cache(&self, group: &Group, index: usize) -> HashMap<Id, Score> {
        let mut penalty = HashMap::new();
        let target_id = group.members[index].id;
        for idx in 0..group.members.len() {
            if idx == index { continue; }
            let member = &group.members[idx];
            penalty.insert(member.id, self.get_pair([member.id, target_id]));
        }
        penalty
    }
}

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


pub mod action {
    use std::ops::Add;
    use crate::model::{entity::Member, condition::Score};

    pub type Index = usize;
    pub struct Position {
        pub group_index: Index,
        pub member_index: Index,
    }


    pub enum GroupAction {
        Add(Member),
        Remove(Index),
        Replace(Index, Member),
    }


    pub enum Action {
        Swap(Position, Position),
        Add{ member: Member, group_index: Index },
        Remove(Position),
    }

    #[derive(Debug, Clone)]
    pub enum ActionError {
        InvalidPosition,
        InvalidGroupAction,
    }

    pub enum ActionResult {
        ScoreDiff(Score),
        UnsatisfiedScoreDiff(Score),   // score, unsatisfied constraint
        Failed(Vec<ActionError>),
    }

    impl Add for ActionResult {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            match (&self, &rhs) {
                (ActionResult::ScoreDiff(s1), ActionResult::ScoreDiff(s2))
                    => ActionResult::ScoreDiff(s1 + s2),
                (ActionResult::ScoreDiff(s1), ActionResult::UnsatisfiedScoreDiff(s2))
                    => ActionResult::UnsatisfiedScoreDiff(s1 + s2),
                (ActionResult::UnsatisfiedScoreDiff(s1), ActionResult::UnsatisfiedScoreDiff(s2))
                    => ActionResult::UnsatisfiedScoreDiff(s1 + s2),
                (ActionResult::Failed(err1), ActionResult::Failed(err2))
                    => ActionResult::Failed(err1.into_iter().chain(err2.into_iter()).cloned().collect()),
                _ => rhs + self
            }
        }
    }
}


pub mod cache {
    use std::collections::{HashMap, HashSet};
    use std::ops::{Add, Sub};

    use itertools::Itertools;

    use crate::model::entity::{Id, Tag, Member};
    use crate::model::group::{Group, Table};
    use crate::model::condition::{RelationPenalty, Constraint, Condition, Score};
    use super::action::{Index, Action, ActionResult, ActionError};

    struct CachedMember {
        pub member: Member,
        pub penalty: HashMap<Id, Score>,
        pub score: Score,
    }

    impl CachedMember {
        pub fn create(group: &Group, index: usize, penalty: &RelationPenalty) -> CachedMember {
            let member = group.members[index].clone();
            let penalty = penalty.get_personal_score_cache(group, index);
            let score = penalty.values().sum();
            CachedMember { member, penalty, score }
        }
        pub fn simulate(&self, added_ids: Vec<Id>, removed_ids: Vec<Id>, penalty: &RelationPenalty) -> ActionResult {
            let my_id = self.member.id;
            let sub = removed_ids.iter()
                .filter(|id| **id != my_id)
                .map(|id| self.penalty.get(id))
                .try_fold(0., |acc, score| score.map(|score| acc + score));
            if let Some(sub) = sub {
                let add = added_ids.iter()
                    .filter(|id| **id != my_id)
                    .map(|id| penalty.get_pair([my_id, *id])).sum::<Score>();
                return ActionResult::ScoreDiff(add - sub)
            }
            ActionResult::Failed(vec![ActionError::InvalidPosition])
        }
    }

    struct TagCounter (HashMap<Tag, usize>);

    impl From<Vec<Tag>> for TagCounter {
        fn from(tags: Vec<Tag>) -> Self {
            let mut counter = HashMap::new();
            for tag in tags {
                *counter.entry(tag).or_insert(0) += 1;
            }
            TagCounter(counter)
        }
    }

    impl Add for TagCounter {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            let mut counter = self.0;
            for (tag, count) in rhs.0 {
                *counter.entry(tag).or_insert(0) += count;
            }
            TagCounter(counter)
        }
    }

    impl Sub for TagCounter {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self::Output {
            let mut counter = self.0;
            for (tag, count) in rhs.0 {
                *counter.entry(tag).or_insert(count) -= count;
            }
            TagCounter(counter)
        }
    }

    struct GroupCache {
        pub members: Vec<CachedMember>,
        pub tagcounts: TagCounter,
        pub penalty_score: f64,
    }

    impl GroupCache {
        pub fn create(group: &Group, penalty: &RelationPenalty) -> GroupCache {
            let tagcounts = group.members
                .iter()
                .flat_map(|member| member.tags.iter().cloned()).collect::<Vec<Tag>>().into();
            let penalty_score = group.calc_score(penalty);
            let members = group.members.iter().enumerate().map(|(idx, _)| {
                CachedMember::create(group, idx, penalty)
            }).collect();
            GroupCache { members, tagcounts, penalty_score }
        }
        pub fn simulate(
            &self,
            added_members: Vec<Member>,
            removed_indices: Vec<Index>,
            condition: &Condition
        ) -> ActionResult {
            let added_ids = added_members.iter().map(|member| member.id).collect();
            // let removed_members = removed_indices.iter().map(|idx| &self.members[*idx].member).collect();
            let removed_ids = removed_members.iter().map(|member| member.id).collect();
            unimplemented!("# TODO: check constraint, calc score diff")
        }
    }

    struct TableCache {
        pub groups: Vec<GroupCache>,
        pub penalty_score: f64,
    }

    impl TableCache {
        fn create(table: &Table, penalty: &RelationPenalty) -> TableCache {
            let groups = table.groups.iter().map(|group| {
                GroupCache::create(group, penalty)
            }).collect();
            let penalty_score = table.groups.iter().map(|group| {
                group.calc_score(penalty)
            }).sum();
            TableCache { groups, penalty_score }
        }
        fn simulate(&self, action: &Action, condition: &Condition) -> ActionResult {
            unimplemented!()
        }
    }
}
