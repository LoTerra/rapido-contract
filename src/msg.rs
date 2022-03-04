use crate::state::BallsRange;
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub denom: String,
    pub start_time: u64,
    pub frequency: u64,
    pub fee_collector: Decimal,
    pub fee_collector_address: String,
    pub fee_collector_drand: Decimal,
    pub fee_collector_drand_address: String,
    pub set_of_balls: u8,
    pub range_min: u8,
    pub range_max: u8,
    pub bonus_set_of_balls: u8,
    pub bonus_range_min: u8,
    pub bonus_range_max: u8,
    pub prize_rank: Vec<Uint128>,
    pub ticket_price: Vec<Uint128>,
    pub multiplier: Vec<Decimal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Register {
        numbers: Vec<Vec<u8>>,
        bonus: u8,
        address: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the config
    Config {},
    /// Get the state
    State {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub denom: String,
    pub frequency: u64,
    pub fee_collector: Decimal,
    pub fee_collector_address: String,
    pub fee_collector_drand: Decimal,
    pub fee_collector_drand_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub start_time: u64,
    pub round: u64,
    pub set_of_balls: u8,
    pub range_min: u8,
    pub range_max: u8,
    pub bonus_set_of_balls: u8,
    pub bonus_range_min: u8,
    pub bonus_range_max: u8,
    pub prize_rank: Vec<Uint128>,
    pub ticket_price: Vec<Uint128>,
    pub multiplier: Vec<Decimal>,
}
