use std::collections::HashSet;
use typedb_driver::{Promise, TransactionType, TypeDBDriver};
use typedb_driver::answer::ConceptRow;
use typeql::schema::definable::function::Stream;
use crate::clause::{Clause, ClauseVariable};
use crate::Instance;

mod tilde;
mod classification;

pub struct TypeDBHelper {
    pub driver: TypeDBDriver,
    pub database: String,
}

impl TypeDBHelper {
    pub fn new(driver: TypeDBDriver, database: String) -> Self {
        Self { driver, database }
    }
}

impl TypeDBHelper {
    // Returns example instances which satisfy the clause
    pub fn test_clause(&self, clause: &Clause, instance_var_name: &str) -> Result<HashSet<Instance>, typedb_driver::Error> {
        let query = format!("match {}; select ${};", clause.to_typeql(), instance_var_name);
        let tx = self.driver.transaction(self.database.as_str(), TransactionType::Read)?;
        tx.query(query).resolve()?.into_rows().map(|row| {
            Ok(row?.get(instance_var_name).unwrap().unwrap().into())
        }).collect()
    }

    pub(crate) fn query(&self, query: &str) -> Result<impl Iterator<Item=Result<ConceptRow, typedb_driver::Error>>, typedb_driver::Error> {
        let tx = self.driver.transaction(self.database.as_str(), TransactionType::Read)?;
        Ok(tx.query(query).resolve()?.into_rows())
    }

}
