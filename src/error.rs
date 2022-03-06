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

    #[error("Wrong set of balls or not allowed duplicated numbers")]
    WrongSetOfBallsOrDuplicateNotAllowed {},

    #[error("Error determining the tier you are playing")]
    ErrorTierDetermination {},

    #[error("Register will be open soon")]
    RegisterClosed {},

    #[error("Bonus number is out of range")]
    BonusOutOfRange {},

    #[error("Lottery still in progress")]
    LotteryInProgress {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
