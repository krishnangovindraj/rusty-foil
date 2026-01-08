use crate::language::{HypothesisLanguage};
use typedb_driver::{TransactionType, TypeDBDriver, Promise};

pub struct LearningTask {
    pub driver: TypeDBDriver,
    pub database_name: String,

    pub class_attribute_label: String, // Label of the class attribute
    pub language: HypothesisLanguage,
    pub dataset: Vec<Example>,
}

pub(crate) struct Example {
    instance: typedb_driver::concept::Concept,
    class: typedb_driver::concept::Concept, // TypeQL literals to be pasted in
}

impl LearningTask {
    pub fn discover(
        driver: TypeDBDriver,
        database_name: String,
        language: HypothesisLanguage,
        class_attribute_label: String,
    ) -> Result<Self, typedb_driver::Error> {
        let query = format!("match $instance has {class_attribute_label} $class;");

        let tx = driver.transaction(database_name.as_str(), TransactionType::Read)?;
        let dataset = tx.query(query).resolve()?.into_rows().into_iter().map(|row_result| {
            let row = row_result?;
            Ok::<_, typedb_driver::Error>(Example {
                instance: row.get("instance").unwrap().unwrap().clone(),
                class: row.get("class").unwrap().unwrap().clone(),
            })
        }).collect::<Result<Vec<_>, _>>()?;
        Ok(Self { driver, database_name, class_attribute_label, language, dataset })
    }
}
