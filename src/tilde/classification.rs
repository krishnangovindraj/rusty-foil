use std::collections::{HashMap, HashSet};
use std::ops::AddAssign;
use typedb_driver::{Promise};
use crate::clause::{Clause, ClauseVariable};
use crate::Instance;
use crate::language::{HypothesisLanguage, SchemaType};
use crate::TypeDBHelper;

type ExampleClassType = bool;
#[derive(Clone)]
pub struct Example {
    pub instance: Instance,
    pub class: ExampleClassType,
}

#[derive(Clone)]
pub struct Dataset {
    pub examples: Vec<Example>,
}

impl Dataset {
    pub(super) fn count_by_class(&self) -> HashMap<ExampleClassType, usize> {
        let mut counters = HashMap::new();
        for example in &self.examples {
            counters.entry(example.class.clone()).or_insert(0).add_assign(1);
        }
        counters
    }

    pub(super) fn split_on(&self, included_instances: &HashSet<Instance>) -> (Dataset, Dataset) {
        let mut left = Dataset { examples: Vec::with_capacity(included_instances.len()) };
        let mut right = Dataset { examples: Vec::with_capacity(self.examples.len() - included_instances.len()) };
        for e in &self.examples {
            if included_instances.contains(&e.instance) {
                left.examples.push(e.clone());
            } else {
                right.examples.push(e.clone());
            }
        }
        (left, right)
    }
}

pub struct ClassificationTask {
    pub target_type: SchemaType, // The type we're classifying.
    pub class_type: SchemaType, // The type we use as class.
    pub dataset: Dataset,
}

pub(super) fn entropy(dataset: &Dataset) -> f64 {
    let counters = dataset.count_by_class();
    _entropy(&counters.values().cloned().collect::<Vec<_>>())
}

fn _entropy(counts: &[usize]) -> f64 {
    let total = counts.iter().sum::<usize>();
    if total == 0 {
        return 0f64
    }
    counts.iter().map(|count|{
        let p = *count as f64 / total as f64;
        -p * p.log2()
    }).sum()
}

pub(super) fn weighted_information_gain(before: &Dataset, after: [&Dataset; 2]) -> f64 {
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
            ClauseVariable::INSTANCE_VAR_NAME, target_type, class_attribute_label, Self::CLASS_VAR_NAME
        );
        let tx = typedb.read_tx()?;
        let examples = tx.query(query.as_str()).resolve()?.into_rows().map(|row_result| {
            let row = row_result?;
            Ok::<_, typedb_driver::Error>(Example {
                instance: row.get(ClauseVariable::INSTANCE_VAR_NAME).unwrap().unwrap().into(),
                class: row.get(Self::CLASS_VAR_NAME).unwrap().unwrap().try_get_boolean().unwrap(),
            })
        }).collect::<Result<Vec<Example>, _>>()?;
        let dataset = Dataset { examples };

        Ok(Self { class_type, target_type, dataset })
    }

    pub(super) fn initial_clause(&self, language: &HypothesisLanguage) -> Clause {
        Clause::new_empty().extend_with_isa(
            &ClauseVariable(ClauseVariable::INSTANCE_VAR_NAME.to_owned()),
            &self.target_type,
            &language.schema
        )
    }
}
