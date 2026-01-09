use crate::language::{HypothesisLanguage};
use crate::tilde::TildeResult;
use crate::tilde::tree::{LeafNode, TildeTree};
use crate::TypeDBHelper;
use super::classification::{ClassificationTask, Dataset};


pub struct TildeLearningTask {
    pub typedb: TypeDBHelper,

    pub task: ClassificationTask, // Label of the type. Used for initial clause.
    pub language: HypothesisLanguage,
}

impl TildeLearningTask {

    const INSTANCE_VAR_NAME: &'static str = "instance_0";
    const CLASS_VAR_NAME: &'static str = "class_0";
    const MAX_THEORY_LENGTH: usize = 20 ;
    const MAX_CLAUSE_LENGTH: usize = 10 ;

    pub fn discover(
        typedb: TypeDBHelper,
        language: HypothesisLanguage,
        target_type_label: &str,
        class_attribute_label: &str,
    ) -> TildeResult<Self> {
        let task = ClassificationTask::discover(&typedb, &language, target_type_label, class_attribute_label)?;
        Ok(Self { typedb, task, language })
    }


    pub fn deconstruct(self) -> TypeDBHelper {
        self.typedb
    }

    pub fn search(&self) -> TildeResult<TildeTree> {
        let mut root = LeafNode::new(self.task.initial_clause(&self.language), self.task.dataset.clone());
        root.try_split_recursive(&self.typedb, &self.language, 0)
    }
}