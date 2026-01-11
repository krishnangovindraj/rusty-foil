use std::{
    collections::{BTreeSet, HashMap},
    fmt::Formatter,
};

use itertools::Itertools;

use crate::{
    INDENT,
    language::{HypothesisLanguage, Schema, SchemaType},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClauseVariable(pub String);

impl ClauseVariable {
    pub const INSTANCE_VAR_NAME: &'static str = "instance";
    pub(crate) fn name(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for ClauseVariable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}", self.name())
    }
}

#[derive(Debug, Clone)]
enum ValueComparator {
    Eq,
    Neq,
    Lte,
    Gte,
}

impl ValueComparator {
    const VALUES: [ValueComparator; 4] =
        [ValueComparator::Eq, ValueComparator::Neq, ValueComparator::Lte, ValueComparator::Gte];

    fn to_typeql(&self) -> typeql::token::Comparator {
        match self {
            ValueComparator::Eq => typeql::token::Comparator::Eq,
            ValueComparator::Neq => typeql::token::Comparator::Neq,
            ValueComparator::Lte => typeql::token::Comparator::Lte,
            ValueComparator::Gte => typeql::token::Comparator::Gte,
        }
    }
}
impl std::fmt::Display for ValueComparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.to_typeql().fmt(f)
    }
}

#[derive(Debug, Clone)]
pub enum ClauseLiteral {
    Has { owner: ClauseVariable, type_: SchemaType, attribute: ClauseVariable },
    HasValue { owner: ClauseVariable, type_: SchemaType, value: typedb_driver::concept::value::Value },
    Links { relation: ClauseVariable, role: SchemaType, player: ClauseVariable },
    Isa { instance: ClauseVariable, type_: SchemaType },
    CompareVariables { lhs: ClauseVariable, comparator: ValueComparator, rhs: ClauseVariable },
    CompareConstant { lhs: ClauseVariable, comparator: ValueComparator, rhs: typedb_driver::concept::value::Value },
}

impl ClauseLiteral {
    fn to_typeql(&self) -> String {
        match self {
            ClauseLiteral::Has { owner, type_, attribute } => {
                format!("{owner} has {type_} {attribute}")
            },
            ClauseLiteral::HasValue { owner, type_, value } => {
                format!("{owner} has {type_} {value}")
            },
            ClauseLiteral::Links { player, role, relation } => {
                let unscoped_role = role.label().rsplit_once(":").unwrap().1;
                format!("{relation} links ({unscoped_role}: {player})")
            }
            ClauseLiteral::Isa { instance, type_ } => {
                format!("{instance} isa {type_}")
            }
            ClauseLiteral::CompareVariables { lhs, comparator, rhs } => {
                format!("{lhs} {comparator} {rhs}")
            }
            ClauseLiteral::CompareConstant { lhs, comparator, rhs } => {
                format!("{lhs} {comparator} {rhs}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Clause {
    conjunction: Vec<ClauseLiteral>,
    // Contains possible types for each variable, based on the schema
    types_: HashMap<ClauseVariable, BTreeSet<SchemaType>>,
    // value_types: HashMap<ClauseVariable, BTreeSet<typedb_driver::concept::ValueType>>,
}

// TODO: Lookahead
// TODO: Unify existing variables rather than always introducing a new one
impl Clause {
    pub fn new_empty() -> Clause {
        Self { conjunction: Vec::new(), types_: HashMap::new() } // value_types: HashMap::new() }
    }

    pub fn new_from_isa(type_: SchemaType, schema: &Schema) -> Self {
        let clause = Self::new_empty();
        clause.extend_with_isa(&clause.fresh_variable(&type_, None), &type_, schema)
    }

    pub(crate) fn len(&self) -> usize {
        self.conjunction.len()
    }

    pub fn refine(&self, language: &HypothesisLanguage) -> Vec<Clause> {
        let mut refinements = Vec::new();
        let schema = &language.schema;
        // For each existing variable, generate refinements
        // TODO: Unify vars in hypothesis
        let existing_vars: Vec<_> = self.types_.keys().cloned().collect();
        for (var, possible_types) in self.types_.iter() {
            // 1. Add type constraints (Isa literals)
            if possible_types.len() > 1 {
                for type_ in possible_types {
                    refinements.push(self.extend_with_isa(var, type_, schema));
                }
            }

            // Attribute ownerships
            #[cfg(FALSE)]
            for type_ in possible_types {
                for attr_type in schema.owns.get(&type_).unwrap_or(&BTreeSet::new()) {
                    refinements.push(self.extend_with_has(var, attr_type, schema));
                }
            }
            #[cfg(FALSE)]
            // Attribute ownerships by themselves will not do much unless we refine them a bit
            // But for now we keep it simple.
            for (var, var_types) in &self.types_ {
                // This isn't perfect
                schema.categorical_attribute_values.iter().filter(|(t, _)| var_types.contains(t)).for_each(
                    |(t, values)| {
                        values.iter().for_each(|value| {
                            refinements.push(self.extend_with_eq(var, value));
                        });
                    },
                );
            }

            for type_ in possible_types {
                for attr_type in schema.owns.get(&type_).unwrap_or(&BTreeSet::new()) {
                    if let Some(values) = schema.categorical_attribute_values.get(attr_type) {
                        values.iter().for_each(|value| {
                            refinements.push(self.extend_with_has_value(var, attr_type, value, schema));
                        })
                    }
                }
            }

            // Relations we relate
            for type_ in possible_types {
                for role_type in schema.relates.get(type_).unwrap_or(&BTreeSet::new()) {
                    refinements.push(self.extend_with_related_links(&var, role_type, schema));
                }
                // TODO: Find types relating role, and see if existing vars can be used.
            }

            // Relations we play roles in
            for type_ in possible_types {
                for role_type in schema.plays.get(type_).unwrap_or(&BTreeSet::new()) {
                    let refined = self.extend_with_played_links(&var, role_type, schema);
                    refinements.push(refined.clone());

                    {   // TODO: Decide if we want to keep this.
                        let last = refined.conjunction.last().unwrap().clone();
                        let ClauseLiteral::Links { relation: rel_var, .. } = last else { unreachable!() };
                        for rel_type_ in refined.types_.get(&rel_var).unwrap() {
                            for other_role in schema.relates.get(rel_type_).unwrap_or(&BTreeSet::new()) {
                                refinements.push(refined.extend_with_related_links(&rel_var, other_role, schema));
                            }
                        }
                    }
                }
                // TODO: Find types playing role, and see if existing vars can be used.
            }
        }

        #[cfg(FALSE)]
        for i in 0..existing_vars.len() {
            for j in (i + 1)..existing_vars.len() {
                let var1 = &existing_vars[i];
                let var2 = &existing_vars[j];
                if let (Some(types1), Some(types2)) = (self.types_.get(var1), self.types_.get(var2)) {
                    if types1.intersection(types2).count() > 0 {
                        for comparator in ValueComparator::VALUES {
                            refinements.push(self.extend_with_comparison(var1, comparator, var2, schema));
                        }
                    }
                }
            }
        }
        // TODO: Add comparison against value

        refinements
    }

    pub(crate) fn extend_with_isa(&self, var: &ClauseVariable, type_: &SchemaType, schema: &Schema) -> Clause {
        let mut new_clause = self.clone();
        new_clause.conjunction.push(ClauseLiteral::Isa { instance: var.clone(), type_: type_.clone() });

        // Narrow the types for this variable
        let new_types = schema.subtypes.get(&type_).unwrap().clone();
        // TODO: Do I have to add subtypes?
        new_clause.update_types(&var, new_types);
        new_clause
    }

    pub(crate) fn extend_with_has(&self, owner: &ClauseVariable, attr_type: &SchemaType, schema: &Schema) -> Clause {
        let mut new_clause = self.clone();
        let attr_var = self.fresh_variable(attr_type, None);
        new_clause.conjunction.push(ClauseLiteral::Has {
            owner: owner.clone(),
            type_: attr_type.clone(),
            attribute: attr_var.clone(),
        });

        // Add the attribute variable to types
        // TODO: Do I have to add subtypes?
        let attr_types = BTreeSet::from([attr_type.clone()]);
        new_clause.update_types(&attr_var, attr_types);
        let owner_types = schema.owners.get(&attr_type).unwrap().clone();
        new_clause.update_types(&owner, owner_types);
        new_clause
    }

    fn extend_with_has_value(
        &self,
        owner: &ClauseVariable,
        attr_type: &SchemaType,
        value: &typedb_driver::concept::value::Value,
        schema: &Schema,
    ) -> Clause {
        let mut new_clause = self.clone();
        new_clause.conjunction.push(ClauseLiteral::HasValue {
            owner: owner.clone(),
            type_: attr_type.clone(),
            value: value.clone(),
        });
        let owner_types = schema.owners.get(&attr_type).unwrap().clone();
        new_clause.update_types(&owner, owner_types);
        new_clause
    }

    pub(crate) fn extend_with_eq(
        &self,
        attr_var: &ClauseVariable,
        value: &typedb_driver::concept::value::Value,
    ) -> Clause {
        let mut new_clause = self.clone();
        new_clause.conjunction.push(ClauseLiteral::CompareConstant {
            lhs: attr_var.clone(),
            comparator: ValueComparator::Eq,
            rhs: value.clone(),
        });
        new_clause
    }

    pub(crate) fn extend_with_played_links(
        &self,
        player: &ClauseVariable,
        role_type: &SchemaType,
        schema: &Schema,
    ) -> Clause {
        let mut new_clause = self.clone();
        let relation = self.fresh_variable(role_type, Some("rel"));
        new_clause.conjunction.push(ClauseLiteral::Links {
            player: player.clone(),
            role: role_type.clone(),
            relation: relation.clone(),
        });
        new_clause.update_types(&relation, schema.related_by[role_type].clone());
        // TODO: Do we have to update player types?
        new_clause
    }

    pub(crate) fn extend_with_related_links(
        &self,
        relation: &ClauseVariable,
        role_type: &SchemaType,
        schema: &Schema,
    ) -> Clause {
        let mut new_clause = self.clone();
        let player = self.fresh_variable(role_type, None);
        new_clause.conjunction.push(ClauseLiteral::Links {
            relation: relation.clone(),
            role: role_type.clone(),
            player: player.clone(),
        });
        // TODO: Do we have to update relation types?
        new_clause.update_types(&player, schema.players[role_type].clone());
        new_clause
    }

    pub(crate) fn extend_with_comparison(
        &self,
        var1: &ClauseVariable,
        comparator: ValueComparator,
        var2: &ClauseVariable,
    ) -> Clause {
        let mut new_clause = self.clone();
        new_clause.conjunction.push(ClauseLiteral::CompareVariables {
            lhs: var1.clone(),
            comparator,
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

    pub fn to_typeql(&self) -> String {
        self.conjunction.iter().map(|literal| literal.to_typeql()).join(";\n")
    }

    pub fn fmt_with_indent(&self, f: &mut Formatter<'_>, depth: usize) -> std::fmt::Result {
        let indent = INDENT.repeat(depth);
        let newline_indent = format!("\n{}", indent);
        writeln!(f, "{}{}", indent, self.to_typeql().replace("\n", newline_indent.as_str()))
    }

    fn update_types(&mut self, var: &ClauseVariable, types_: BTreeSet<SchemaType>) {
        if let Some(existing) = self.types_.get_mut(var) {
            existing.retain(|t| types_.contains(t));
        } else {
            self.types_.insert(var.clone(), types_);
        }
    }
}

impl std::fmt::Display for Clause {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_typeql().as_str())
    }
}
