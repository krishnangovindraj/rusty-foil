use std::collections::HashMap;
use std::ops::AddAssign;
use typedb_driver::{TransactionType, TypeDBDriver};
use crate::clause::{Clause, ClauseVariable};
use crate::Instance;
use crate::language::{HypothesisLanguage, SchemaType};
use crate::tilde::TypeDBHelper;

pub struct Example {
    pub instance: Instance,
    pub class: Instance,
}

pub struct Dataset {
    pub examples: Vec<Example>,
}

pub struct ClassificationTask {
    pub target_type: SchemaType, // The type we're classifying.
    pub class_type: SchemaType, // The type we use as class.
    pub dataset: Dataset,
}

fn entropy(dataset: &Dataset) -> f64 {
    let n_examples = dataset.examples.len();
    if n_examples == 0 {
        return 0f64
    }
    let mut counters: HashMap<Instance, usize> = HashMap::new();
    for example in &dataset.examples {
        counters.entry(example.class.clone()).or_insert(0).add_assign(1);
    }
    let total = counters.values().sum::<usize>();

    let mut entropy = 0f64;
    for (_, count) in counters {
        let p = count as f64 / total as f64;
        entropy +=  -p * p.log2();
    }
    entropy
}

fn weighted_information_gain(before: &Dataset, after: Vec<&Dataset>) -> f64 {
    let weighted_entropy_after = after.iter().map(|d| {
        d.examples.len() as f64 * entropy(d)
    }).sum::<f64>() / before.examples.len() as f64;
    entropy(before) - weighted_entropy_after
}

impl ClassificationTask {

    const INSTANCE_VAR_NAME: &'static str = "instance_0";
    const CLASS_VAR_NAME: &'static str = "class_0";
    const MAX_THEORY_LENGTH: usize = 20 ;
    const MAX_CLAUSE_LENGTH: usize = 10 ;

    pub fn discover(
        typedb: &TypeDBHelper,
        language: &HypothesisLanguage,
        target_type_label: &str,
        class_attribute_label: &str,
    ) -> Result<Self, typedb_driver::Error> {
        let target_type = language.lookup_type(target_type_label)
            .expect("target_type not found");
        let class_type = language.lookup_type(class_attribute_label)
            .expect("class_type not found");
        let query = format!(
            "match ${} isa {}, has {} ${};",
            Self::INSTANCE_VAR_NAME, target_type, class_attribute_label, Self::CLASS_VAR_NAME
        );

        let examples = typedb.query(query.as_str())?.into_iter().map(|row_result| {
            let row = row_result?;
            Ok::<_, typedb_driver::Error>(Example {
                instance: row.get(Self::INSTANCE_VAR_NAME).unwrap().unwrap().into(),
                class: row.get(Self::CLASS_VAR_NAME).unwrap().unwrap().into(),
            })
        }).collect::<Result<Vec<Example>, _>>()?;
        let dataset = Dataset { examples };

        Ok(Self { class_type, target_type, dataset })
    }

    fn initial_clause(&self, language: &HypothesisLanguage) -> Clause {
        Clause::new_empty().extend_with_isa(
            &ClauseVariable(Self::INSTANCE_VAR_NAME.to_owned()),
            &self.target_type,
            &language.schema
        )
    }
}
