use std::collections::{HashMap, HashSet};
use std::ops::{Add, Sub};

use itertools::Itertools;

use crate::model::entity::{Id, Tag, Member};
use crate::model::group::{Group, Table};
use crate::model::condition::{RelationPenalty, Constraint, Condition, Score, Range};
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
    fn check(&self, tagcounts: &TagCounter, n_members: usize) -> Result<(), HashSet<String>> {
        let error_tags: HashSet<String> = self.0.iter().map(|(tag, range)| {
            let count = tagcounts.0.get(tag).copied().unwrap_or(0);
            match range {
                Range::Ratio {min, max} => {
                    if (count as f64) < *min * n_members as f64 || (count as f64) > *max * n_members as f64 {
                        Option::Some(tag.clone())
                    } else {
                        Option::None
                    }
                },
                Range::Count {min, max} => {
                    if count < *min || count > *max {
                        Option::Some(tag.clone())
                    } else {
                        Option::None
                    }
                },
            }
        }).filter_map(|x| x).collect();
        if error_tags.is_empty() {
            Ok(())
        } else {
            Err(error_tags)
        }
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
        if condition.constraint.check(&tagcounts, self.members.len() + 1).is_ok() {
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
            if condition.constraint.check(&tagcounts, self.members.len() - 1).is_ok() {
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
                .filter(|id| **id != removed_member.id)
                .map(|id| condition.penalty.get_pair([member.id, *id]) - condition.penalty.get_pair([removed_member.id, *id]))
                .sum::<Score>();
            let tagcounts = self.tagcounts.clone()
                + member.tags.iter().cloned().collect::<Vec<Tag>>().into()
                - removed_member.tags.iter().cloned().collect::<Vec<Tag>>().into();
            if condition.constraint.check(&tagcounts, self.members.len()).is_ok() {
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
            Action::Move { source_position: from, target_group: to } => {
                if let (Some(member), Some(group)) = (self.get_member(from), self.get_group(from)) {
                    group.simulate_remove(from.member_index, condition)
                        + self.groups.get(*to).unwrap().simulate_add(&member, condition)
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
            Action::Move { source_position: from, target_group: to } => {
                let group_from = self.groups.get_mut(from.group_index).ok_or(ActionError::InvalidPosition)?;
                let mut score_diff = - group_from.penalty_score;
                let member = group_from.remove(from.member_index, condition)?;
                score_diff += group_from.penalty_score;
                let group_to = self.groups.get_mut(to).ok_or(ActionError::InvalidPosition)?;
                score_diff -= group_to.penalty_score;
                group_to.add(member, &condition)?;
                score_diff += group_to.penalty_score;
                self.penalty_score += score_diff;
                Ok(None)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use super::*;
    use crate::model::condition::Range;

    fn table_fixture() -> Table {
        let groups = vec![
            Group {
                members: vec![
                    Member { id: 0, tags: ["a".to_string()].into() },
                    Member { id: 1, tags: ["b".to_string()].into() },
                    Member { id: 2, tags: ["c".to_string()].into() },
                ],
            },
            Group {
                members: vec![
                    Member { id: 3, tags: ["a".to_string(), "b".to_string()].into() },
                    Member { id: 4, tags: ["a".to_string(), "c".to_string()].into() },
                    Member { id: 5, tags: ["b".to_string(), "c".to_string()].into() },
                ],
            }
        ];
        Table { groups }
    }

    fn condition_fixture() -> Condition {
        Condition {
            penalty: RelationPenalty {
                scores: [
                    ([0, 1].into_iter().collect::<BTreeSet<Id>>(), 1 as Score),
                    ([1, 2].into_iter().collect::<BTreeSet<Id>>(), 2 as Score),
                    ([2, 3].into_iter().collect::<BTreeSet<Id>>(), 3 as Score),
                    ([3, 4].into_iter().collect::<BTreeSet<Id>>(), 4 as Score),
                    ([4, 5].into_iter().collect::<BTreeSet<Id>>(), 5 as Score),
                    ([5, 6].into_iter().collect::<BTreeSet<Id>>(), 6 as Score),
                ].into_iter().collect(),
                default: 0 as Score,
            },
            constraint: Constraint (
                [
                    ("a".to_string(), Range::Count { min: 1, max: 2}),
                    ("b".to_string(), Range::Count { min: 1, max: 2}),
                    ("c".to_string(), Range::Count { min: 1, max: 2}),
                ].into()
            )
        }
    }

    fn tablecache_fixture() -> TableCache {
        TableCache::create(&table_fixture(), &condition_fixture().penalty)
    }

    #[test]
    fn test_create_table() {
        let table = TableCache::create(&table_fixture(), &condition_fixture().penalty);
        assert_eq!(table.groups.len(), 2);
        assert_eq!(table.penalty_score, 12 as Score);
        assert_eq!(table.groups[0].members.len(), 3);
        assert_eq!(table.groups[1].members.len(), 3);
        assert_eq!(table.groups[0].penalty_score, 3 as Score);
        assert_eq!(table.groups[1].penalty_score, 9 as Score);
    }

    #[test]
    fn test_simulate_add() {
        let table = tablecache_fixture();
        let condition = &condition_fixture();
        let idx_tags_result = [
            (0, Vec::new(), ActionResult::ScoreDiff(0 as Score)),
            (1, Vec::new(), ActionResult::ScoreDiff(6 as Score)),
            (0, vec!["a".to_string()], ActionResult::ScoreDiff(0 as Score)),
            (1, vec!["a".to_string()], ActionResult::UnsatisfiedScoreDiff(6 as Score)),
            (2, vec!["a".to_string()], ActionResult::Failed(vec![ActionError::InvalidPosition])),
        ];

        for (group_index, tags, result) in idx_tags_result {
            let member = Member { id: 6, tags: tags.into_iter().collect() };
            let action = Action::Add { group_index: group_index, member };
            assert_eq!(table.simulate(&action, &condition), result);
        };
    }

    #[test]
    fn test_simulate_remove() {
        let table = tablecache_fixture();
        let condition = &condition_fixture();
        let idx_tags_result = [
            (0, 0, ActionResult::UnsatisfiedScoreDiff(-1 as Score)),
            (0, 1, ActionResult::UnsatisfiedScoreDiff(-3 as Score)),
            (0, 2, ActionResult::UnsatisfiedScoreDiff(-2 as Score)),
            (1, 0, ActionResult::ScoreDiff(-4 as Score)),
            (1, 1, ActionResult::ScoreDiff(-9 as Score)),
            (1, 2, ActionResult::ScoreDiff(-5 as Score)),
            (0, 3, ActionResult::Failed(vec![ActionError::InvalidPosition])),
            (1, 3, ActionResult::Failed(vec![ActionError::InvalidPosition])),
        ];

        for (group_index, member_index, result) in idx_tags_result {
            let position = Position { group_index, member_index };
            let action = Action::Remove(position);
            assert_eq!(table.simulate(&action, &condition), result);
        };
    }

    #[test]
    fn test_simulate_swap() {
        let table = tablecache_fixture();
        let condition = &condition_fixture();
        let idx_tags_result = [
            (0, 0, 1, 0, ActionResult::ScoreDiff(-2 as Score)),
            (0, 0, 1, 1, ActionResult::ScoreDiff(-10 as Score)),
            (0, 0, 1, 2, ActionResult::UnsatisfiedScoreDiff(-6 as Score)),
            (0, 1, 1, 0, ActionResult::ScoreDiff(-4 as Score)),
            (0, 1, 1, 1, ActionResult::UnsatisfiedScoreDiff(-12 as Score)),
            (0, 1, 1, 2, ActionResult::ScoreDiff(-8 as Score)),
            (0, 2, 1, 0, ActionResult::UnsatisfiedScoreDiff(-6 as Score)),
            (0, 2, 1, 1, ActionResult::ScoreDiff(-8 as Score)),
            (0, 2, 1, 2, ActionResult::ScoreDiff(-4 as Score)),
            (1, 0, 0, 0, ActionResult::ScoreDiff(-2 as Score)),
            (1, 1, 0, 0, ActionResult::ScoreDiff(-10 as Score)),
            (1, 2, 0, 0, ActionResult::UnsatisfiedScoreDiff(-6 as Score)),
            (1, 0, 0, 1, ActionResult::ScoreDiff(-4 as Score)),
            (1, 1, 0, 1, ActionResult::UnsatisfiedScoreDiff(-12 as Score)),
            (1, 2, 0, 1, ActionResult::ScoreDiff(-8 as Score)),
            (1, 0, 0, 2, ActionResult::UnsatisfiedScoreDiff(-6 as Score)),
            (1, 1, 0, 2, ActionResult::ScoreDiff(-8 as Score)),
            (1, 2, 0, 2, ActionResult::ScoreDiff(-4 as Score)),
            (0, 3, 1, 0, ActionResult::Failed(vec![ActionError::InvalidPosition])),
            (0, 3, 1, 1, ActionResult::Failed(vec![ActionError::InvalidPosition])),
        ];

        for (
            group_index,
            member_index,
            other_group_index,
            other_member_index,
            result
        ) in idx_tags_result {
            let position = Position { group_index, member_index };
            let other_position = Position { group_index: other_group_index, member_index: other_member_index };
            let action = Action::Swap(position, other_position);
            assert_eq!(table.simulate(&action, &condition), result);
        };
    }

    #[test]
    fn test_simulate_move() {
        let table = tablecache_fixture();
        let condition = &condition_fixture();
        let idx_tags_result = [
            (0, 0, 1, ActionResult::UnsatisfiedScoreDiff(-1 as Score)),
            (0, 1, 1, ActionResult::UnsatisfiedScoreDiff(-3 as Score)),
            (0, 2, 1, ActionResult::UnsatisfiedScoreDiff(1 as Score)),
            (1, 0, 0, ActionResult::ScoreDiff(-1 as Score)),
            (1, 1, 0, ActionResult::ScoreDiff(-9 as Score)),
            (1, 2, 0, ActionResult::ScoreDiff(-5 as Score)),
            (0, 3, 1, ActionResult::Failed(vec![ActionError::InvalidPosition])),
        ];

        for (group_index, member_index, target_group, result) in idx_tags_result {
            let source_position = Position { group_index, member_index };
            let action = Action::Move{ source_position, target_group: target_group };
            assert_eq!(table.simulate(&action, &condition), result);
        };
    }
}
