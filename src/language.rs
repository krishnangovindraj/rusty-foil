use std::collections::HashMap;
use typedb_driver::{TypeDBDriver, Promise, Transaction};


#[derive(Debug, Clone)]
pub struct HypothesisLanguage {
    pub schema: Schema,
}

impl HypothesisLanguage {
    const OWNS_QUERY: &'static str = "match $left owns $right;";
    const RELATES_QUERY: &'static str = "match $left relates $right;";
    const PLAYS_QUERY: &'static str = "match $left plays $right;";
    pub fn fetch_from_typedb(
        driver: &TypeDBDriver,
        database_name: &str,
    ) -> Result<Self, typedb_driver::Error> {
        fn _populate(
            tx: &Transaction, query: &str
        ) -> Result<(HashMap<String, Vec<String>>, HashMap<String, Vec<String>>), typedb_driver::Error> {
            let mut lr = HashMap::new();
            let mut rl = HashMap::new();
            tx.query(query).resolve()?.into_rows()
                .try_for_each(|result| {
                    let concept_map = result?;
                    let left = concept_map.get("left").unwrap().unwrap().get_label().to_owned();
                    let right = concept_map.get("right").unwrap().unwrap().get_label().to_owned();
                    lr.entry(left.clone()).or_insert_with(Vec::new).push(right.clone());
                    rl.entry(right).or_insert_with(Vec::new).push(left);
                    Ok::<_, typedb_driver::Error>(())
                })?;
            Ok((lr, rl))
        }

        let schema = {
            let tx = driver.transaction(database_name, typedb_driver::TransactionType::Read).unwrap();
            let (owns, owners) = _populate(&tx, Self::OWNS_QUERY)?;
            let (relates, related_by) = _populate(&tx, Self::RELATES_QUERY)?;
            let (plays, players) = _populate(&tx, Self::PLAYS_QUERY)?;
            Schema {
                owns,
                owners,
                relates,
                related_by,
                plays,
                players,
            }
        };

        Ok(Self { schema })
    }
}

#[derive(Debug, Clone)]
pub struct Schema {
    // Entity/Relation types that own attribute types
    pub owns: HashMap<String, Vec<String>>,
    pub owners: HashMap<String, Vec<String>>,

    // Relation types and their role types
    pub relates: HashMap<String, Vec<String>>,
    pub related_by: HashMap<String, Vec<String>>,

    // Entity/Relation types and which roles they can play
    pub plays: HashMap<String, Vec<String>>,
    pub players: HashMap<String, Vec<String>>,
}
