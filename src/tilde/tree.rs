use std::fmt::{Formatter, Pointer};
use itertools::Itertools;
use typedb_driver::answer::concept_document::Leaf;
use crate::clause::Clause;
use crate::{Instance, TypeDBHelper};
use crate::language::HypothesisLanguage;
use crate::tilde::classification::{entropy, weighted_information_gain, Dataset};
use crate::tilde::TildeResult;

const MIN_SPLIT_EXAMPLES: usize = 4;
const MIN_SPLIT_ENTROPY: f64 = 0.0;
const MIN_SPLIT_GAIN: f64 = 0.0;

pub enum TildeTree {
    Leaf(LeafNode),
    Inner(InnerNode),
}

pub struct LeafNode {
    test_prefix: Clause,
    dataset: Dataset,
}

pub struct InnerNode {
    test_prefix: Clause,
    dataset: Dataset, // TODO: Could remove
    left: Box<TildeTree>,
    right: Box<TildeTree>,
}

impl LeafNode {
    pub(crate) fn new(test_prefix: Clause, dataset: Dataset) -> LeafNode {
        Self { test_prefix, dataset }
    }

    pub(super) fn try_split_recursive(self, typedb: &TypeDBHelper, language: &HypothesisLanguage, depth: usize) -> TildeResult<TildeTree> {
        let split = self.try_split(typedb, language)?;
        if let TildeTree::Inner(InnerNode { test_prefix, dataset, left, right }) = split {
            let (left, right) = match (*left, *right) {
                (TildeTree::Leaf(l), TildeTree::Leaf(r)) => {
                    (
                        Box::new(l.try_split_recursive(typedb, language, depth+1)?),
                        Box::new(r.try_split_recursive(typedb, language, depth+1)?)
                    )
                }
                _ => unreachable!()
            };
            Ok(TildeTree::Inner(InnerNode { test_prefix, dataset, left, right, }))
        } else {
            Ok(split)
        }
    }
    pub fn try_split(self, typedb: &TypeDBHelper, language: &HypothesisLanguage) -> TildeResult<TildeTree> {
        // TODO: Consider things like max-depth etc
        let dont_split = self.dataset.examples.len() < MIN_SPLIT_EXAMPLES ||
            entropy(&self.dataset) < MIN_SPLIT_ENTROPY;
        if dont_split {
            println!("Don't split. Entropy: {}", entropy(&self.dataset));
            return Ok(TildeTree::Leaf(self))
        }
        let refinements = self.test_prefix.refine(&language);
        let best_split_opt = refinements.iter().map(|refined| {
            let covered_instances = typedb.test_clause(refined).unwrap(); // TODO: Handle error
            let (left_ds, right_ds) = self.dataset.split_on(&covered_instances);
            let gain = weighted_information_gain(&self.dataset, [&left_ds, &right_ds]);
            (gain, refined, left_ds, right_ds)
        }).max_by(|a,b| a.0.partial_cmp(&b.0).unwrap());
        match best_split_opt {
            None => {
                Ok(TildeTree::Leaf(self))
            }
            Some((_, clause, left_ds, right_ds)) => {
                // Note: Right test_prefix is unchanged
                let left = Box::new(TildeTree::Leaf(LeafNode { test_prefix: clause.clone(), dataset: left_ds }));
                let right = Box::new(TildeTree::Leaf(LeafNode { test_prefix: self.test_prefix.clone(), dataset: right_ds }));
                println!("===\nSplitting ---\n{}\n -to- \n{}\n -and- \n{}\n===", self, left, right);
                Ok(TildeTree::Inner(InnerNode { test_prefix: self.test_prefix.clone(), dataset: self.dataset, left, right }))
            }
        }
    }

    fn target(&self) -> Option<Instance> {
        todo!("Return majority in dataset")
    }
}


impl std::fmt::Display for TildeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TildeTree::Leaf(inner) => inner.fmt(f),
            TildeTree::Inner(inner) => inner.fmt(f),
        }
    }
}
impl std::fmt::Display for LeafNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let split = self.dataset.count_by_class().values().join("/");
        write!(f, "[ clause: ({}) , cover: ({}) ]", self.test_prefix, split)
    }
}
impl std::fmt::Display for InnerNode {

    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let split = self.dataset.count_by_class().values().join("/");
        write!(f, "[ clause: ({}) , cover: ({}), left: {}, right: {}]", self.test_prefix, split, self.left, self.right)
    }
}
