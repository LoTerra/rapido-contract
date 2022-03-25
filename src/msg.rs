use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub denom: String,
    pub frequency: u64,
    pub fee_collector: Decimal,
    pub fee_collector_address: String,
    pub fee_collector_terrand: Decimal,
    pub terrand_address: String,
    pub set_of_balls: u8,
    pub range_min: u8,
    pub range_max: u8,
    pub bonus_set_of_balls: u8,
    pub bonus_range_min: u8,
    pub bonus_range_max: u8,
    pub prize_rank: Vec<Uint128>,
    pub ticket_price: Vec<Uint128>,
    pub multiplier: Vec<Decimal>,
    pub live_round_max: u16,
    pub burn_rate: Decimal
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Register {
        numbers: Vec<u8>,
        multiplier: Uint128,
        live_round: u16,
        address: Option<String>,
    },
    Draw {},
    Collect {
        round: u64,
        player: String,
        game_id: Vec<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the config
    Config {},
    /// Get the state
    State {},
    /// Get all games from player at round x
    Games {
        start_after: Option<u64>,
        limit: Option<u32>,
        round: u64,
        player: String,
    },
    /// Query lottery
    LotteryState { round: u64 },
    /// Query all lotteries
    LotteriesState {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Query game stats by address
    GameStats {
        player: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Query lottery stats by round
    LotteryStats { round: u64 },
    // /// Get a game from player
    // GameStats {round: u64, player: String}
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub denom: String,
    pub frequency: u64,
    pub fee_collector: Decimal,
    pub fee_collector_address: String,
    pub fee_collector_terrand: Decimal,
    pub fee_collector_terrand_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameResponse {
    pub number: Vec<u8>,
    pub bonus: u8,
    pub multiplier: Decimal,
    pub resolved: bool,
    pub game_id: u64,
    pub lottery_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LotteryResponse {
    pub draw_time: u64,
    pub terrand_round: u64,
    pub terrand_worker: Option<String>,
    pub prize_rank: Vec<Uint128>,
    pub ticket_price: Vec<Uint128>,
    pub multiplier: Vec<Decimal>,
    pub winning_number: Option<Vec<u8>>,
    pub bonus_number: Option<u8>,
    pub lottery_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameStatsResponse {
    pub total_ticket: u64,
    pub total_spent: Uint128,
    pub game_stats_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LotteryStatsResponse {
    pub counter_player: Option<u64>,
    pub total_ticket_sold: Option<u64>,
    pub total_collected: Option<Uint128>,
    pub lottery_stats_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
