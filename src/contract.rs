use std::cmp::Ordering;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, Decimal};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::is_lower_hex;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};
use crate::state::{BallsRange, Config, State, CONFIG, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:loterra-v2.0";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        denom: msg.denom,
        start_time: msg.start_time,
        frequency: msg.frequency,
        fee_collector: msg.fee_collector,
        fee_collector_address: deps.api.addr_canonicalize(&msg.fee_collector_address)?,
        fee_collector_drand: msg.fee_collector_drand,
        fee_collector_drand_address: deps
            .api
            .addr_canonicalize(&msg.fee_collector_drand_address)?,
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
        prize_rank: msg.prize_rank,
        ticket_price: msg.ticket_price,
        multiplier: msg.multiplier,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    CONFIG.save(deps.storage, &config)?;

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
        ExecuteMsg::Register {
            numbers,
            bonus,
            address,
        } => try_register(deps, env, info, numbers, bonus, address),
    }
}

pub fn try_register(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    numbers: Vec<Vec<u8>>,
    bonus: u8,
    address: Option<String>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

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

    let sender = match address {
        None => info.sender.to_string(),
        Some(address) => address
    };

    let tiers = state.ticket_price.into_iter().filter(|tier| sent.u128() == tier.u128() * numbers.len() as u128).collect::<Vec<Uint128>>();

    // if tiers.is_empty() || tiers.len() > {
    //     return Err(ContractError::ErrorTierDetermination{})
    // };
    //let multiplier = Decimal::from_ratio(tiers[0], Uint128::from(1_000_000u128));

    // Get the multiplier
    let multiplier = match u128::from(tiers[0]) {
        1_000_000 => state.multiplier[0],
        2_000_000 => state.multiplier[1],
        5_000_000 => state.multiplier[2],
        _ => {return Err(ContractError::ErrorTierDetermination{})}
    };

    // Check if duplicate numbers
    for mut number in numbers.clone() {
        number.sort();
        number.dedup();
        if number.len() as u8 != state.set_of_balls {
            return Err(ContractError::DuplicateNotAllowed{})
        }
    }



    // println!("{}", multiplier);
    // for combo in combination.clone() {
    //     // Regex to check if the combination is allowed
    //     if !is_lower_hex(&combo, state.combination_len) {
    //         return Err(StdError::generic_err(format!(
    //             "Not authorized use combination of [a-f] and [0-9] with length {}",
    //             state.combination_len
    //         )));
    //     }
    // }

    for set_of_balls in numbers {
        if set_of_balls.len() as u8 != state.set_of_balls {
            return Err(ContractError::WrongSetOfBalls(state.set_of_balls))
        }
    }




    Ok(Response::new().add_attribute("method", "try_register"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        denom: config.denom,
        start_time: config.start_time,
        frequency: config.frequency,
        fee_collector: config.fee_collector,
        fee_collector_address: deps
            .api
            .addr_humanize(&config.fee_collector_address)?
            .to_string(),
        fee_collector_drand: config.fee_collector_drand,
        fee_collector_drand_address: deps
            .api
            .addr_humanize(&config.fee_collector_drand_address)?
            .to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Decimal, Uint128, Coin};
    use std::str::FromStr;

    fn default_init(deps: DepsMut) {
        let msg = InstantiateMsg {
            denom: "uusd".to_string(),
            start_time: 0,
            frequency: 300,
            fee_collector: Decimal::from_str("0.05").unwrap(),
            fee_collector_address: "STAKING".to_string(),
            fee_collector_drand: Decimal::from_str("0.01").unwrap(),
            fee_collector_drand_address: "TERRAND".to_string(),
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

        let res = instantiate(deps, mock_env(), mock_info("creator", &[]), msg).unwrap();
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
            numbers: vec![vec![5, 7, 12, 15], vec![1, 2, 17, 6]],
            bonus: 1,
            address: None
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info("alice", &[Coin{ denom: "uusd".to_string(), amount: Uint128::from(10_000_000u128) }]), msg).unwrap();
        println!("{:?}", res);
    }

}
