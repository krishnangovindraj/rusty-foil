use std::collections::{BTreeSet, HashSet};
use crate::language::{HypothesisLanguage};
use typedb_driver::{TransactionType, TypeDBDriver, Promise};
use typedb_driver::concept::Concept;
use crate::Instance;
use crate::clause::Clause;

type FoilExample = Instance;

pub struct LearningTask {
    pub driver: TypeDBDriver,
    pub database_name: String,

    pub class_attribute_label: String, // Label of the class attribute
    pub language: HypothesisLanguage,
    pub positive_examples: HashSet<FoilExample>,
    pub negative_examples: HashSet<FoilExample>,
}

impl LearningTask {

    const INSTANCE_VAR_NAME: &'static str = "instance_0";
    const CLASS_VAR_NAME: &'static str = "class_0";
    pub fn discover(
        driver: TypeDBDriver,
        database_name: String,
        language: HypothesisLanguage,
        class_attribute_label: String,
    ) -> Result<Self, typedb_driver::Error> {
        let query = format!(
            "match ${} has {} ${};",
            Self::INSTANCE_VAR_NAME, class_attribute_label, Self::CLASS_VAR_NAME
        );

        let tx = driver.transaction(database_name.as_str(), TransactionType::Read)?;
        let dataset = tx.query(query).resolve()?.into_rows().into_iter().map(|row_result| {
            let row = row_result?;
            Ok::<_, typedb_driver::Error>((
                row.get(Self::INSTANCE_VAR_NAME).unwrap().unwrap().clone(),
                row.get(Self::CLASS_VAR_NAME).unwrap().unwrap().try_get_boolean().expect("Expected class attribute to be boolean for FOIL tasks"),
            ))
        }).collect::<Result<Vec<(Concept, bool)>, _>>()?;
        let positive_examples = dataset.iter()
            .filter_map(|(concept, is_positive)| is_positive.then_some(concept.into()))
            .collect();
        let negative_examples = dataset.iter()
            .filter_map(|(concept, is_positive)| (!is_positive).then_some(concept.into()))
            .collect();
        Ok(Self { driver, database_name, class_attribute_label, language, positive_examples, negative_examples })
    }

    pub(super) fn initial_clause(&self) -> Clause {
        Clause::new_empty()
    }

    // Returns example instances which satisfy the clause
    pub(super) fn run_query(&self, driver: TypeDBDriver, clause: Clause) -> Result<Vec<Concept>, typedb_driver::Error> {
        let query = format!("match {}; select ${};", clause.to_typeql(), Self::INSTANCE_VAR_NAME);

        let tx = driver.transaction(&self.database_name, TransactionType::Read)?;
        tx.query(query).resolve()?.into_rows().map(|row| {
            Ok(row?.get(Self::INSTANCE_VAR_NAME).unwrap().unwrap().clone())
        }).collect()
    }

    // if-else clauses, I think
    fn search(&self) -> Result<Vec<Clause>, typedb_driver::Error> {

    }
}
