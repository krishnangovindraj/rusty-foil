use std::collections::{BTreeSet, HashMap, HashSet};
use crate::language::{Schema, SchemaType};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClauseVariable(String);

#[derive(Debug, Clone)]
pub enum Literal {
    Has { owner: ClauseVariable, attribute: ClauseVariable },
    Links { player: ClauseVariable, role: SchemaType, relation: ClauseVariable, other_role: SchemaType, other_player: ClauseVariable },
    Isa { instance: ClauseVariable, type_: SchemaType },
    CompareVariables { lhs: ClauseVariable, rhs: ClauseVariable  },
    CompareConstant { lhs: ClauseVariable, rhs: typeql::value::ValueLiteral  },
}

#[derive(Debug, Clone)]
pub struct Clause {
    conjunction: Vec<Literal>,
    // Contains possible types for each variable, based on the schema
    types_: HashMap<ClauseVariable, BTreeSet<SchemaType>>,
    value_types: HashMap<ClauseVariable, BTreeSet<typedb_driver::concept::ValueType>>,
}

impl Clause {
    pub fn refine(&self, schema: &Schema) -> Vec<Clause> {
        let mut refinements = Vec::new();

        // For each existing variable, generate refinements
        let existing_vars: Vec<_> = self.types_.keys().cloned().collect();
        for (var, possible_types) in self.types_.iter() {
            // 1. Add type constraints (Isa literals)
            if possible_types.len() > 1 {
                for type_ in possible_types {
                    refinements.push(self.extend_with_isa(var, type_));
                }
            }

            // 2. Add attribute ownership (Has literals)
            for type_ in possible_types {
                for attr_type in schema.owns.get(&type_).unwrap_or(&BTreeSet::new()) {
                    refinements.push(self.extend_with_has(var, attr_type));
                }
            }

            // 3. Add relation participation (Links literals)
            for type_ in possible_types {
                for role_type in schema.plays.get(type_).unwrap_or(&BTreeSet::new()) {
                    let other_roles = schema.related_by.get(role_type).unwrap().iter()
                        .flat_map(|relation| schema.relates.get(relation).unwrap().iter())
                        .cloned()
                        .collect::<HashSet<_>>();
                    for other_role in other_roles {
                        if let Some(new_clause) = self.extend_with_links(var, role_type, &other_role, schema) {
                            refinements.push(new_clause);
                        }
                    }
                }
            }
        }

        // 4. Add comparisons between existing variables
        for i in 0..existing_vars.len() {
            for j in (i + 1)..existing_vars.len() {
                let var1 = &existing_vars[i];
                let var2 = &existing_vars[j];
                if let (Some(types1), Some(types2)) = (self.types_.get(var1), self.types_.get(var2)) {
                    if types1.intersection(types2).count() > 0 {
                        refinements.push(self.extend_with_comparison(var1, var2));
                    }
                }
            }
        }

        refinements
    }

    fn extend_with_isa(&self, var: &ClauseVariable, type_: &SchemaType) -> Clause {
        let mut new_clause = self.clone();
        new_clause.conjunction.push(Literal::Isa {
            instance: var.clone(),
            type_: type_.clone(),
        });

        // Narrow the types for this variable
        let mut new_types = BTreeSet::new();
        new_types.insert(type_.clone());
        new_clause.types_.insert(var.clone(), new_types);

        new_clause
    }

    fn extend_with_has(&self, owner: &ClauseVariable, attr_type: &SchemaType) -> Clause {
        let mut new_clause = self.clone();
        let attr_var = self.fresh_variable(attr_type, None);
        new_clause.conjunction.push(Literal::Has {
            owner: owner.clone(),
            attribute: attr_var.clone(),
        });

        // Add the attribute variable to types

        let mut attr_types = BTreeSet::new();
        attr_types.insert(attr_type.clone());
        new_clause.types_.insert(attr_var, attr_types);

        new_clause
    }

    fn extend_with_links(
        &self,
        player: &ClauseVariable,
        role_type: &SchemaType,
        other_role_type: &SchemaType,
        schema: &Schema,
    ) -> Option<Clause> {
        let mut new_clause = self.clone();
        let rel_var = self.fresh_variable(role_type, Some(format!("rel__{}", other_role_type.label()).as_str()));
        let other_player = self.fresh_variable(other_role_type, None);
        if let Some(other_player_types) = schema.players.get(other_role_type) {
            // Add relation link for current variable
            new_clause.conjunction.push(Literal::Links {
                player: player.clone(),
                role: role_type.clone(),
                relation: rel_var.clone(),
                other_role: other_role_type.clone(),
                other_player: other_player.clone(),
            });

            // Add types for new variables
            let mut rel_types = schema.related_by.get(role_type).unwrap().clone();
            rel_types.retain(|x| schema.related_by.get(other_role_type).unwrap().contains(x));
            new_clause.types_.insert(rel_var, rel_types);

            new_clause.types_.insert(other_player, other_player_types.clone());
            Some(new_clause)
        } else {
            None
        }
    }

    fn extend_with_comparison(&self, var1: &ClauseVariable, var2: &ClauseVariable) -> Clause {
        let mut new_clause = self.clone();
        new_clause.conjunction.push(Literal::CompareVariables {
            lhs: var1.clone(),
            rhs: var2.clone(),
        });
        new_clause
    }

    fn fresh_variable(&self, type_: &SchemaType, suffix_opt: Option<&str>) -> ClauseVariable {
        let name = if let Some(suffix) = suffix_opt {
            format!("{}_{}_{}", type_.label().replace(":", "__"), self.conjunction.len(), suffix)
        } else {
            format!("{}_{}", type_.label().replace(":", "__"), self.conjunction.len())
        };
        ClauseVariable(name)
    }
}
