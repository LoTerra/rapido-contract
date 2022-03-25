#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, SubMsg, Uint128, WasmQuery, from_binary};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use std::convert::TryInto;
use std::ops::Mul;
use std::str::FromStr;
use terrand::msg::MigrateMsg;

use crate::error::ContractError;
use crate::helpers::{bonus_number, count_match, save_game, winning_number};
use crate::msg::{ConfigResponse, ExecuteMsg, GameResponse, GameStatsResponse, InstantiateMsg, LotteryResponse, LotteryStatsResponse, QueryMsg, ReceiveMsg, StateResponse};
use crate::state::{
    BallsRange, Config, GameStats, LotteryState, LotteryStats, State, CONFIG, GAMES, GAMES_STATS,
    LOTTERY_STATE, LOTTERY_STATS, STATE,
};
use crate::taxation::deduct_tax;
use cw20::{Cw20ReceiveMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:loterra-v2.0";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DRAND_GENESIS_TIME: u64 = 1595431050;
const DRAND_PERIOD: u64 = 30;
const DRAND_NEXT_ROUND_SECURITY: u64 = 3;

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
        fee_collector_terrand: msg.fee_collector_terrand,
        terrand_address: deps.api.addr_canonicalize(&msg.terrand_address)?,
        live_round_max: msg.live_round_max,
        burn_rate: msg.burn_rate,
        cw20_contract_address: deps.api.addr_canonicalize(&msg.cw20_contract_address)?
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
    let from_genesis = draw_time.checked_sub(DRAND_GENESIS_TIME).unwrap();
    let next_round = from_genesis
        .checked_div(DRAND_PERIOD)
        .unwrap()
        .checked_add(DRAND_NEXT_ROUND_SECURITY)
        .unwrap();

    LOTTERY_STATE.save(
        deps.storage,
        &state.round.to_be_bytes(),
        &LotteryState {
            draw_time,
            terrand_round: next_round,
            terrand_worker: None,
            prize_rank: msg.prize_rank,
            ticket_price: msg.ticket_price,
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
        ExecuteMsg::Draw {} => try_draw(deps, env, info),
        ExecuteMsg::Collect {
            round,
            player,
            game_id,
        } => try_collect(deps, env, info, round, player, game_id),
        ExecuteMsg::Receive(msg) => try_receive(deps, env, info, msg),
    }
}

pub fn try_receive(deps: DepsMut, env: Env, info: MessageInfo, wrapper: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // only approved cw20 contract can send receive msg
    if info.sender != deps.api.addr_humanize(&config.cw20_contract_address)? {
        return Err(ContractError::Unauthorized {});
    }

    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    match msg {
        ReceiveMsg::Register {numbers, multiplier, live_round, address} => try_register(deps, env, info, wrapper.sender, wrapper.amount, numbers, multiplier, live_round, address),
    }
}

pub fn try_register(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: String,
    amount: Uint128,
    numbers: Vec<u8>,
    multiplier: Uint128,
    live_round: u16,
    address: Option<String>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let lottery = LOTTERY_STATE.load(deps.storage, &state.round.to_be_bytes())?;

    if lottery.draw_time < env.block.time.seconds() {
        return Err(ContractError::RegisterClosed {});
    }

    if live_round == 0 || live_round > config.live_round_max {
        return Err(ContractError::LiveRoundMaxLifeExceeded {});
    }

    let address_raw = match address {
        None => deps.api.addr_canonicalize(&sender)?,
        Some(address) => deps.api.addr_canonicalize(&address)?,
    };

    //let price_per_ticket = sent.div(Uint128::from(numbers.len() as u128));
    //let price_per_round = sent.div(Uint128::from(live_round as u128));

    // let tiers = lottery
    //     .ticket_price
    //     .into_iter()
    //     .filter(|tier| &sent == tier)
    //     .collect::<Vec<Uint128>>();

    //println!("mul ratio {}", price_per_ticket.mul(decimal_per_round));

    // if tiers.is_empty() || tiers.len() > {
    //     return Err(ContractError::ErrorTierDetermination{})
    // };
    //let multiplier = Decimal::from_ratio(tiers[0], Uint128::from(1_000_000u128));

    // Get the multiplier
    let multiplier_decimal = match multiplier.u128() {
        1_000_000 => lottery.multiplier[0],
        2_000_000 => lottery.multiplier[1],
        5_000_000 => lottery.multiplier[2],
        _ => {
            return Err(ContractError::ErrorTierDetermination {});
        }
    };

    let expected_amount = state.ticket_price[0]
        .mul(multiplier_decimal)
        .mul(Decimal::from_str(&live_round.to_string()).unwrap());
    if amount != expected_amount {
        return Err(ContractError::AmountSentError(amount, expected_amount));
    };

    let mut new_number = vec![];
    // Check if duplicate numbers
    //for mut number in numbers.clone() {
    let mut new_arr = numbers.clone();
    let bonus_number = numbers.last().unwrap();
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
    //}
    let mut rounds_info = vec![];
    for round in state.round..state.round.checked_add(u64::from(live_round)).unwrap() {
        rounds_info.push(round.to_string());
        match GAMES_STATS.may_load(
            deps.storage,
            (&address_raw.as_slice(), &round.to_be_bytes()),
        )? {
            None => {
                save_game(
                    deps.storage,
                    round,
                    &address_raw,
                    numbers.clone(),
                    multiplier_decimal,
                    None,
                )?;
                GAMES_STATS.save(
                    deps.storage,
                    (&address_raw.as_slice(), &round.to_be_bytes()),
                    &GameStats {
                        total_ticket: 1,
                        total_spent: multiplier,
                    },
                )?;

                match LOTTERY_STATS.may_load(deps.storage, &round.to_be_bytes())? {
                    None => {
                        LOTTERY_STATS.save(
                            deps.storage,
                            &round.to_be_bytes(),
                            &LotteryStats {
                                counter_player: Some(1),
                                total_ticket_sold: Some(1),
                                total_collected: Some(multiplier),
                            },
                        )?;
                    }
                    Some(_) => {
                        LOTTERY_STATS.update(
                            deps.storage,
                            &round.to_be_bytes(),
                            |lottery_stats| -> Result<_, ContractError> {
                                let mut update_lottery_stats = lottery_stats.unwrap();

                                if update_lottery_stats.counter_player.is_none() {
                                    update_lottery_stats.counter_player = Some(1);
                                } else {
                                    update_lottery_stats.counter_player =
                                        update_lottery_stats.counter_player.unwrap().checked_add(1);
                                }

                                if update_lottery_stats.total_ticket_sold.is_none() {
                                    update_lottery_stats.total_ticket_sold = Some(1);
                                } else {
                                    update_lottery_stats.total_ticket_sold = update_lottery_stats
                                        .total_ticket_sold
                                        .unwrap()
                                        .checked_add(1);
                                }
                                if update_lottery_stats.total_collected.is_none() {
                                    update_lottery_stats.total_collected = Some(multiplier);
                                } else {
                                    update_lottery_stats.total_collected = Some(
                                        update_lottery_stats
                                            .total_collected
                                            .unwrap()
                                            .checked_add(multiplier)
                                            .unwrap(),
                                    );
                                }

                                Ok(update_lottery_stats)
                            },
                        )?;
                    }
                }
            }
            Some(game_stats) => {
                save_game(
                    deps.storage,
                    round,
                    &address_raw,
                    numbers.clone(),
                    multiplier_decimal,
                    Some(game_stats),
                )?;

                GAMES_STATS.update(
                    deps.storage,
                    (&address_raw.as_slice(), &round.to_be_bytes()),
                    |game_stats| -> Result<_, ContractError> {
                        let mut update_game_stats = game_stats.unwrap();
                        update_game_stats.total_spent = update_game_stats
                            .total_spent
                            .checked_add(multiplier)
                            .unwrap();
                        update_game_stats.total_ticket += 1;
                        Ok(update_game_stats)
                    },
                )?;

                match LOTTERY_STATS.may_load(deps.storage, &round.to_be_bytes())? {
                    None => {
                        LOTTERY_STATS.save(
                            deps.storage,
                            &round.to_be_bytes(),
                            &LotteryStats {
                                counter_player: Some(1),
                                total_ticket_sold: Some(1),
                                total_collected: Some(multiplier),
                            },
                        )?;
                    }
                    Some(_) => {
                        LOTTERY_STATS.update(
                            deps.storage,
                            &round.to_be_bytes(),
                            |lottery_stats| -> Result<_, ContractError> {
                                let mut update_lottery_stats = lottery_stats.unwrap();

                                if update_lottery_stats.total_ticket_sold.is_none() {
                                    update_lottery_stats.total_ticket_sold = Some(1);
                                } else {
                                    update_lottery_stats.total_ticket_sold = update_lottery_stats
                                        .total_ticket_sold
                                        .unwrap()
                                        .checked_add(1);
                                }
                                if update_lottery_stats.total_collected.is_none() {
                                    update_lottery_stats.total_collected = Some(multiplier);
                                } else {
                                    update_lottery_stats.total_collected = Some(
                                        update_lottery_stats
                                            .total_collected
                                            .unwrap()
                                            .checked_add(multiplier)
                                            .unwrap(),
                                    );
                                }

                                Ok(update_lottery_stats)
                            },
                        )?;
                    }
                }
            }
        }
    }

    Ok(Response::new()
        .add_attribute("method", "try_register")
        .add_attribute("round", format!("[{}]", rounds_info.join(", ")))
        .add_attribute("live_round", live_round.to_string())
        .add_attribute("ticket_amount", "1".to_string())
        .add_attribute("sender", info.sender)
        .add_attribute(
            "recipient",
            deps.api.addr_humanize(&address_raw)?.to_string(),
        ))
}

pub fn try_draw(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let lottery = LOTTERY_STATE.load(deps.storage, &state.round.to_be_bytes())?;
    let lottery_stats = LOTTERY_STATS.may_load(deps.storage, &state.round.to_be_bytes())?;

    if lottery.draw_time > env.block.time.seconds() {
        return Err(ContractError::LotteryInProgress {});
    }

    let mut msgs = vec![];
    if lottery_stats.is_some() {
        // Query terrand for the randomness
        let msg = terrand::msg::QueryMsg::GetRandomness {
            round: lottery.terrand_round,
        };
        let terrand_human = deps.api.addr_humanize(&config.terrand_address)?;
        let query = WasmQuery::Smart {
            contract_addr: terrand_human.to_string(),
            msg: to_binary(&msg)?,
        };
        let terrand_randomness: terrand::msg::GetRandomResponse =
            deps.querier.query(&query.into())?;
        let randomness_hash: String = hex::encode(terrand_randomness.randomness.as_slice());
        // let x = random_number(randomness_hash.clone(), state.set_of_balls, state.range.max);

        let numbers: Vec<_> = randomness_hash.chars().collect();
        let winning_number = winning_number(numbers.clone())?;
        let bonus_number = bonus_number(numbers.last().unwrap())?;

        let worker_raw = deps.api.addr_canonicalize(&terrand_randomness.worker)?;
        // Update lottery winning and bonus number
        LOTTERY_STATE.update(
            deps.storage,
            &state.round.to_be_bytes(),
            |lottery_state| -> Result<_, ContractError> {
                let mut update_lottery_state = lottery_state.unwrap();
                update_lottery_state.winning_number = Some(winning_number);
                update_lottery_state.bonus_number = Some(bonus_number);
                update_lottery_state.terrand_worker = Some(worker_raw);
                Ok(update_lottery_state)
            },
        )?;

        let randomness_fee = CosmosMsg::Bank(BankMsg::Send {
            to_address: terrand_randomness.worker,
            amount: vec![deduct_tax(
                &deps.querier,
                Coin {
                    denom: config.denom.clone(),
                    amount: Uint128::from(1_000_000u128),
                },
            )?],
        });

        msgs.push(randomness_fee)
    }

    // Update state & save
    state.round += 1;
    STATE.save(deps.storage, &state)?;

    // calculate next round randomness from now
    let draw_time = env.block.time.plus_seconds(config.frequency).seconds();
    let from_genesis = draw_time.checked_sub(DRAND_GENESIS_TIME).unwrap();
    let next_round = from_genesis
        .checked_div(DRAND_PERIOD)
        .unwrap()
        .checked_add(DRAND_NEXT_ROUND_SECURITY)
        .unwrap();

    // Create new lottery
    LOTTERY_STATE.save(
        deps.storage,
        &state.round.to_be_bytes(),
        &LotteryState {
            draw_time,
            terrand_round: next_round,
            terrand_worker: None,
            prize_rank: state.prize_rank,
            ticket_price: state.ticket_price,
            multiplier: state.multiplier,
            winning_number: None,
            bonus_number: None,
        },
    )?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("method", "try_draw")
        .add_attribute("round", state.round.checked_sub(1).unwrap().to_string()))
}

pub fn try_collect(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    round: u64,
    player: String,
    game_id: Vec<u64>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let player_raw = deps
        .api
        .addr_canonicalize(&Addr::unchecked(player.clone()).as_str())?;
    let lottery = LOTTERY_STATE.load(deps.storage, &round.to_be_bytes())?;

    if lottery.winning_number.is_none() && lottery.bonus_number.is_none() {
        return Err(ContractError::LotteryInProgress {});
    }

    let mut total_amount_to_send = Uint128::zero();
    for id in game_id {
        let game = GAMES.load(
            deps.storage,
            (
                &round.to_be_bytes(),
                &player_raw.as_slice(),
                &id.to_be_bytes(),
            ),
        )?;

        if !game.resolved {
            let match_amount = count_match(
                &game.number,
                &lottery.clone().winning_number.unwrap(),
                state.set_of_balls,
            );
            let bonus = lottery.bonus_number.unwrap() == game.bonus;

            let prize = if lottery.prize_rank.len() == 9 {
                match match_amount {
                    0 if bonus => lottery.prize_rank[0],
                    1 if !bonus => lottery.prize_rank[1],
                    1 if bonus => lottery.prize_rank[2],
                    2 if !bonus => lottery.prize_rank[3],
                    2 if bonus => lottery.prize_rank[4],
                    3 if !bonus => lottery.prize_rank[5],
                    3 if bonus => lottery.prize_rank[6],
                    4 if !bonus => lottery.prize_rank[7],
                    4 if bonus => lottery.prize_rank[8],
                    _ => Uint128::zero(),
                }
            } else {
                match match_amount {
                    1 if !bonus => lottery.prize_rank[0],
                    1 if bonus => lottery.prize_rank[1],
                    2 if !bonus => lottery.prize_rank[2],
                    2 if bonus => lottery.prize_rank[3],
                    3 if !bonus => lottery.prize_rank[4],
                    3 if bonus => lottery.prize_rank[5],
                    4 if !bonus => lottery.prize_rank[6],
                    4 if bonus => lottery.prize_rank[7],
                    _ => Uint128::zero(),
                }
            };
            // let prize = match match_amount {
            //     0 if bonus => state.prize_rank[0],
            //     1 if !bonus => state.prize_rank[1],
            //     1 if bonus => state.prize_rank[2],
            //     2 if !bonus => state.prize_rank[3],
            //     2 if bonus => state.prize_rank[4],
            //     3 if !bonus => state.prize_rank[5],
            //     3 if bonus => state.prize_rank[6],
            //     4 if !bonus => state.prize_rank[7],
            //     4 if bonus => state.prize_rank[8],
            //     _ => Uint128::zero(),
            // };
            let price_multiplier = prize.mul(game.multiplier);
            total_amount_to_send = total_amount_to_send.checked_add(price_multiplier).unwrap();

            GAMES.update(
                deps.storage,
                (
                    &round.to_be_bytes(),
                    &player_raw.as_slice(),
                    &id.to_be_bytes(),
                ),
                |game| -> Result<_, ContractError> {
                    let mut update_game = game.unwrap();
                    update_game.resolved = true;
                    Ok(update_game)
                },
            )?;
        }
    }

    let mut res = Response::new();

    if !total_amount_to_send.is_zero() {
        let collector_tax_amount = total_amount_to_send.mul(config.fee_collector);
        let terrand_tax_amount = total_amount_to_send.mul(config.fee_collector_terrand);

        let msg_prize_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: player,
            amount: vec![deduct_tax(
                &deps.querier,
                Coin {
                    denom: config.denom.clone(),
                    amount: total_amount_to_send
                        .checked_sub(collector_tax_amount)
                        .unwrap()
                        .checked_sub(terrand_tax_amount)
                        .unwrap(),
                },
            )?],
        });
        res.messages.push(SubMsg::new(msg_prize_payout));

        let msg_fee_collector_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: deps
                .api
                .addr_humanize(&config.fee_collector_address)?
                .to_string(),
            amount: vec![deduct_tax(
                &deps.querier,
                Coin {
                    denom: config.denom.clone(),
                    amount: collector_tax_amount,
                },
            )?],
        });
        res.messages.push(SubMsg::new(msg_fee_collector_payout));

        // prepare message to pay tax to terrand worker
        let msg_fee_terrand_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: deps
                .api
                .addr_humanize(&lottery.terrand_worker.unwrap())?
                .to_string(),
            amount: vec![deduct_tax(
                &deps.querier,
                Coin {
                    denom: config.denom,
                    amount: terrand_tax_amount,
                },
            )?],
        });
        res.messages.push(SubMsg::new(msg_fee_terrand_payout));
    }

    res.attributes.push(Attribute::new("method", "try_collect"));

    Ok(res)
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
        QueryMsg::LotteryState { round } => to_binary(&query_lottery_state(deps, round)?),
        QueryMsg::LotteriesState { start_after, limit } => {
            to_binary(&query_lotteries_state(deps, start_after, limit)?)
        }
        QueryMsg::GameStats {
            player,
            start_after,
            limit,
        } => to_binary(&query_game_stats(deps, player, start_after, limit)?),
        QueryMsg::LotteryStats { round } => to_binary(&query_lottery_stats(deps, round)?),
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
        fee_collector_terrand: config.fee_collector_terrand,
        fee_collector_terrand_address: deps.api.addr_humanize(&config.terrand_address)?.to_string(),
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
            pair.map(|(k, game)| GameResponse {
                number: game.number,
                bonus: game.bonus,
                multiplier: game.multiplier,
                resolved: game.resolved,
                game_id: u64::from_be_bytes(k.try_into().unwrap()),
                lottery_id: round,
            })
        })
        .collect::<StdResult<Vec<GameResponse>>>()?;

    Ok(games)
}

fn query_lottery_state(deps: Deps, round: u64) -> StdResult<LotteryResponse> {
    let lottery = LOTTERY_STATE.load(deps.storage, &round.to_be_bytes())?;
    let worker = match lottery.terrand_worker {
        None => None,
        Some(terrand_worker) => Some(deps.api.addr_humanize(&terrand_worker)?.to_string()),
    };
    Ok(LotteryResponse {
        draw_time: lottery.draw_time,
        terrand_round: lottery.terrand_round,
        terrand_worker: worker,
        prize_rank: lottery.prize_rank,
        ticket_price: lottery.ticket_price,
        multiplier: lottery.multiplier,
        winning_number: lottery.winning_number,
        bonus_number: lottery.bonus_number,
        lottery_id: round,
    })
}
fn query_lotteries_state(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<LotteryResponse>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|d| Bound::Exclusive(d.to_be_bytes().to_vec()));

    let lotteries = LOTTERY_STATE
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|pair| {
            pair.and_then(|(k, lottery)| {
                let worker = match lottery.terrand_worker {
                    None => None,
                    Some(terrand_worker) => {
                        Some(deps.api.addr_humanize(&terrand_worker)?.to_string())
                    }
                };
                Ok(LotteryResponse {
                    draw_time: lottery.draw_time,
                    terrand_round: lottery.terrand_round,
                    terrand_worker: worker,
                    prize_rank: lottery.prize_rank,
                    ticket_price: lottery.ticket_price,
                    multiplier: lottery.multiplier,
                    winning_number: lottery.winning_number,
                    bonus_number: lottery.bonus_number,
                    lottery_id: u64::from_be_bytes(k.try_into().unwrap()),
                })
            })
        })
        .collect::<StdResult<Vec<LotteryResponse>>>()?;

    Ok(lotteries)
}

fn query_game_stats(
    deps: Deps,
    player: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<GameStatsResponse>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|d| Bound::Exclusive(d.to_be_bytes().to_vec()));

    let owner_addr = deps.api.addr_validate(&player)?;
    let raw_address = deps.api.addr_canonicalize(&owner_addr.as_str())?;
    let game_stats = GAMES_STATS
        .prefix(&raw_address.as_slice())
        .range(deps.storage, None, start, Order::Descending)
        .take(limit)
        .map(|pair| {
            pair.map(|(k, game_stats)| GameStatsResponse {
                total_ticket: game_stats.total_ticket,
                total_spent: game_stats.total_spent,
                game_stats_id: u64::from_be_bytes(k.try_into().unwrap()),
            })
        })
        .collect::<StdResult<Vec<GameStatsResponse>>>()?;

    Ok(game_stats)
}

fn query_lottery_stats(deps: Deps, round: u64) -> StdResult<LotteryStatsResponse> {
    let lottery_stats = LOTTERY_STATS.may_load(deps.storage, &round.to_be_bytes())?;

    let lottery_stats = match lottery_stats {
        None => LotteryStatsResponse {
            counter_player: None,
            total_ticket_sold: None,
            total_collected: None,
            lottery_stats_id: round,
        },
        Some(lottery_stats) => LotteryStatsResponse {
            counter_player: lottery_stats.counter_player,
            total_ticket_sold: lottery_stats.total_ticket_sold,
            total_collected: lottery_stats.total_collected,
            lottery_stats_id: round,
        },
    };

    Ok(lottery_stats)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    //let mut config = CONFIG.load(deps.storage)?;
    //config.fee_collector = Decimal::from_str("0.1").unwrap();
    //let mut state = STATE.load(deps.storage)?;
    // state.prize_rank = vec![
    //     Uint128::from(2_000_000u128),
    //     Uint128::from(1_000_000u128),
    //     Uint128::from(5_000_000u128),
    //     Uint128::from(10_000_000u128),
    //     Uint128::from(30_000_000u128),
    //     Uint128::from(50_000_000u128),
    //     Uint128::from(150_000_000u128),
    //     Uint128::from(1_000_000_000u128),
    //     Uint128::from(10_000_000_000u128),
    // ];
    // STATE.save(deps.storage, &state)?;
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use std::result;
    use super::*;
    use crate::mock_querier::custom_mock_dependencies;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Attribute, Coin, Decimal, Timestamp, Uint128};
    use std::str::FromStr;

    fn default_init(deps: DepsMut) {
        let msg = InstantiateMsg {
            denom: "uusd".to_string(),
            frequency: 86400,
            fee_collector: Decimal::from_str("0.05").unwrap(),
            fee_collector_address: "STAKING".to_string(),
            fee_collector_terrand: Decimal::from_str("0.01").unwrap(),
            terrand_address: "TERRAND".to_string(),
            set_of_balls: 5,
            range_min: 1,
            range_max: 16,
            bonus_set_of_balls: 1,
            bonus_range_min: 1,
            bonus_range_max: 8,
            prize_rank: vec![
                Uint128::from(1_000_000u128),
                Uint128::from(2_000_000u128),
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
            live_round_max: 5,
            burn_rate: Decimal::from_str("0.5").unwrap(),
            cw20_contract_address: "LOTA".to_string()
        };

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);
        let res = instantiate(deps, env, mock_info("creator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    fn custom_init_prize(deps: DepsMut) {
        let msg = InstantiateMsg {
            denom: "uusd".to_string(),
            frequency: 86400,
            fee_collector: Decimal::from_str("0.05").unwrap(),
            fee_collector_address: "STAKING".to_string(),
            fee_collector_terrand: Decimal::from_str("0.01").unwrap(),
            terrand_address: "TERRAND".to_string(),
            set_of_balls: 5,
            range_min: 1,
            range_max: 16,
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
            live_round_max: 5,
            burn_rate: Decimal::from_str("0.5").unwrap(),
            cw20_contract_address: "LOTA".to_string()
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

        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 13, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);

        let sender = mock_info(
            "LOTA",
            &[],
        );
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME).plus_seconds(86401);

        // Error time not started yet
        let err = execute(deps.as_mut(), env, sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::RegisterClosed {});

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);

        // Remove this because not fitting with the current design
        // // Error duplicated numbers
        // let msg = ExecuteMsg::Register {
        //     numbers: vec![vec![5, 7, 5, 15, 2], vec![1, 2, 17, 1, 3]],
        //     address: None,
        //     live_round: 1
        // };
        // let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        // assert_eq!(err, ContractError::WrongSetOfBallsOrDuplicateNotAllowed {});

        // Error bonus out of range
        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 13, 10],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::BonusOutOfRange {});
        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 13, 0],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::BonusOutOfRange {});

        // live round error max life exceeded
        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 13, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 0,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::LiveRoundMaxLifeExceeded {});

        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 13, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 6,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::LiveRoundMaxLifeExceeded {});

        // // Wrong set of balls
        // let msg = ExecuteMsg::Register {
        //     numbers: vec![vec![5, 7, 1, 15, 4, 4], vec![1, 2, 17, 5, 1]],
        //     address: None,
        //     live_round: 1
        // };
        // let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        // assert_eq!(err, ContractError::WrongSetOfBallsOrDuplicateNotAllowed {});

        // Success
        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 13, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("method", "try_register"),
                Attribute::new("round", "[0]"),
                Attribute::new("live_round", "1"),
                Attribute::new("ticket_amount", "1"),
                Attribute::new("sender", "alice"),
                Attribute::new("recipient", "alice"),
            ]
        );
        // Error sent less than required
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1_000_000u128),
            }],
        );

        let msg = ReceiveMsg::Register {
            numbers: vec![1, 2, 17, 6, 4],
            multiplier: Uint128::from(1_000_000u128),
            live_round: 4,
            address: None,
        };

        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::AmountSentError(
                Uint128::from(1_000_000u128),
                Uint128::from(4_000_000u128)
            )
        );
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(4_000_000u128),
            }],
        );

        let msg = ReceiveMsg::Register {
            numbers: vec![1, 2, 17, 6, 12, 4],
            multiplier: Uint128::from(1_000_000u128),
            live_round: 4,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("method", "try_register"),
                Attribute::new("round", "[0, 1, 2, 3]"),
                Attribute::new("live_round", "4"),
                Attribute::new("ticket_amount", "1"),
                Attribute::new("sender", "alice"),
                Attribute::new("recipient", "alice"),
            ]
        );

        let games = query_games(deps.as_ref(), None, None, 0, "alice".to_string()).unwrap();
        assert_eq!(
            games,
            vec![
                GameResponse {
                    number: vec![1, 2, 17, 6, 12],
                    bonus: 4,
                    multiplier: Decimal::from_str("1").unwrap(),
                    resolved: false,
                    game_id: 1,
                    lottery_id: 0
                },
                GameResponse {
                    number: vec![5, 7, 12, 15, 13],
                    bonus: 1,
                    multiplier: Decimal::from_str("5").unwrap(),
                    resolved: false,
                    game_id: 0,
                    lottery_id: 0
                }
            ]
        );
        let games = query_games(deps.as_ref(), None, None, 1, "alice".to_string()).unwrap();
        assert_eq!(
            games,
            vec![GameResponse {
                number: vec![1, 2, 17, 6, 12],
                bonus: 4,
                multiplier: Decimal::from_str("1").unwrap(),
                resolved: false,
                game_id: 0,
                lottery_id: 1
            }]
        );
        let games = query_games(deps.as_ref(), None, None, 2, "alice".to_string()).unwrap();
        assert_eq!(
            games,
            vec![GameResponse {
                number: vec![1, 2, 17, 6, 12],
                bonus: 4,
                multiplier: Decimal::from_str("1").unwrap(),
                resolved: false,
                game_id: 0,
                lottery_id: 2
            }]
        );
        let games = query_games(deps.as_ref(), None, None, 3, "alice".to_string()).unwrap();
        assert_eq!(
            games,
            vec![GameResponse {
                number: vec![1, 2, 17, 6, 12],
                bonus: 4,
                multiplier: Decimal::from_str("1").unwrap(),
                resolved: false,
                game_id: 0,
                lottery_id: 3
            }]
        );

        // Error sent
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(10_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 1,
            address: None,
        };

        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "bob".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let err = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::AmountSentError(
                Uint128::from(10_000_000u128),
                Uint128::from(5_000_000u128)
            )
        );

        // Success
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(5_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 13, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "bob".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), env.clone(), sender.clone(), msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("method", "try_register"),
                Attribute::new("round", "[0]"),
                Attribute::new("live_round", "1"),
                Attribute::new("ticket_amount", "1"),
                Attribute::new("sender", "bob"),
                Attribute::new("recipient", "bob"),
            ]
        );

        let games = query_games(deps.as_ref(), None, None, 0, "bob".to_string()).unwrap();
        assert_eq!(
            games,
            vec![GameResponse {
                number: vec![5, 7, 12, 15, 13],
                bonus: 1,
                multiplier: Decimal::from_str("5").unwrap(),
                resolved: false,
                game_id: 0,
                lottery_id: 0
            },]
        );
    }

    #[test]
    fn try_draw() {
        let mut deps = custom_mock_dependencies(&[]);
        default_init(deps.as_mut());

        let msg = ExecuteMsg::Draw {};
        let err = execute(deps.as_mut(), mock_env(), mock_info("alice", &[]), msg).unwrap_err();
        assert_eq!(err, ContractError::LotteryInProgress {});

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);
        env.block.time = env.block.time.plus_seconds(300);
        let msg = ExecuteMsg::Draw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();

        let past_lottery_state = query_lottery_state(deps.as_ref(), 0).unwrap();
        assert_eq!(past_lottery_state.terrand_round, 13);
        assert_eq!(past_lottery_state.draw_time, 1595431350);
        assert_eq!(past_lottery_state.bonus_number, None);
        assert_eq!(past_lottery_state.winning_number, None);
        assert_eq!(
            past_lottery_state.multiplier,
            vec![
                Decimal::from_str("1").unwrap(),
                Decimal::from_str("2").unwrap(),
                Decimal::from_str("5").unwrap()
            ]
        );
        assert_eq!(
            past_lottery_state.ticket_price,
            vec![
                Uint128::from(1000000u128),
                Uint128::from(2000000u128),
                Uint128::from(5000000u128)
            ]
        );
        assert_eq!(
            past_lottery_state.prize_rank,
            vec![
                Uint128::from(1000000u128),
                Uint128::from(2000000u128),
                Uint128::from(5000000u128),
                Uint128::from(10000000u128),
                Uint128::from(30000000u128),
                Uint128::from(50000000u128),
                Uint128::from(150000000u128),
                Uint128::from(1000000000u128),
                Uint128::from(10000000000u128)
            ]
        );

        let new_lottery_state = query_lottery_state(deps.as_ref(), 1).unwrap();
        assert_eq!(new_lottery_state.terrand_round, 23);
        assert_eq!(new_lottery_state.draw_time, 1595431650);
        assert_eq!(new_lottery_state.bonus_number, None);
        assert_eq!(new_lottery_state.winning_number, None);
        assert_eq!(
            new_lottery_state.multiplier,
            vec![
                Decimal::from_str("1").unwrap(),
                Decimal::from_str("2").unwrap(),
                Decimal::from_str("5").unwrap()
            ]
        );
        assert_eq!(
            new_lottery_state.ticket_price,
            vec![
                Uint128::from(1000000u128),
                Uint128::from(2000000u128),
                Uint128::from(5000000u128)
            ]
        );
        assert_eq!(
            new_lottery_state.prize_rank,
            vec![
                Uint128::from(1000000u128),
                Uint128::from(2000000u128),
                Uint128::from(5000000u128),
                Uint128::from(10000000u128),
                Uint128::from(30000000u128),
                Uint128::from(50000000u128),
                Uint128::from(150000000u128),
                Uint128::from(1000000000u128),
                Uint128::from(10000000000u128)
            ]
        );

        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 2,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let sender = mock_info(
            "bob",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(10_000_000u128),
            }],
        );

        let res = execute(deps.as_mut(), env.clone(), sender, msg.clone()).unwrap();
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(10_000_000u128),
            }],
        );
        let res = execute(deps.as_mut(), env.clone(), sender, msg).unwrap();

        env.block.time = env.block.time.plus_seconds(300);
        let msg = ExecuteMsg::Draw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();
        let worker_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: "worker".to_string(),
            amount: vec![Coin::new(990_099, "uusd")],
        });
        assert_eq!(res.messages, vec![SubMsg::new(worker_msg)]);

        let lottery_stats = query_lottery_stats(deps.as_ref(), 1).unwrap();
        println!("{:?}", lottery_stats);
        let new_lottery_state = query_lottery_state(deps.as_ref(), 1).unwrap();
        assert_eq!(new_lottery_state.bonus_number, Some(7));
        assert_eq!(new_lottery_state.winning_number, Some(vec![4, 15, 6, 4]));

        let lottery_stats = query_lottery_stats(deps.as_ref(), 2).unwrap();
        println!("{:?}", lottery_stats);
        let new_lottery_state = query_lottery_state(deps.as_ref(), 2).unwrap();

        assert_eq!(new_lottery_state.bonus_number, None);
        assert_eq!(new_lottery_state.winning_number, None);
    }

    #[test]
    fn try_collect() {
        let mut deps = custom_mock_dependencies(&[]);
        default_init(deps.as_mut());

        let sender = mock_info(
            "alice",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(10_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![5, 7, 12, 15, 1],
            multiplier: Uint128::from(5_000_000u128),
            live_round: 2,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();

        // Alice winning number found
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![4, 15, 6, 4, 7],
            multiplier: Uint128::from(2_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();

        // Bob 3 numbers found and 1 bonus
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![4, 15, 6, 5, 7],
            multiplier: Uint128::from(2_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "bob".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();

        // Charlie 2 numbers found and 0 bonus
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![4, 15, 6, 2, 2],
            multiplier: Uint128::from(2_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "charlie".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();

        // Mario 0 numbers found and 1 bonus refund
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![1, 1, 1, 1, 7],
            multiplier: Uint128::from(1_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "mario".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();
        // Mario 0 numbers found and 1 bonus refund
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![4, 1, 1, 1, 1],
            multiplier: Uint128::from(1_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "mario".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();
        // Mario 0 numbers found
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![1, 1, 1, 1, 1],
            multiplier: Uint128::from(1_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "mario".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);
        env.block.time = env.block.time.plus_seconds(300);
        let msg = ExecuteMsg::Draw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();

        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "alice".to_string(),
            game_id: vec![0, 1],
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("alice", &[]),
            msg.clone(),
        )
        .unwrap();
        let msg_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: "alice".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(18_799_000_000u128),
            }],
        });
        let msg_fee_collector = CosmosMsg::Bank(BankMsg::Send {
            to_address: "STAKING".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(999_000_000u128),
            }],
        });
        let msg_fee_worker = CosmosMsg::Bank(BankMsg::Send {
            to_address: "worker".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(199_000_000u128),
            }],
        });
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(msg_payout),
                SubMsg::new(msg_fee_collector),
                SubMsg::new(msg_fee_worker)
            ]
        );
        // Collect again error with no prize
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "bob".to_string(),
            game_id: vec![0],
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();
        let msg_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: "bob".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(281_000_000u128),
            }],
        });
        let msg_fee_collector = CosmosMsg::Bank(BankMsg::Send {
            to_address: "STAKING".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(14_851_485u128),
            }],
        });
        let msg_fee_worker = CosmosMsg::Bank(BankMsg::Send {
            to_address: "worker".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2_970_297u128),
            }],
        });
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(msg_payout),
                SubMsg::new(msg_fee_collector),
                SubMsg::new(msg_fee_worker)
            ]
        );

        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "charlie".to_string(),
            game_id: vec![0],
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();
        let msg_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: "charlie".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(93_069_306u128),
            }],
        });
        let msg_fee_collector = CosmosMsg::Bank(BankMsg::Send {
            to_address: "STAKING".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(4_950_495u128),
            }],
        });
        let msg_fee_worker = CosmosMsg::Bank(BankMsg::Send {
            to_address: "worker".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(990_099u128),
            }],
        });
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(msg_payout),
                SubMsg::new(msg_fee_collector),
                SubMsg::new(msg_fee_worker)
            ]
        );

        // Win a refund
        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "mario".to_string(),
            game_id: vec![0],
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();
        let msg_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: "mario".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(930_693u128),
            }],
        });
        let msg_fee_collector = CosmosMsg::Bank(BankMsg::Send {
            to_address: "STAKING".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(49_504u128),
            }],
        });
        let msg_fee_worker = CosmosMsg::Bank(BankMsg::Send {
            to_address: "worker".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(9_900u128),
            }],
        });
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(msg_payout),
                SubMsg::new(msg_fee_collector),
                SubMsg::new(msg_fee_worker)
            ]
        );
        // Win 2UST
        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "mario".to_string(),
            game_id: vec![1],
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();
        let msg_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: "mario".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(1_861_386u128),
            }],
        });
        let msg_fee_collector = CosmosMsg::Bank(BankMsg::Send {
            to_address: "STAKING".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(99_009u128),
            }],
        });
        let msg_fee_worker = CosmosMsg::Bank(BankMsg::Send {
            to_address: "worker".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(19_801u128),
            }],
        });
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(msg_payout),
                SubMsg::new(msg_fee_collector),
                SubMsg::new(msg_fee_worker)
            ]
        );

        // Win 0UST
        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "mario".to_string(),
            game_id: vec![2],
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // Error too soon to collect
        let msg = ExecuteMsg::Collect {
            round: 1,
            player: "alice".to_string(),
            game_id: vec![0],
        };

        let err = execute(deps.as_mut(), env, mock_info("alice", &[]), msg).unwrap_err();
        assert_eq!(err, ContractError::LotteryInProgress {});
    }

    #[test]
    fn try_collect_on_custom_init() {
        let mut deps = custom_mock_dependencies(&[]);
        custom_init_prize(deps.as_mut());

        // bonus not counting
        let sender = mock_info(
            "LOTA",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(2_000_000u128),
            }],
        );
        let msg = ReceiveMsg::Register {
            numbers: vec![1, 1, 1, 1, 7],
            multiplier: Uint128::from(2_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();

        // 1 number
        let msg = ReceiveMsg::Register {
            numbers: vec![4, 1, 1, 1, 7],
            multiplier: Uint128::from(2_000_000u128),
            live_round: 1,
            address: None,
        };
        let cw20_receive_msg = Cw20ReceiveMsg {
            sender: "alice".to_string(),
            amount: Uint128::from(5_000_000u128),
            msg: to_binary(&msg).unwrap(),
        };
        let msg = ExecuteMsg::Receive(cw20_receive_msg);
        let res = execute(deps.as_mut(), mock_env(), sender.clone(), msg).unwrap();

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DRAND_GENESIS_TIME);
        env.block.time = env.block.time.plus_seconds(300);
        let msg = ExecuteMsg::Draw {};
        let res = execute(deps.as_mut(), env.clone(), mock_info("alice", &[]), msg).unwrap();

        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "alice".to_string(),
            game_id: vec![0],
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("alice", &[]),
            msg.clone(),
        )
        .unwrap();

        assert_eq!(res.messages.len(), 0);

        let msg = ExecuteMsg::Collect {
            round: 0,
            player: "alice".to_string(),
            game_id: vec![1],
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("alice", &[]),
            msg.clone(),
        )
        .unwrap();
        let msg_payout = CosmosMsg::Bank(BankMsg::Send {
            to_address: "alice".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(9_306_930u128),
            }],
        });
        let msg_fee_collector = CosmosMsg::Bank(BankMsg::Send {
            to_address: "STAKING".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(495_049u128),
            }],
        });
        let msg_fee_worker = CosmosMsg::Bank(BankMsg::Send {
            to_address: "worker".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(99_009u128),
            }],
        });
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(msg_payout),
                SubMsg::new(msg_fee_collector),
                SubMsg::new(msg_fee_worker)
            ]
        );
    }
}
