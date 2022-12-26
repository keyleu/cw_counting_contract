use thiserror::Error;
use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized - only {owner} can call it")]
    Unauthorized { owner: String },

    #[error("Migrating invalid contract: {0}")]
    InvalidName(String),

    #[error("Migrating from unsupported version: {0}")]
    InvalidVersion(String)
}