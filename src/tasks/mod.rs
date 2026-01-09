use std::collections::HashSet;
use typedb_driver::{Promise, TransactionType, TypeDBDriver};
use crate::clause::{Clause, ClauseVariable};
use crate::Instance;

pub mod foil;
// mod tilde;
// mod measures;

pub struct Example {
    pub instance: Instance,
    pub class: Instance,
}

pub struct Dataset {
    pub examples: Vec<Example>,
}

// Returns example instances which satisfy the clause
pub fn test_clause(driver: &TypeDBDriver, database: &str, clause: &Clause, instance_var: &ClauseVariable) -> Result<HashSet<Instance>, typedb_driver::Error> {
    let query = format!("match {}; select ${};", clause.to_typeql(), instance_var.name());
    let tx = driver.transaction(&database, TransactionType::Read)?;
    tx.query(query).resolve()?.into_rows().map(|row| {
        Ok(row?.get(instance_var.name()).unwrap().unwrap().into())
    }).collect()
}
