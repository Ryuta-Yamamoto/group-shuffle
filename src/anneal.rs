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
    use crate::model::entity::Member;

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

    pub enum ActionResult {
        ScoreDiff(f64),
        UnsatisfiedScoreDiff(f64),   // score, unsatisfied constraint
        Failed,
    }
}


pub mod cache {
    use std::collections::{HashMap, HashSet};

    use itertools::Itertools;

    use crate::model::entity::{Id, Tag, Member};
    use crate::model::group::{Group, Table};
    use crate::model::condition::{RelationPenalty, Constraint, Condition, Score};
    use super::action::{Action, ActionResult};

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
            let sub = removed_ids.iter().map(|id| self.penalty.get(id))
                .try_fold(0., |acc, score| score.map(|score| acc + score));
            if let Some(sub) = sub {
                let add = added_ids.iter().map(|id| penalty.get_pair([my_id, *id])).sum::<Score>();
                return ActionResult::ScoreDiff(add - sub)
            }
            ActionResult::Failed
        }
    }

    struct GroupCache {
        pub members: Vec<CachedMember>,
        pub tagcounts: HashMap<Tag, usize>,
        pub penalty_score: f64,
    }

    impl GroupCache {
        pub fn create(group: &Group, penalty: &RelationPenalty) -> GroupCache {
            let tagcounts = group.members
                .iter()
                .flat_map(|member| member.tags.iter().cloned())
                .counts();
            let penalty_score = group.calc_score(penalty);
            let members = group.members.iter().enumerate().map(|(idx, _)| {
                CachedMember::create(group, idx, penalty)
            }).collect();
            GroupCache { members, tagcounts, penalty_score }
        }
        pub fn simulate(&self, action: &Action, condition: &Condition) -> ActionResult {
            unimplemented!()
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
