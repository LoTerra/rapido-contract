use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, CanonicalAddr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub denom: String,
    pub frequency: u64,
    pub fee_collector: Decimal,
    pub fee_collector_address: CanonicalAddr,
    pub fee_collector_drand: Decimal,
    pub fee_collector_drand_address: CanonicalAddr,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BallsRange {
    pub min: u8,
    pub max: u8,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub draw_time: u64,
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
pub struct Lottery {
    pub jackpot: Uint128,
    pub counter_player: u64,
}
pub const LOTTERY: Map<&[u8], Lottery> = Map::new("lottery");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Game {
    pub number: Vec<u8>,
    pub bonus: u8,
    pub multiplier: Decimal,
    pub resolved: bool,
}
pub const GAMES: Map<(&[u8], &[u8], &[u8]), Game> = Map::new("games");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GameStats {
    pub total_ticket: u64,
    pub total_spent: Uint128,
}
pub const GAMES_STATS: Map<(&[u8], &[u8]), GameStats> = Map::new("games_stats");
