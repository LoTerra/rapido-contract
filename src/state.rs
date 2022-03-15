use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub denom: String,
    pub frequency: u64,
    pub fee_collector: Decimal,
    pub fee_collector_address: CanonicalAddr,
    pub fee_collector_terrand: Decimal,
    pub terrand_address: CanonicalAddr,
    pub live_round_max: u16,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BallsRange {
    pub min: u8,
    pub max: u8,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub round: u64,
    pub set_of_balls: u8,
    pub range: BallsRange,
    pub bonus_set_of_balls: u8,
    pub bonus_range: BallsRange,
    pub prize_rank: Vec<Uint128>,
    pub ticket_price: Vec<Uint128>,
    pub multiplier: Vec<Decimal>,
}
pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LotteryState {
    pub draw_time: u64,
    pub terrand_round: u64,
    pub terrand_worker: Option<CanonicalAddr>,
    pub prize_rank: Vec<Uint128>,
    pub ticket_price: Vec<Uint128>,
    pub multiplier: Vec<Decimal>,
    pub winning_number: Option<Vec<u8>>,
    pub bonus_number: Option<u8>,
}
pub const LOTTERY_STATE: Map<&[u8], LotteryState> = Map::new("lottery_state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LotteryStats {
    pub counter_player: Option<u64>,
    pub total_ticket_sold: Option<u64>,
    pub total_collected: Option<Uint128>,
}
pub const LOTTERY_STATS: Map<&[u8], LotteryStats> = Map::new("lottery_stats");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Game {
    pub number: Vec<u8>,
    pub bonus: u8,
    pub multiplier: Decimal,
    pub resolved: bool,
}
#[allow(clippy::type_complexity)]
pub const GAMES: Map<(&[u8], &[u8], &[u8]), Game> = Map::new("games");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameStats {
    pub total_ticket: u64,
    pub total_spent: Uint128,
}
pub const GAMES_STATS: Map<(&[u8], &[u8]), GameStats> = Map::new("games_stats");
