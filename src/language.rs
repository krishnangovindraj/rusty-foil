use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    fmt::Formatter,
    hash::{Hash, Hasher},
};

use typedb_driver::{
    Promise, Transaction, TypeDBDriver,
    concept::{Concept, type_::Type},
};

use crate::TypeDBHelper;

pub enum LanguageDiscoveryOption {
    CategoricalAttributes { type_labels: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct HypothesisLanguage {
    pub schema: Schema,
}

impl HypothesisLanguage {
    const OWNS_QUERY: &'static str = "match $left owns $right;";
    const RELATES_QUERY: &'static str = "match $left relates $right;";
    const PLAYS_QUERY: &'static str = "match $left plays $right;";
    const SUB_QUERY: &'static str = "match $left sub $right;";

    fn _exec(
        tx: &Transaction,
        query: &str,
    ) -> Result<impl Iterator<Item = Result<(Concept, Concept), typedb_driver::Error>>, typedb_driver::Error> {
        Ok(tx.query(query).resolve()?.into_rows().map(|result| {
            let concept_map = result?;
            let left = concept_map.get("left").unwrap().unwrap().clone();
            let right = concept_map.get("right").unwrap().unwrap().clone();
            Ok::<_, typedb_driver::Error>((left, right))
        }))
    }

    pub fn fetch_from_typedb(
        typedb: &TypeDBHelper,
        options: &[LanguageDiscoveryOption],
    ) -> Result<Self, typedb_driver::Error> {
        fn _collect_lr(
            mut iter: impl Iterator<Item = Result<(Concept, Concept), typedb_driver::Error>>,
        ) -> Result<
            (HashMap<SchemaType, BTreeSet<SchemaType>>, HashMap<SchemaType, BTreeSet<SchemaType>>),
            typedb_driver::Error,
        > {
            let mut lr = HashMap::new();
            let mut rl = HashMap::new();
            iter.try_for_each(|result| {
                let (left, right) = result?;
                lr.entry(left.clone().into()).or_insert_with(BTreeSet::new).insert(right.clone().into());
                rl.entry(right.clone().into()).or_insert_with(BTreeSet::new).insert(left.clone().into());
                Ok::<_, typedb_driver::Error>(())
            })?;
            Ok((lr, rl))
        }

        let schema = {
            let tx = typedb.driver.transaction(&typedb.database, typedb_driver::TransactionType::Read).unwrap();
            let (owns, owners) = _collect_lr(Self::_exec(&tx, Self::OWNS_QUERY)?)?;
            let (relates, related_by) = _collect_lr(Self::_exec(&tx, Self::RELATES_QUERY)?)?;
            let (plays, players) = _collect_lr(Self::_exec(&tx, Self::PLAYS_QUERY)?)?;
            let (_, subtypes) = _collect_lr(Self::_exec(&tx, Self::SUB_QUERY)?)?;
            let categorical_attribute_values = Self::read_categorical_attribute_values(&tx, options)?;
            Schema { owns, owners, relates, related_by, plays, players, subtypes, categorical_attribute_values }
        };

        Ok(Self { schema })
    }

    pub(crate) fn lookup_type(&self, label: &str) -> Option<SchemaType> {
        self.schema.subtypes.keys().find(|t| t.label() == label).cloned()
    }

    fn read_categorical_attribute_values(
        tx: &Transaction,
        options: &[LanguageDiscoveryOption],
    ) -> Result<HashMap<SchemaType, Vec<typedb_driver::concept::value::Value>>, typedb_driver::Error> {
        let mut categorical_attribute_values = HashMap::new();
        options
            .iter()
            .filter_map(|option| {
                match option {
                    LanguageDiscoveryOption::CategoricalAttributes { type_labels } => Some(type_labels),
                    // _ => None,
                }
            })
            .flat_map(|labels| labels.iter())
            .try_for_each(|label| {
                Self::_exec(&tx, format!("match attribute $left label {};  $right isa $left;", label).as_str())?
                    .try_for_each(|result| {
                        let (type_, attr) = result?;
                        categorical_attribute_values
                            .entry(type_.clone().into())
                            .or_insert_with(Vec::new)
                            .push(attr.try_get_value().unwrap().clone());
                        Ok::<_, typedb_driver::Error>(())
                    })
            })?;
        Ok(categorical_attribute_values)
    }
}

#[derive(Debug, Clone)]
pub struct Schema {
    // Entity/Relation types that own attribute types
    pub owns: HashMap<SchemaType, BTreeSet<SchemaType>>,
    pub owners: HashMap<SchemaType, BTreeSet<SchemaType>>,

    // Relation types and their role types
    pub relates: HashMap<SchemaType, BTreeSet<SchemaType>>,
    pub related_by: HashMap<SchemaType, BTreeSet<SchemaType>>,

    // Entity/Relation types and which roles they can play
    pub plays: HashMap<SchemaType, BTreeSet<SchemaType>>,
    pub players: HashMap<SchemaType, BTreeSet<SchemaType>>,

    pub subtypes: HashMap<SchemaType, BTreeSet<SchemaType>>,
    pub categorical_attribute_values: HashMap<SchemaType, Vec<typedb_driver::concept::value::Value>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaType(typedb_driver::concept::type_::Type);

impl SchemaType {
    pub fn label(&self) -> &str {
        self.0.label()
    }
}

impl Hash for SchemaType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.0.label().as_bytes())
    }
}

impl Eq for SchemaType {}

impl PartialOrd<Self> for SchemaType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SchemaType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.label().cmp(other.0.label())
    }
}

impl From<typedb_driver::concept::Concept> for SchemaType {
    fn from(value: Concept) -> Self {
        match value {
            Concept::EntityType(type_) => SchemaType(Type::EntityType(type_)),
            Concept::RelationType(type_) => SchemaType(Type::RelationType(type_)),
            Concept::RoleType(type_) => SchemaType(Type::RoleType(type_)),
            Concept::AttributeType(type_) => SchemaType(Type::AttributeType(type_)),
            _ => unreachable!("Expected type"),
        }
    }
}

impl std::fmt::Display for SchemaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
