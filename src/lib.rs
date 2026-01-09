use typedb_driver::concept::Concept;

pub mod clause;
pub mod language;
pub mod foil;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Instance(typedb_driver::IID);

impl From<&Concept> for Instance {
    fn from(value: &Concept) -> Self {
        Self(value.try_get_iid().expect("Expected instance variant which has IID").clone())
    }
}
