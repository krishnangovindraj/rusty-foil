use std::fmt::{Formatter, Pointer};

use itertools::Itertools;
use tracing::{Level, event};
use typedb_driver::answer::concept_document::Leaf;

use crate::{
    INDENT, Instance, TypeDBHelper,
    clause::Clause,
    language::HypothesisLanguage,
    tilde::{
        TildeResult,
        classification::{Dataset, entropy, weighted_information_gain},
    },
};

const MIN_SPLIT_EXAMPLES: usize = 4;
const MIN_SPLIT_ENTROPY: f64 = 1e-6;
const MIN_SPLIT_GAIN: f64 = 1e-3;
const MAX_LOOKAHEAD: usize = 3;

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

    pub(super) fn try_split_recursive(
        self,
        typedb: &TypeDBHelper,
        language: &HypothesisLanguage,
        depth: usize,
    ) -> TildeResult<TildeTree> {
        let split = self.try_split(typedb, language)?;
        if let TildeTree::Inner(InnerNode { test_prefix, dataset, left, right }) = split {
            let (left, right) = match (*left, *right) {
                (TildeTree::Leaf(l), TildeTree::Leaf(r)) => (
                    Box::new(l.try_split_recursive(typedb, language, depth + 1)?),
                    Box::new(r.try_split_recursive(typedb, language, depth + 1)?),
                ),
                _ => unreachable!(),
            };
            Ok(TildeTree::Inner(InnerNode { test_prefix, dataset, left, right }))
        } else {
            Ok(split)
        }
    }

    pub fn try_split(self, typedb: &TypeDBHelper, language: &HypothesisLanguage) -> TildeResult<TildeTree> {
        // TODO: Consider things like max-depth etc
        let dont_split = self.dataset.examples.len() < MIN_SPLIT_EXAMPLES || entropy(&self.dataset) < MIN_SPLIT_ENTROPY;
        if dont_split {
            event!(Level::TRACE, "Don't split. Entropy: {}", entropy(&self.dataset));
            return Ok(TildeTree::Leaf(self));
        }
        let mut depth = 0;
        let mut best_split_opt: Option<(f64, Clause, Dataset, Dataset)> = None;
        while best_split_opt.as_ref().map(|bs| bs.0 < MIN_SPLIT_GAIN).unwrap_or(true) && depth < MAX_LOOKAHEAD {
            depth += 1;
            let mut refinements = Vec::new();
            refine_to_length(&self.test_prefix, &language, depth, &mut refinements);
            best_split_opt = refinements
                .into_iter()
                .map(|refined| {
                    let covered_instances = typedb.test_clause(&refined).unwrap(); // TODO: Handle error
                    let (left_ds, right_ds) = self.dataset.split_on(covered_instances);
                    let gain = weighted_information_gain(&self.dataset, [&left_ds, &right_ds]);
                    (gain, refined, left_ds, right_ds)
                })
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
        match best_split_opt {
            None => Ok(TildeTree::Leaf(self)),
            Some((_gain, clause, left_ds, right_ds)) => {
                // Note: Right test_prefix is unchanged
                let left = Box::new(TildeTree::Leaf(LeafNode { test_prefix: clause.clone(), dataset: left_ds }));
                let right =
                    Box::new(TildeTree::Leaf(LeafNode { test_prefix: self.test_prefix.clone(), dataset: right_ds }));
                // tracing::trace!(
                println!(
                    "---Splitting with gain {}---\n{}\n -to- \n{}\n -and- \n{}\n---",
                    _gain,
                    self,
                    left,
                    right
                );
                Ok(TildeTree::Inner(InnerNode {
                    test_prefix: self.test_prefix.clone(),
                    dataset: self.dataset,
                    left,
                    right,
                }))
            }
        }
    }

    fn target(&self) -> Option<super::classification::ExampleClassType> {
        self.dataset.majority_class()
    }
}

fn refine_to_length(
    clause: &Clause,
    language: &HypothesisLanguage,
    depth: usize,
    collect: &mut Vec<Clause>,
) -> TildeResult<()> {
    let refined = clause.refine(language);
    if depth == 1 {
        collect.extend(refined)
    } else {
        for new_clause in refined {
            refine_to_length(&new_clause, language, depth - 1, collect)?;
        }
    }
    Ok(())
}

impl std::fmt::Display for TildeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl TildeTree {
    fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> std::fmt::Result {
        let indent = INDENT.repeat(depth);
        match self {
            TildeTree::Leaf(leaf) => leaf.fmt_with_indent(f, depth),
            TildeTree::Inner(inner) => inner.fmt_with_indent(f, depth),
        }
    }
}
impl std::fmt::Display for LeafNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl LeafNode {
    fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> std::fmt::Result {
        let indent = INDENT.repeat(depth);
        let counts: Vec<String> = self
            .dataset
            .count_by_class()
            .iter()
            .map(|(class, &count)| format!("{}: {}", if *class { "+" } else { "-" }, count))
            .collect();
        let total = self.dataset.examples.len();
        writeln!(f, "{}(samples={}, {}) LEAF [", indent, total, counts.join(", "))?;
        self.test_prefix.fmt_with_indent(f, depth + 1)?;
        writeln!(f, "{}]", indent)
    }
}

impl std::fmt::Display for InnerNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl InnerNode {
    fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> std::fmt::Result {
        let indent = INDENT.repeat(depth);
        let counts: Vec<String> = self
            .dataset
            .count_by_class()
            .iter()
            .map(|(class, &count)| format!("{}: {}", if *class { "+" } else { "-" }, count))
            .collect();
        let total = self.dataset.examples.len();
        writeln!(f, "{}(samples={}, {}) INNER [", indent, total, counts.join(", "))?;
        self.test_prefix.fmt_with_indent(f, depth + 1)?;
        writeln!(f, "{}]", indent)?;

        // Print left branch (true condition)
        writeln!(f, "{}  if true:", indent)?;
        self.left.fmt_with_indent(f, depth + 2)?;
        writeln!(f)?;

        // Print right branch (false condition)
        writeln!(f, "{}  if false:", indent)?;
        self.right.fmt_with_indent(f, depth + 2)?;

        writeln!(f, "{}}}", indent)
    }
}
