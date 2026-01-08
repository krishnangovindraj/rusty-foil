use crate::language::{HypothesisLanguage};
use typedb_driver::TypeDBDriver;

#[derive(Debug, Clone, PartialEq)]
pub struct Example {
    pub bindings: Vec<(String, String)>,
}

impl Example {
    pub fn new(bindings: Vec<(String, String)>) -> Self {
        Example { bindings }
    }
}

pub struct LearningTask<'a> {
    pub target: (), // TODO
    pub language: HypothesisLanguage,
    pub database_name: String,
    pub driver: &'a TypeDBDriver,
    pub positive_examples: Vec<Example>,
    pub negative_examples: Vec<Example>,
}

impl<'a> LearningTask<'a> {
    pub fn new(
        target: (),
        language: HypothesisLanguage,
        database_name: String,
        driver: &'a TypeDBDriver,
        positive_examples: Vec<Example>,
        negative_examples: Vec<Example>,
    ) -> Self {
        todo!()
    }
}
