#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
    Uint128, WasmQuery,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use std::cmp::Ordering;
use std::convert::TryInto;

use crate::error::ContractError;
use crate::helpers::{bonus_number, is_lower_hex, save_game, winning_number};
use crate::msg::{
    ConfigResponse, ExecuteMsg, GameResponse, InstantiateMsg, QueryMsg, StateResponse,
};
use crate::state::{
    BallsRange, Config, Game, GameStats, LotteryState, State, CONFIG, GAMES, GAMES_STATS,
    LOTTERY_STATE, STATE,
};
use terrand;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:loterra-v2.0";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DRAND_GENESIS_TIME: u64 = 1595431050;
const DRAND_PERIOD: u64 = 30;
const DRAND_NEXT_ROUND_SECURITY: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        denom: msg.denom,
        frequency: msg.frequency,
        fee_collector: msg.fee_collector,
        fee_collector_address: deps.api.addr_canonicalize(&msg.fee_collector_address)?,
        fee_collector_drand: msg.fee_collector_drand,
        drand_address: deps.api.addr_canonicalize(&msg.drand_address)?,
    };

    let state = State {
        round: 0,
        set_of_balls: msg.set_of_balls,
        range: BallsRange {
            min: msg.range_min,
            max: msg.range_max,
        },
        bonus_set_of_balls: msg.bonus_set_of_balls,
        bonus_range: BallsRange {
            min: msg.bonus_range_min,
            max: msg.bonus_range_max,
        },
        prize_rank: msg.prize_rank.clone(),
        ticket_price: msg.ticket_price.clone(),
        multiplier: msg.multiplier.clone(),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    CONFIG.save(deps.storage, &config)?;

    // calculate next round randomness from now
    let draw_time = env.block.time.plus_seconds(config.frequency).seconds();
    let from_genesis = draw_time - DRAND_GENESIS_TIME;
    let next_round = (from_genesis / DRAND_PERIOD) + DRAND_NEXT_ROUND_SECURITY;

    LOTTERY_STATE.save(
        deps.storage,
        &state.round.to_be_bytes(),
        &LotteryState {
            draw_time,
            terrand_round: next_round,
            prize_rank: msg.prize_rank,
            ticket_price: msg.ticket_price,
            counter_player: None,
            multiplier: msg.multiplier,
            winning_number: None,
            bonus_number: None,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Register { numbers, address } => {
            try_register(deps, env, info, numbers, address)
        }
        ExecuteMsg::Draw {} => try_draw(deps, env, info),
    }
}

pub fn try_register(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    numbers: Vec<Vec<u8>>,
    address: Option<String>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let lottery = LOTTERY_STATE.load(deps.storage, &state.round.to_be_bytes())?;

    if lottery.draw_time < env.block.time.seconds() {
        return Err(ContractError::RegisterClosed {});
    }

    let sent = match info.funds.len() {
        0 => Err(ContractError::EmptyFunds {}),
        1 => {
            if info.funds[0].denom != config.denom {
                return Err(ContractError::WrongDenom {});
            }
            Ok(info.funds[0].amount)
        }
        _ => Err(ContractError::MultipleDenoms {}),
    }?;

    let address_raw = match address {
        None => deps.api.addr_canonicalize(&info.sender.as_str())?,
        Some(address) => deps.api.addr_canonicalize(&address)?,
    };

    let tiers = lottery
        .ticket_price
        .into_iter()
        .filter(|tier| sent.u128() == tier.u128() * numbers.len() as u128)
        .collect::<Vec<Uint128>>();

    // if tiers.is_empty() || tiers.len() > {
    //     return Err(ContractError::ErrorTierDetermination{})
    // };
    //let multiplier = Decimal::from_ratio(tiers[0], Uint128::from(1_000_000u128));

    // Get the multiplier
    let multiplier = match u128::from(tiers[0]) {
        1_000_000 => lottery.multiplier[0],
        2_000_000 => lottery.multiplier[1],
        5_000_000 => lottery.multiplier[2],
        _ => {
            return Err(ContractError::ErrorTierDetermination {});
        }
    };

    let mut new_number = vec![];
    // Check if duplicate numbers
    for mut number in numbers.clone() {
        let mut new_arr = number.clone();
        let bonus_number = number.last().unwrap();
        // Handle the bonus number is in the range
        if bonus_number > &state.bonus_range.max || bonus_number < &state.bonus_range.min {
            return Err(ContractError::BonusOutOfRange {});
        }

        new_arr.retain(|&x| &x != bonus_number);
        new_arr.sort();
        new_arr.dedup();

        if new_arr.len() as u8 != state.set_of_balls {
            return Err(ContractError::WrongSetOfBallsOrDuplicateNotAllowed {});
        }

        new_number.push(new_arr);
    }

    match GAMES_STATS.may_load(
        deps.storage,
        (&state.round.to_be_bytes(), &address_raw.as_slice()),
    )? {
        None => {
            save_game(
                deps.storage,
                state.round,
                &address_raw,
                numbers.clone(),
                multiplier,
                None,
            )?;
            GAMES_STATS.save(
                deps.storage,
                (&state.round.to_be_bytes(), &address_raw.as_slice()),
                &GameStats {
                    total_ticket: numbers.len() as u64,
                    total_spent: sent,
                },
            )?;
        }
        Some(game_stats) => {
            save_game(
                deps.storage,
                state.round,
                &address_raw,
                numbers.clone(),
                multiplier,
                Some(game_stats),
            )?;

            GAMES_STATS.update(
                deps.storage,
                (&state.round.to_be_bytes(), &address_raw.as_slice()),
                |game_stats| -> Result<_, ContractError> {
                    let mut update_game_stats = game_stats.unwrap();
                    update_game_stats.total_spent =
                        update_game_stats.total_spent.checked_add(sent).unwrap();
                    update_game_stats.total_ticket += numbers.len() as u64;
                    Ok(update_game_stats)
                },
            )?;
        }
    }

    Ok(Response::new()
        .add_attribute("method", "try_register")
        .add_attribute("round", state.round.to_string())
        .add_attribute("ticket_amount", numbers.len().to_string())
        .add_attribute("sender", info.sender)
        .add_attribute(
            "recipient",
            deps.api.addr_humanize(&address_raw)?.to_string(),
        ))
}

pub fn try_draw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let lottery = LOTTERY_STATE.load(deps.storage, &state.round.to_be_bytes())?;

    if lottery.draw_time > env.block.time.seconds() {
        return Err(ContractError::Unauthorized {});
    }

    // Query terrand for the randomness
    let msg = terrand::msg::QueryMsg::GetRandomness {
        round: lottery.terrand_round,
    };
    let terrand_human = deps.api.addr_humanize(&config.drand_address)?;
    let query = WasmQuery::Smart {
        contract_addr: terrand_human.to_string(),
        msg: to_binary(&msg)?,
    };
    let terrand_randomness: terrand::msg::GetRandomResponse = deps.querier.query(&query.into())?;
    let randomness_hash: String = hex::encode(terrand_randomness.randomness.as_slice());

    let numbers: Vec<_> = randomness_hash.chars().collect();
    let winning_number = winning_number(numbers.clone())?;
    let bonus_number = bonus_number(numbers.last().unwrap())?;

    // Update lottery winning and bonus number
    LOTTERY_STATE.update(
        deps.storage,
        &state.round.to_be_bytes(),
        |lottery_state| -> Result<_, ContractError> {
            let mut update_lottery_state = lottery_state.unwrap();
            update_lottery_state.winning_number = Some(winning_number);
            update_lottery_state.bonus_number = Some(bonus_number);
            Ok(update_lottery_state)
        },
    )?;

    // Update state & save
    state.round += 1;
    STATE.save(deps.storage, &state)?;

    // calculate next round randomness from now
    let draw_time = env.block.time.plus_seconds(config.frequency).seconds();
    let from_genesis = draw_time - DRAND_GENESIS_TIME;
    let next_round = (from_genesis / DRAND_PERIOD) + DRAND_NEXT_ROUND_SECURITY;

    // Create new lottery
    LOTTERY_STATE.save(
        deps.storage,
        &state.round.to_be_bytes(),
        &LotteryState {
            draw_time,
            terrand_round: next_round,
            prize_rank: state.prize_rank,
            ticket_price: state.ticket_price,
            counter_player: None,
            multiplier: state.multiplier,
            winning_number: None,
            bonus_number: None,
        },
    )?;

    // println!("{:?}", bonus_number);
    // println!("{:?}", winning_number);
    // println!("{:?}", numbers);

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Games {
            start_after,
            limit,
            round,
            player,
        } => to_binary(&query_games(deps, start_after, limit, round, player)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        denom: config.denom,
        frequency: config.frequency,
        fee_collector: config.fee_collector,
        fee_collector_address: deps
            .api
            .addr_humanize(&config.fee_collector_address)?
            .to_string(),
        fee_collector_drand: config.fee_collector_drand,
        fee_collector_drand_address: deps.api.addr_humanize(&config.drand_address)?.to_string(),
    })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse {
        round: state.round,
        set_of_balls: state.set_of_balls,
        range_min: state.range.min,
        bonus_set_of_balls: state.bonus_set_of_balls,
        bonus_range_min: state.bonus_range.min,
        prize_rank: state.prize_rank,
        ticket_price: state.ticket_price,
        multiplier: state.multiplier,
        range_max: state.range.max,
        bonus_range_max: state.bonus_range.max,
    })
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;
fn query_games(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    round: u64,
    player: String,
) -> StdResult<Vec<GameResponse>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|d| Bound::Exclusive(d.to_be_bytes().to_vec()));

    let owner_addr = deps.api.addr_validate(&player)?;
    let raw_address = deps.api.addr_canonicalize(&owner_addr.as_str())?;
    let games = GAMES
        .prefix((&round.to_be_bytes(), raw_address.as_slice()))
        .range(deps.storage, None, start, Order::Descending)
        .take(limit)
        .map(|pair| {
            pair.and_then(|(k, game)| {
                Ok(GameResponse {
                    number: game.number,
                    bonus: game.bonus,
                    multiplier: game.multiplier,
                    resolved: game.resolved,
                    game_id: u64::from_be_bytes(k.try_into().unwrap()),
                })
            })
        })
        .collect::<StdResult<Vec<GameResponse>>>()?;

    Ok(games)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::custom_mock_dependencies;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Attribute, Coin, Decimal, Timestamp, Uint128};
    use std::str::FromStr;

    fn default_init(deps: DepsMut) {
        let msg = InstantiateMsg {
            denom: "uusd".to_string(),
            draw_time: 0,
            frequency: 300,
            fee_collector: Decimal::from_str("0.05").unwrap(),
            fee_collector_address: "STAKING".to_string(),
            fee_collector_drand: Decimal::from_str("0.01").unwrap(),
            drand_address: "TERRAND".to_string(),
            set_of_balls: 4,
            range_min: 1,
            range_max: 17,
            bonus_set_of_balls: 1,
            bonus_range_min: 1,
            bonus_range_max: 8,
            prize_rank: vec![
                Uint128::from(1_000_000u128),
                Uint128::from(5_000_000u128),
                Uint128::from(10_000_000u128),
                Uint128::from(30_000_000u128),
                Uint128::from(50_000_000u128),
                Uint128::from(150_000_000u128),
                Uint128::from(1_000_000_000u128),
                Uint128::from(10_000_000_000u128),
            ],
            ticket_price: vec![
                Uint128::from(1_000_000u128),
                Uint128::from(2_000_000u128),
                Uint128::from(5_000_000u128),
            ],
            multiplier: vec![
                Decimal::from_str("1").unwrap(),
                Decimal::from_str("2").unwrap(),
                Decimal::from_str("5").unwrap(),
            ],
        };

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);
        let res = instantiate(deps, env, mock_info("creator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        default_init(deps.as_mut());
    }

    #[test]
    fn try_register() {
        let mut deps = mock_dependencies(&[]);
        default_init(deps.as_mut());

        let msg = ExecuteMsg::Register {
            numbers: vec![vec![5, 7, 12, 15, 1], vec![1, 2, 17, 6, 2]],
            address: None,
        };
        let sender = mock_info(
            "alice",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(10_000_000u128),
            }],
        );
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME).plus_seconds(301);

        // Error time not started yet
        let err = execute(deps.as_mut(), env, sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::RegisterClosed {});

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);

        // Error duplicated numbers
        let msg = ExecuteMsg::Register {
            numbers: vec![vec![5, 7, 5, 15, 2], vec![1, 2, 17, 1, 3]],
            address: None,
        };
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::WrongSetOfBallsOrDuplicateNotAllowed {});

        // Error bonus out of range
        let msg = ExecuteMsg::Register {
            numbers: vec![vec![5, 7, 1, 15, 10], vec![1, 2, 17, 5, 0]],
            address: None,
        };
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::BonusOutOfRange {});
        let msg = ExecuteMsg::Register {
            numbers: vec![vec![5, 7, 1, 15, 4], vec![1, 2, 17, 5, 0]],

            address: None,
        };
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::BonusOutOfRange {});

        // Wrong set of balls
        let msg = ExecuteMsg::Register {
            numbers: vec![vec![5, 7, 1, 15, 4, 4], vec![1, 2, 17, 5, 1]],
            address: None,
        };
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::WrongSetOfBallsOrDuplicateNotAllowed {});

        // Success
        let msg = ExecuteMsg::Register {
            numbers: vec![vec![5, 7, 12, 15, 1], vec![1, 2, 17, 6, 4]],
            address: None,
        };
        let res = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("method", "try_register"),
                Attribute::new("round", "0"),
                Attribute::new("ticket_amount", "2"),
                Attribute::new("sender", "alice"),
                Attribute::new("recipient", "alice"),
            ]
        );

        let games = query_games(deps.as_ref(), None, None, 0, "alice".to_string()).unwrap();
        assert_eq!(
            games,
            vec![
                GameResponse {
                    number: vec![1, 2, 17, 6],
                    bonus: 4,
                    multiplier: Decimal::from_str("5").unwrap(),
                    resolved: false,
                    game_id: 1
                },
                GameResponse {
                    number: vec![5, 7, 12, 15],
                    bonus: 1,
                    multiplier: Decimal::from_str("5").unwrap(),
                    resolved: false,
                    game_id: 0
                }
            ]
        );
    }

    #[test]
    fn try_draw() {
        let mut deps = custom_mock_dependencies(&[]);
        default_init(deps.as_mut());

        let msg = ExecuteMsg::Register {
            numbers: vec![vec![5, 7, 12, 15, 1], vec![1, 2, 17, 6, 2]],
            address: None,
        };
        let sender = mock_info(
            "alice",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(10_000_000u128),
            }],
        );
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);
        env.block.time = env.block.time.plus_seconds(300);
        let msg = ExecuteMsg::Draw {};

        let res = execute(deps.as_mut(), env, mock_info("alice", &[]), msg).unwrap();
        println!("{:?}", res);
    }
}
