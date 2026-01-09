use typedb_driver::{Promise, TransactionType, TypeDBDriver};
use crate::language::{HypothesisLanguage, SchemaType};
use super::classification::{Dataset, Example};

struct TildeLearningTask {
    pub driver: TypeDBDriver,
    pub database_name: String,

    pub target_type: SchemaType, // Label of the type. Used for initial clause.
    pub class_attribute_label: String, // Label of the class attribute
    pub language: HypothesisLanguage,
    pub dataset: Dataset,
}


const INSTANCE_VAR_NAME: &'static str = "instance_0";
const CLASS_VAR_NAME: &'static str = "class_0";
const MAX_THEORY_LENGTH: usize = 20 ;
const MAX_CLAUSE_LENGTH: usize = 10 ;
impl TildeLearningTask {

    const INSTANCE_VAR_NAME: &'static str = "instance_0";
    const CLASS_VAR_NAME: &'static str = "class_0";
    const MAX_THEORY_LENGTH: usize = 20 ;
    const MAX_CLAUSE_LENGTH: usize = 10 ;

    pub fn discover(
        driver: TypeDBDriver,
        database_name: String,
        language: HypothesisLanguage,
        target_type_label: String,
        class_attribute_label: String,
    ) -> Result<Self, typedb_driver::Error> {
        let target_type = language.lookup_type(&target_type_label)
            .expect("Expected target_type to be in schema.subtypes");
        let query = format!(
            "match ${} isa {}, has {} ${};",
            Self::INSTANCE_VAR_NAME, target_type_label, class_attribute_label, Self::CLASS_VAR_NAME
        );

        let tx = driver.transaction(database_name.as_str(), TransactionType::Read)?;
        let examples = tx.query(query).resolve()?.into_rows().into_iter().map(|row_result| {
            let row = row_result?;
            Ok::<_, typedb_driver::Error>(Example {
                instance: row.get(Self::INSTANCE_VAR_NAME).unwrap().unwrap().into(),
                class: row.get(Self::CLASS_VAR_NAME).unwrap().unwrap().into(),
            })
        }).collect::<Result<Vec<Example>, _>>()?;
        let dataset = Dataset { examples };

        Ok(Self { driver, database_name, class_attribute_label, target_type, language, dataset })
    }


    pub fn deconstruct(self) -> TypeDBDriver {
        self.driver
    }


}