use macros::Component;

use crate::ecs::World;

/// What type of access does the query have.
#[derive(Clone, Copy, Default)]
pub enum Access {
    #[default]
    Noop,
    Include,
    Exclude,
    Read,
    Write,
}

impl Access {
    /// Is the access type no operations
    fn is_noop(self) -> bool {
        matches!(self, Self::Noop)
    }
}

/// Definition of a query term
#[derive(Clone)]
struct Term {
    field: u64,
    access: Access,
}

impl Default for Term {
    fn default() -> Self {
        Self {
            field: 0,
            access: Access::Noop,
        }
    }
}

pub struct Query {
    world: World,
    terms: Vec<Term>,
}
