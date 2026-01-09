use std::{io::Read, path::Path};

use typedb_driver::{Credentials, DriverOptions, Promise, TypeDBDriver};
use rusty_foil::foil::FoilLearningTask;
use rusty_foil::language::HypothesisLanguage;
use rusty_foil::tilde::TypeDBHelper;

const TYPEDB_ADDRESS: &str = "localhost:1729";

// Helper function to setup test database
fn setup_test_database(
    driver: &TypeDBDriver,
    database: &str,
    schema: &Path,
    data: &Path,
) -> Result<(), typedb_driver::Error> {
    // Delete database if it exists
    if driver.databases().contains(database)? {
        driver.databases().get(database)?.delete()?;
    }

    // Create new database
    driver.databases().create(database)?;

    // Define schema
    {
        let tx = driver.transaction(database, typedb_driver::TransactionType::Schema).unwrap();
        tx.query(std::fs::read_to_string(schema).unwrap()).resolve()?;
        tx.commit().resolve()?;
    }

    {
        let tx = driver.transaction(database, typedb_driver::TransactionType::Write).unwrap();
        tx.query(std::fs::read_to_string(data).unwrap()).resolve()?;
        tx.commit().resolve()?;
    }

    Ok(())
}
fn cleanup_test_database(driver: &TypeDBDriver, database: &str) -> Result<(), Box<dyn std::error::Error>> {
    if driver.databases().contains(database)? {
        driver.databases().get(database)?.delete()?;
    }
    Ok(())
}

#[test]
fn test_bongard_foil() -> Result<(), Box<dyn std::error::Error>> {
    let db_name = "bongard_foil";
    let target_type_label = "bongard-problem";
    let class_label = "class";
    let driver = TypeDBDriver::new(
        TYPEDB_ADDRESS,
        Credentials::new("admin", "password"),
        DriverOptions::new(false, None).unwrap(),
    )?;

    setup_test_database(
        &driver,
        db_name,
        Path::new("examples/bongard/schema.tql"),
        Path::new("examples/bongard/data.tql"),
    )?;
    let typedb = TypeDBHelper::new(driver, db_name.to_owned());
    let language = HypothesisLanguage::fetch_from_typedb(&typedb)?;
    let task = FoilLearningTask::discover(typedb, language, target_type_label.to_owned(), class_label.to_owned())?;

    let clauses = task.search()?;
    println!("Found {} clauses", clauses.len());

    let driver = task.deconstruct();
    if let Err(_) = cleanup_test_database(&driver.driver, db_name) {
        eprintln!("Cleanup failed");
    }
    Ok(())
}
