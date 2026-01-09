use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use typedb_driver::{TypeDBDriver, Promise, Transaction};
use typedb_driver::concept::Concept;
use typedb_driver::concept::type_::Type;

#[derive(Debug, Clone)]
pub struct HypothesisLanguage {
    pub schema: Schema,
}

impl HypothesisLanguage {
    const OWNS_QUERY: &'static str = "match $left owns $right;";
    const RELATES_QUERY: &'static str = "match $left relates $right;";
    const PLAYS_QUERY: &'static str = "match $left plays $right;";
    const SUB_QUERY: &'static str = "match $left sub $right;";

    pub fn fetch_from_typedb(
        driver: &TypeDBDriver,
        database_name: &str,
    ) -> Result<Self, typedb_driver::Error> {
        fn _populate(
            tx: &Transaction, query: &str
        ) -> Result<(HashMap<SchemaType, BTreeSet<SchemaType>>, HashMap<SchemaType, BTreeSet<SchemaType>>), typedb_driver::Error> {
            let mut lr = HashMap::new();
            let mut rl = HashMap::new();
            tx.query(query).resolve()?.into_rows()
                .try_for_each(|result| {
                    let concept_map = result?;
                    let left: SchemaType = concept_map.get("left").unwrap().unwrap().clone().into();
                    let right: SchemaType = concept_map.get("right").unwrap().unwrap().clone().into();
                    lr.entry(left.clone()).or_insert_with(BTreeSet::new).insert(right.clone());
                    rl.entry(right.clone()).or_insert_with(BTreeSet::new).insert(left.clone());
                    Ok::<_, typedb_driver::Error>(())
                })?;
            Ok((lr, rl))
        }

        let schema = {
            let tx = driver.transaction(database_name, typedb_driver::TransactionType::Read).unwrap();
            let (owns, owners) = _populate(&tx, Self::OWNS_QUERY)?;
            let (relates, related_by) = _populate(&tx, Self::RELATES_QUERY)?;
            let (plays, players) = _populate(&tx, Self::PLAYS_QUERY)?;
            let (_, subtypes) = _populate(&tx, Self::SUB_QUERY)?;
            Schema {
                owns,
                owners,
                relates,
                related_by,
                plays,
                players,
                subtypes,
            }
        };

        Ok(Self { schema })
    }

    pub(crate) fn lookup_type(&self, label: &str) -> Option<SchemaType> {
        self.schema.subtypes.keys().find(|t| t.label() == label).cloned()
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

impl Eq for SchemaType { }

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
            _ => unreachable!("Expected type")
        }
    }
}

impl std::fmt::Display for SchemaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}