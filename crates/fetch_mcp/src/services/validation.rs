use crate::errors::FetchServerError;

pub trait Validate {
    fn validate(&self) -> Result<(), FetchServerError>;
}
