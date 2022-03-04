use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Game credits required")]
    EmptyFunds {},

    #[error("This native token is not accepted")]
    WrongDenom {},

    #[error("Multiple tokens are not accepted")]
    MultipleDenoms {},

    #[error("Set of balls need to be {0}")]
    WrongSetOfBalls(u8),

    #[error("Error determining the tier you are playing")]
    ErrorTierDetermination {},

    #[error("Duplicating numbers are not allowed")]
    DuplicateNotAllowed {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
