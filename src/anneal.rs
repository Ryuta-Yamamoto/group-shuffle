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
