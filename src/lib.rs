use std::collections::HashSet;

use typedb_driver::{Promise, Transaction, TransactionType, TypeDBDriver, answer::ConceptRow, concept::Concept};

use crate::clause::{Clause, ClauseVariable};

pub mod clause;
pub mod language;

pub mod foil;
pub mod tilde;

const INDENT: &'static str = "  ";

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Instance(typedb_driver::IID);

impl From<&Concept> for Instance {
    fn from(value: &Concept) -> Self {
        Self(value.try_get_iid().expect("Expected instance variant which has IID").clone())
    }
}

pub struct TypeDBHelper {
    pub driver: TypeDBDriver,
    pub database: String,
}

impl TypeDBHelper {
    pub fn new(driver: TypeDBDriver, database: String) -> Self {
        Self { driver, database }
    }

    // Returns example instances which satisfy the clause
    pub fn test_clause(&self, clause: &Clause) -> Result<HashSet<Instance>, typedb_driver::Error> {
        let query = format!("match {}; select ${};", clause.to_typeql(), ClauseVariable::INSTANCE_VAR_NAME);
        let tx = self.driver.transaction(self.database.as_str(), TransactionType::Read)?;
        tx.query(query)
            .resolve()?
            .into_rows()
            .map(|row| Ok(row?.get(ClauseVariable::INSTANCE_VAR_NAME).unwrap().unwrap().into()))
            .collect()
    }

    pub(crate) fn read_tx(&self) -> Result<Transaction, typedb_driver::Error> {
        self.driver.transaction(self.database.as_str(), TransactionType::Read)
    }
    //
    // pub(crate) fn query(&self, query: &str) -> Result<impl Iterator<Item=Result<ConceptRow, typedb_driver::Error>>, typedb_driver::Error> {
    //     let tx = self.driver.transaction(self.database.as_str(), TransactionType::Read)?;
    //     Ok(tx.query(query))
    // }
}
