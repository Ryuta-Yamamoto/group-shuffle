use std::collections::{HashMap, HashSet};

pub type Id = u32;
pub type Tag = String;

pub struct Member {
    pub id: Id,
    pub tags: HashSet<Tag>,
}

pub struct Group {
    pub members: Vec<Member>,
}

pub struct Table {
    pub groups: Vec<Group>,
}

pub struct Penalty (pub HashMap<HashSet<Id>, i64>);

pub struct Constraint (pub HashMap<Tag, std::ops::Range<usize>>);

pub struct Condition {
    pub penalty: Penalty,
    pub constraint: Constraint,
}

pub struct Position {
    pub group_index: usize,
    pub member_index: usize,
}

pub struct Swap {
    from: Position,
    to: Position,
}
