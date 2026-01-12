use std::collections::{BTreeSet, HashMap};

use rusty_foil::{clause::{Clause, ClauseVariable}, language::SchemaType, TypeDBHelper};
use typedb_driver::{Credentials, DriverOptions, Promise, TypeDBDriver};

const TEST_DATABASE: &str = "rusty_foil_integration_tests";
const TYPEDB_ADDRESS: &str = "localhost:1729";

// Test schema definition
const TEST_SCHEMA: &str = r#"
define

# Entity types
entity person,
    owns name,
    owns age,
    plays employment:employee,
    plays parenthood:parent,
    plays parenthood:child;

entity company,
    owns company-name,
    plays employment:employer;

# Attribute types
attribute name, value string;
attribute age, value integer;
attribute company-name, sub name;

# Relation types
relation parenthood,
    relates parent,
    relates child;

relation employment,
    relates employer,
    relates employee;
"#;

// Helper function to setup test database
fn setup_test_database(driver: &TypeDBDriver) -> Result<(), typedb_driver::Error> {
    // Delete database if it exists
    if driver.databases().contains(TEST_DATABASE)? {
        driver.databases().get(TEST_DATABASE)?.delete()?;
    }

    // Create new database
    driver.databases().create(TEST_DATABASE)?;

    // Define schema
    let tx = driver.transaction(TEST_DATABASE, typedb_driver::TransactionType::Schema).unwrap();
    tx.query(TEST_SCHEMA).resolve()?;
    tx.commit().resolve()?;

    Ok(())
}

// Helper function to cleanup test database
fn cleanup_test_database(driver: &TypeDBDriver) -> Result<(), Box<dyn std::error::Error>> {
    if driver.databases().contains(TEST_DATABASE)? {
        driver.databases().get(TEST_DATABASE)?.delete()?;
    }
    Ok(())
}

#[test]
fn test_fetch_schema_from_typedb() -> Result<(), Box<dyn std::error::Error>> {
    let driver = TypeDBDriver::new(
        TYPEDB_ADDRESS,
        Credentials::new("admin", "password"),
        DriverOptions::new(false, None).unwrap(),
    )?;

    setup_test_database(&driver)?;

    let typedb = TypeDBHelper::new(driver, TEST_DATABASE.to_owned());
    let language = rusty_foil::language::HypothesisLanguage::fetch_from_typedb(&typedb, &[])?;

    fn _contains(set: &HashMap<SchemaType, BTreeSet<SchemaType>>, key: &str, value: &str) -> bool {
        let key_type = set.keys().find(|k| k.label() == key).unwrap();
        set[key_type].iter().find(|v| v.label() == value).is_some()
    }

    assert!(_contains(&language.schema.owns, "person", "name"));
    assert!(_contains(&language.schema.owns, "person", "age"));
    assert!(_contains(&language.schema.relates, "parenthood", "parenthood:parent"));
    assert!(_contains(&language.schema.related_by, "parenthood:child", "parenthood"));
    assert!(_contains(&language.schema.plays, "person", "parenthood:parent"));
    assert!(_contains(&language.schema.players, "parenthood:child", "person"));
    cleanup_test_database(&driver)?;

    Ok(())
}

#[test]
fn test_refinement() -> Result<(), Box<dyn std::error::Error>> {
    let driver = TypeDBDriver::new(
        TYPEDB_ADDRESS,
        Credentials::new("admin", "password"),
        DriverOptions::new(false, None).unwrap(),
    )?;

    setup_test_database(&driver)?;
    let typedb = TypeDBHelper::new(driver, TEST_DATABASE.to_owned());
    let language = rusty_foil::language::HypothesisLanguage::fetch_from_typedb(&typedb, &[])?;
    let person_type = language.schema.owns.keys().find(|x| x.label() == "person").unwrap();

    let start = Clause::new_from_isa(person_type.clone(), &language.schema);
    let refined = start.refine(&language);
    for r in &refined {
        println!("{}", r);
    }
    assert_eq!(refined.len(), language.schema.plays[person_type].len()); // This changes as you change the schema. You need to do the walk

    Ok(())
}
