use std::collections::{HashMap, HashSet};
use std::ops::{Add, Sub};

use itertools::Itertools;

use crate::model::entity::{Id, Tag, Member};
use crate::model::group::{Group, Table};
use crate::model::condition::{RelationPenalty, Constraint, Condition, Score, self};
use crate::action::{Index, Action, ActionResult, ActionError, Position};


impl Group {
    fn calc_score(&self, penalty: &RelationPenalty) -> Score {
        self.members.iter().combinations(2).map(|pair| {
            let ids = [pair[0].id, pair[1].id];
            penalty.get_pair(ids)
        }).sum()
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
    pub members: Vec<Member>,
    pub tagcounts: TagCounter,
    pub penalty_score: Score,
}

impl GroupCache {
    fn create(group: &Group, penalty: &RelationPenalty) -> GroupCache {
        let tagcounts = group.members
            .iter()
            .flat_map(|member| member.tags.iter().cloned()).collect::<Vec<Tag>>().into();
        let penalty_score = group.calc_score(penalty);
        let members = group.members.clone();
        GroupCache { members, tagcounts, penalty_score }
    }

    fn get_ids(&self) -> HashSet<Id> {
        self.members.iter().map(|member| member.id).collect()
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
            let tagcounts = self.tagcounts.clone() - member.tags.iter().cloned().collect::<Vec<Tag>>().into();
            let score = self.get_ids().iter()
                .filter(|id| **id != member.id)
                .map(|id| condition.penalty.get_pair([member.id, *id]))
                .sum::<Score>();
            if condition.constraint.check(&tagcounts, self.members.len() - 1) {
                ActionResult::ScoreDiff(-score)
            } else {
                ActionResult::UnsatisfiedScoreDiff(-score)
            }
        } else {
            ActionResult::Failed(vec![ActionError::InvalidPosition])
        }
    }

    fn simulate_swap(&self, index: Index, member: &Member, condition: &Condition) -> ActionResult {
        if let Option::Some(removed_member) = &self.members.get(index) {
            let score = self.get_ids().iter()
                .map(|id| condition.penalty.get_pair([member.id, *id]) - condition.penalty.get_pair([removed_member.id, *id]))
                .sum::<Score>();
            let tagcounts = self.tagcounts.clone()
                + member.tags.iter().cloned().collect::<Vec<Tag>>().into()
                - removed_member.tags.iter().cloned().collect::<Vec<Tag>>().into();
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
        self.tagcounts = self.tagcounts.clone() + member.tags.iter().cloned().collect::<Vec<Tag>>().into();
        self.penalty_score += self.get_ids().iter()
            .map(|id| condition.penalty.get_pair([member.id, *id]))
            .sum::<Score>();
        self.members.push(member);
        Ok(())
    }

    fn remove(&mut self, index: Index, condition: &Condition) -> Result<Member, ActionError> {
        if self.members.len() <= index {
            return Err(ActionError::InvalidPosition);
        }
        let member = self.members.remove(index);
        self.tagcounts = self.tagcounts.clone() - member.tags.iter().cloned().collect::<Vec<Tag>>().into();
        self.penalty_score -= self.get_ids().iter()
            .map(|id| condition.penalty.get_pair([member.id, *id]))
            .sum::<Score>();
        Ok(member)
    }

    fn swap(&mut self, index: Index, member: Member, condition: &Condition) -> Result<Member, ActionError> {
        if self.members.len() <= index {
            return Err(ActionError::InvalidPosition);
        }
        let removed_member = self.members.remove(index);
        self.tagcounts = self.tagcounts.clone()
            + member.tags.iter().cloned().collect::<Vec<Tag>>().into()
            - removed_member.tags.iter().cloned().collect::<Vec<Tag>>().into();
        self.penalty_score += self.get_ids().iter()
            .map(|id| condition.penalty.get_pair([member.id, *id]) - condition.penalty.get_pair([removed_member.id, *id]))
            .sum::<Score>();
        self.members.insert(index, member);
        Ok(removed_member)
    }

}

struct TableCache {
    pub groups: Vec<GroupCache>,
    pub penalty_score: Score,
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

    fn get_member(&self, position: &Position) -> Option<&Member> {
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
                    self.get_group(position1).unwrap().simulate_swap(position1.member_index, &member2, condition)
                        + self.get_group(position2).unwrap().simulate_swap(position2.member_index, &member1, condition)
                } else {
                    ActionResult::Failed(vec![ActionError::InvalidPosition])
                }
            }
            Action::Move { from, to } => {
                if let (Some(member), Some(group)) = (self.get_member(from), self.get_group(from)) {
                    group.simulate_remove(from.member_index, condition)
                        + self.groups.get(to.group_index).unwrap().simulate_add(&member, condition)
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
                let member = group.remove(position.member_index, condition)?;
                self.penalty_score -= group.penalty_score - prev_score;
                Ok(Some(member))
            }
            Action::Swap(position1, position2) => {
                let member2_clone = self.get_member(&position2).ok_or(ActionError::InvalidPosition)?.clone();
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
                let member = group_from.remove(from.member_index, condition)?;
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