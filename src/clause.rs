use std::collections::{BTreeSet, HashMap};
use typeql::Variable;

pub enum Literal {
    Has { owner: Variable, attribute: Variable },
    Links { relation: Variable, player: Variable, role: String },
    Isa { instance: Variable, type_: String },
    CompareVariables { lhs: Variable, rhs: Variable  },
    CompareConstant { lhs: Variable, rhs: typeql::value::ValueLiteral  },
}

pub struct Clause {
    conjunction: Vec<typeql::Pattern>,
    types_: HashMap<Variable, BTreeSet<String>>,
}

impl Clause {
    fn refine(&self) -> Clause {
        todo!()
    }
}
