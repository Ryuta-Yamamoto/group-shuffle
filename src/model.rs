pub mod entity {
    use std::collections::HashSet;

    pub type Id = u32;
    pub type Tag = String;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Member {
        pub id: Id,
        pub tags: HashSet<Tag>,
    }
}


pub mod group {
    use super::entity::{Id, Member, Tag};
    pub struct Group {
        pub members: Vec<Member>,
    }

    pub struct Table {
        pub groups: Vec<Group>,
    }
}

pub mod condition {
    use std::collections::{HashMap, BTreeSet};
    use super::entity::{Id, Tag};

    pub type Score = f64;

    pub struct RelationPenalty {
        pub scores: HashMap<BTreeSet<Id>, Score>,
        pub default: f64,
    }

    impl RelationPenalty {
        pub fn new(default: Score) -> RelationPenalty {
            RelationPenalty {
                scores: HashMap::new(),
                default,
            }
        }
        pub fn get_pair(&self, ids: [Id; 2]) -> Score {
            self.scores.get(&BTreeSet::from(ids)).copied().unwrap_or(self.default)
        }
    }


    pub enum Range {
        Ratio {min: f64, max: f64},
        Count {min: usize, max: usize},
    }
    pub struct Constraint (pub HashMap<Tag, Range>);

    pub struct Condition {
        pub penalty: RelationPenalty,
        pub constraint: Constraint,
    }
}
