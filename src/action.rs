use std::ops::Add;
use thiserror::Error;
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
