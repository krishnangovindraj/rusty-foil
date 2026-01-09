use std::collections::{BTreeSet, HashSet};
use crate::language::{HypothesisLanguage, SchemaType};
use typedb_driver::{TransactionType, Promise};
use typedb_driver::concept::Concept;
use crate::Instance;
use crate::clause::{Clause, ClauseVariable};
use crate::tilde::TypeDBHelper;

type FoilExample = Instance;

// Ok it's not a foil task, but I have no time.
pub struct FoilLearningTask {
    pub typedb: TypeDBHelper,

    pub target_type: SchemaType, // Label of the type. Used for initial clause.
    pub class_attribute_label: String, // Label of the class attribute
    pub language: HypothesisLanguage,
    pub positive_examples: HashSet<FoilExample>,
    pub negative_examples: HashSet<FoilExample>,
}

impl FoilLearningTask {

    const INSTANCE_VAR_NAME: &'static str = "instance_0";
    const CLASS_VAR_NAME: &'static str = "class_0";
    const MAX_THEORY_LENGTH: usize = 20 ;
    const MAX_CLAUSE_LENGTH: usize = 10 ;

    pub fn discover(
        typedb: TypeDBHelper,
        language: HypothesisLanguage,
        target_type_label: String,
        class_attribute_label: String,
    ) -> Result<Self, typedb_driver::Error> {
        let query = format!(
            "match ${} isa {}, has {} ${};",
            Self::INSTANCE_VAR_NAME, target_type_label, class_attribute_label, Self::CLASS_VAR_NAME
        );
        let dataset = typedb.query(query.as_str())?.map(|row_result| {
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
        let target_type = language.schema.subtypes.keys().find(|t| t.label() == target_type_label)
            .expect("Expected target_type to be in schema.subtypes").clone();
        Ok(Self { typedb, class_attribute_label, target_type, language, positive_examples, negative_examples })
    }

    pub fn deconstruct(self) -> TypeDBHelper {
        self.typedb
    }

    pub(super) fn initial_clause(&self) -> Clause {
        Clause::new_empty().extend_with_isa(
            &ClauseVariable(Self::INSTANCE_VAR_NAME.to_owned()),
            &self.target_type,
            &self.language.schema
        )
    }

    // FOIL search algorithm
    pub fn search(&self) -> Result<Vec<Clause>, typedb_driver::Error> {
        let mut theory = Vec::new();
        let mut uncovered_positives = self.positive_examples.clone();
        let mut all_negatives = self.negative_examples.clone();

        // Learn clauses until all positive examples are covered
        while !uncovered_positives.is_empty() {
            println!("Learning new clause. Uncovered positives: {}", uncovered_positives.len());

            let Some(clause) = self.learn_clause(&uncovered_positives, &all_negatives)? else { break; };

            // Find which positives this clause covers
            let covered_instances = self.typedb.test_clause(&clause, Self::INSTANCE_VAR_NAME)?;
            println!(
                "Learnt clause: {}; Covers pos/neg: {}/{} \n---",
                clause, uncovered_positives.intersection(&covered_instances).count(),
                all_negatives.intersection(&covered_instances).count(),
            );
            uncovered_positives.retain(|ex| !covered_instances.contains(ex));
            // uncovered_negatives.retain(|ex| !covered_instances.contains(ex));
            theory.push(clause);

            // Safety check to prevent infinite loops
            if theory.len() > Self::MAX_THEORY_LENGTH {
                eprintln!("Warning: Learned {} clauses, stopping to prevent infinite loop", Self::MAX_THEORY_LENGTH);
                break;
            }
        }

        println!("Final theory has {} clauses", theory.len());
        Ok(theory)
    }

    // Learn a single clause that covers some positive examples without covering negatives
    fn learn_clause(
        &self,
        target_positives: &HashSet<FoilExample>,
        target_negatives: &HashSet<FoilExample>
    ) -> Result<Option<Clause>, typedb_driver::Error> {
        let mut clause = self.initial_clause();

        let mut covered_positives = target_positives.clone();
        let mut covered_negatives = target_negatives.clone();

        while  clause.len() < Self::MAX_CLAUSE_LENGTH && !covered_negatives.is_empty() && !covered_positives.is_empty() {
            // Get instances covered by current clause
            let covered_instances = self.typedb.test_clause(&clause, Self::INSTANCE_VAR_NAME)?;

            covered_positives.retain(|x| covered_instances.contains(x));
            covered_negatives.retain(|x| covered_instances.contains(x));
            println!("  Current clause covers: {} pos, {} neg", covered_positives.len(), covered_negatives.len());

            // Generate and evaluate refinements
            let refinements = clause.refine(&self.language);

            // Find best refinement using FOIL information gain
            let mut best_clause = None;
            let mut best_gain = f64::NEG_INFINITY;

            for refinement in refinements {
                let refined_covered: HashSet<Instance> = self.typedb.test_clause(&refinement, Self::INSTANCE_VAR_NAME)?;
                let p_new = refined_covered.intersection(&target_positives).count() as f64;
                let n_new = refined_covered.intersection(&target_negatives).count() as f64;

                // Skip refinements that cover no positives
                if p_new == 0.0 {
                    continue;
                }

                // FOIL information gain
                let gain = self.foil_gain(
                    covered_positives.len() as f64,
                    covered_negatives.len() as f64,
                    p_new,
                    n_new,
                );

                if gain > best_gain { // TODO: Verify that bigger is better
                    best_gain = gain;
                    best_clause = Some(refinement);
                }
            }

            match best_clause {
                Some(new_clause) => {
                    println!("  Best refinement gain: {:.4}", best_gain);
                    clause = new_clause;
                }
                None => {
                    println!("  No improving refinement found");
                    break;
                }
            }
        }
        if covered_positives.is_empty() {
            println!("  Clause covers no positives, returning None");
            Ok(None)
        } else {
            if covered_negatives.is_empty() {
                println!("  Clause is pure (no negatives covered)");
            }
            Ok(Some(clause))
        }
    }

    // FOIL information gain heuristic
    fn foil_gain(&self, p_old: f64, n_old: f64, p_new: f64, n_new: f64) -> f64 {
        if p_new == 0.0 || p_old == 0.0 {
            return f64::NEG_INFINITY;
        }
        // TODO: Verify we're not missing a -ve and the best clause has the highest gain
        let old_score = (p_old / (p_old + n_old)).log2();
        let new_score = (p_new / (p_new + n_new)).log2();

        p_new * (new_score - old_score)
    }
}
