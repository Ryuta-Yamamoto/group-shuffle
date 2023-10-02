use std::{collections::{HashMap, HashSet}, hash::Hash, mem};

use itertools::Itertools;

use crate::model::entity::{Id, Tag, Member};
use crate::model::group::{Group, Table};
use crate::model::condition::{RelationPenalty, Constraint, Condition, Score};





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



<<<<<<< Updated upstream
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
        Move { from: Position, to: Position },
        Add{ member: Member, group_index: Index },
        Remove(Position),
    }

    #[derive(Debug, Clone, Error)]
    pub enum ActionError {
        #[error("Invalid position")]
        InvalidPosition,
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
    use std::path::PrefixComponent;
    use std::{vec, mem};

    use itertools::Itertools;

    use crate::model::entity::{Id, Tag, Member};
    use crate::model::group::{Group, Table};
    use crate::model::condition::{RelationPenalty, Constraint, Condition, Score, self};
    use super::action::{Index, Action, ActionResult, ActionError, Position};

    struct CachedMember {
        pub member: Member,
        pub penalty: HashMap<Id, Score>,
        pub score: Score,
    }

    impl CachedMember {
        fn create(group: &Group, index: usize, penalty: &RelationPenalty) -> CachedMember {
            let member = group.members[index].clone();
            let penalty: HashMap<Id, Score> = group.members.iter()
                .filter(|m| m.id != member.id)
                .map(|m| (m.id, penalty.get_pair([member.id, m.id])))
                .collect();
            let score = penalty.values().sum();
            CachedMember { member, penalty, score }
        }
        fn from_ids(member: Member, ids: &[Id], penalty: &RelationPenalty) -> CachedMember {
            let penalty: HashMap<Id, Score> = ids.iter()
                .map(|id| (*id, penalty.get_pair([member.id, *id])))
                .collect();
            let score = penalty.values().sum();
            CachedMember { member, penalty, score }
        }
        fn calc_score(&self, ids: &[Id], penalty: &RelationPenalty) -> Score {
            ids.iter().map(|id| penalty.get_pair([self.member.id, *id])).sum()
        }
        fn snoop_add(&mut self, id: Id, penalty: &RelationPenalty) -> Score {
            let score = penalty.get_pair([self.member.id, id]);
            self.penalty.insert(id, score);
            self.score += score;
            score
        }
        fn snoop_remove(&mut self, id: Id) -> Score {
            let score = self.penalty.remove(&id).unwrap_or(0 as Score);
            self.score -= score;
            score
        }
    }

    #[derive(Debug, Clone)]
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

    impl Constraint {
        fn check(&self, tagcounts: &TagCounter, n_members: usize) -> bool {
            unimplemented!()
        }
    }

    struct GroupCache {
        pub members: Vec<CachedMember>,
        pub tagcounts: TagCounter,
        pub penalty_score: f64,
    }

    impl GroupCache {
        fn create(group: &Group, penalty: &RelationPenalty) -> GroupCache {
            let tagcounts = group.members
                .iter()
                .flat_map(|member| member.tags.iter().cloned()).collect::<Vec<Tag>>().into();
            let penalty_score = group.calc_score(penalty);
            let members = group.members.iter().enumerate().map(|(idx, _)| {
                CachedMember::create(group, idx, penalty)
            }).collect();
            GroupCache { members, tagcounts, penalty_score }
        }

        fn get_ids(&self) -> HashSet<Id> {
            self.members.iter().map(|member| member.member.id).collect()
        }

        fn simulate_add(&self, member: &Member, condition: &Condition) -> ActionResult {
            let score = self.get_ids().iter()
                .map(|id| condition.penalty.get_pair([member.id, *id]))
                .sum();
            let tagcounts = self.tagcounts.clone() + member.tags.iter().cloned().collect::<Vec<Tag>>().into();
            if condition.constraint.check(&tagcounts, self.members.len() + 1) {
                ActionResult::ScoreDiff(score)
            } else {
                ActionResult::UnsatisfiedScoreDiff(score)
            }
        }

        fn simulate_remove(&self, index: Index, condition: &Condition) -> ActionResult {
            if let Option::Some(member) = &self.members.get(index) {
                let tagcounts = self.tagcounts.clone() - member.member.tags.iter().cloned().collect::<Vec<Tag>>().into();
                if condition.constraint.check(&tagcounts, self.members.len() - 1) {
                    ActionResult::ScoreDiff(-member.score)
                } else {
                    ActionResult::UnsatisfiedScoreDiff(-member.score)
                }
            } else {
                ActionResult::Failed(vec![ActionError::InvalidPosition])
            }
        }

        fn simulate_swap(&self, index: Index, member: &Member, condition: &Condition) -> ActionResult {
            if let Option::Some(removed_member) = &self.members.get(index) {
                let score = self.get_ids().iter()
                    .map(|id| condition.penalty.get_pair([member.id, *id]))
                    .sum::<Score>()
                    - removed_member.score;
                let tagcounts = self.tagcounts.clone()
                    + member.tags.iter().cloned().collect::<Vec<Tag>>().into()
                    - removed_member.member.tags.iter().cloned().collect::<Vec<Tag>>().into();
                if condition.constraint.check(&tagcounts, self.members.len()) {
                    ActionResult::ScoreDiff(score)
                } else {
                    ActionResult::UnsatisfiedScoreDiff(score)
                }
            } else {
                ActionResult::Failed(vec![ActionError::InvalidPosition])
            }
        }

        fn add(&mut self, member: Member, condition: &Condition) -> Result<(), ActionError> {
            self.members.iter_mut().for_each(|m| { m.snoop_add(m.member.id, &condition.penalty); });
            self.tagcounts = self.tagcounts.clone() + member.tags.iter().cloned().collect::<Vec<Tag>>().into();
            let cache = CachedMember::from_ids(member, &self.get_ids().into_iter().collect_vec(), &condition.penalty);
            self.members.push(cache);
            Ok(())
        }

        fn remove(&mut self, index: Index) -> Result<Member, ActionError> {
            if self.members.len() <= index {
                return Err(ActionError::InvalidPosition);
            }
            let member = self.members.remove(index);
            self.members.iter_mut().for_each(|m| { m.snoop_remove(member.member.id); });
            self.tagcounts = self.tagcounts.clone() - member.member.tags.iter().cloned().collect::<Vec<Tag>>().into();
            self.members.remove(index);
            Ok(member.member)
        }

        fn swap(&mut self, index: Index, member: Member, condition: &Condition) -> Result<Member, ActionError> {
            if self.members.len() <= index {
                return Err(ActionError::InvalidPosition);
            }
            let removed_member = self.members.remove(index);
            self.members.iter_mut()
                .for_each(|m| {
                    m.snoop_remove(removed_member.member.id);
                    m.snoop_add(member.id, &condition.penalty);
                });
            self.tagcounts = self.tagcounts.clone()
                + member.tags.iter().cloned().collect::<Vec<Tag>>().into()
                - removed_member.member.tags.iter().cloned().collect::<Vec<Tag>>().into();
            let cache = CachedMember::from_ids(member, &self.get_ids().into_iter().collect_vec(), &condition.penalty);
            self.members.insert(index, cache);
            Ok(removed_member.member)
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

        fn get_member(&self, position: &Position) -> Option<&CachedMember> {
            self.groups.get(position.group_index)?.members.get(position.member_index)
        }

        fn get_group(&self, position: &Position) -> Option<&GroupCache> {
            self.groups.get(position.group_index)
        }

        fn get_mut_group(&mut self, position: &Position) -> Option<&mut GroupCache> {
            self.groups.get_mut(position.group_index)
        }

        fn simulate(&self, action: &Action, condition: &Condition) -> ActionResult {
            match action {
                Action::Add { group_index, member } => {
                    if let Option::Some(group) = self.groups.get(*group_index) {
                        group.simulate_add(member, condition)
                    } else {
                        ActionResult::Failed(vec![ActionError::InvalidPosition])
                    }
                }
                Action::Remove(position) => {
                    if let Option::Some(group) = self.groups.get(position.group_index) {
                        group.simulate_remove(position.member_index, condition)
                    } else {
                        ActionResult::Failed(vec![ActionError::InvalidPosition])
                    }
                }
                Action::Swap(position1, position2) => {
                    if let (Some(member1), Some(member2)) = (self.get_member(position1), self.get_member(position2)) {
                        self.get_group(position1).unwrap().simulate_swap(position1.member_index, &member2.member, condition)
                            + self.get_group(position2).unwrap().simulate_swap(position2.member_index, &member1.member, condition)
                    } else {
                        ActionResult::Failed(vec![ActionError::InvalidPosition])
                    }
                }
                Action::Move { from, to } => {
                    if let (Some(member), Some(group)) = (self.get_member(from), self.get_group(from)) {
                        group.simulate_remove(from.member_index, condition)
                            + self.groups.get(to.group_index).unwrap().simulate_add(&member.member, condition)
                    } else {
                        ActionResult::Failed(vec![ActionError::InvalidPosition])
                    }
                }
            }
        }

        fn act(&mut self, action: Action, condition: &Condition) -> Result<Option<Member>, ActionError> {
            match action {
                Action::Add { group_index, member } => {
                    let group = self.groups.get_mut(group_index).ok_or(ActionError::InvalidPosition)?;
                    let prev_score = group.penalty_score;
                    group.add(member, condition)?;
                    self.penalty_score += group.penalty_score - prev_score;
                    Ok(None)
                }
                Action::Remove(position) => {
                    let group = self.groups.get_mut(position.group_index).ok_or(ActionError::InvalidPosition)?;
                    let prev_score = group.penalty_score;
                    let member = group.remove(position.member_index)?;
                    self.penalty_score -= group.penalty_score - prev_score;
                    Ok(Some(member))
                }
                Action::Swap(position1, position2) => {
                    let member2_clone = self.get_member(&position2).ok_or(ActionError::InvalidPosition)?.member.clone();
                    let group1 = self.groups.get_mut(position1.group_index).ok_or(ActionError::InvalidPosition)?;
                    let mut score_diff = - group1.penalty_score;
                    let member1 = group1.swap(position1.member_index, member2_clone.clone(), condition)?;
                    score_diff += group1.penalty_score;
                    let group2 = self.groups.get_mut(position2.group_index).ok_or(ActionError::InvalidPosition)?;
                    score_diff -= group2.penalty_score;
                    group2.swap(position2.member_index, member1, condition)?;
                    score_diff += group2.penalty_score;
                    self.penalty_score += score_diff;
                    Ok(None)
                }
                Action::Move { from, to } => {
                    let group_from = self.groups.get_mut(from.group_index).ok_or(ActionError::InvalidPosition)?;
                    let mut score_diff = - group_from.penalty_score;
                    let member = group_from.remove(from.member_index)?;
                    score_diff += group_from.penalty_score;
                    let group_to = self.groups.get_mut(to.group_index).ok_or(ActionError::InvalidPosition)?;
                    score_diff -= group_to.penalty_score;
                    group_to.add(member, &condition)?;
                    score_diff += group_to.penalty_score;
                    self.penalty_score += score_diff;
                    Ok(None)
                }
            }
        }
    }
}
=======
>>>>>>> Stashed changes
