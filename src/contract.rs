#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};
use crate::state::{BallsRange, Config, CONFIG, State, STATE};

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
        fee_collector_drand_address: deps.api.addr_canonicalize(&msg.fee_collector_drand_address)?
    };

    let state = State {
        round: 0,
        set_of_balls: msg.set_of_balls,
        range: BallsRange { min: msg.range_min, max: msg.range_max },
        bonus_set_of_balls: msg.bonus_set_of_balls,
        bonus_range: BallsRange { min: msg.bonus_range_min, max: msg.bonus_range_max },
        prize_rank: msg.prize_rank,
        ticket_price: msg.ticket_price,
        multiplier: msg.multiplier
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Register { numbers, bonus, address} => try_register(deps, env, info, numbers, bonus, address),
    }
}

pub fn try_register(deps: DepsMut, env: Env, info: MessageInfo, numbers: Vec<u8>, bonus: u8, address: Option<String> ) -> Result<Response, ContractError> {



    Ok(Response::new().add_attribute("method", "try_increment"))
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
    Ok(ConfigResponse{
        denom: config.denom,
        start_time: config.start_time,
        frequency: config.frequency,
        fee_collector: config.fee_collector,
        fee_collector_address: deps.api.addr_humanize(&config.fee_collector_address)?.to_string(),
        fee_collector_drand: config.fee_collector_drand,
        fee_collector_drand_address: deps.api.addr_humanize(&config.fee_collector_drand_address)?.to_string()
    })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse{
        round: state.round,
        set_of_balls: state.set_of_balls,
        range_min: state.range.min,
        bonus_set_of_balls: state.bonus_set_of_balls,
        bonus_range_min: state.bonus_range.min,
        prize_rank: state.prize_rank,
        ticket_price: state.ticket_price,
        multiplier: state.multiplier,
        range_max: state.range.max,
        bonus_range_max: state.bonus_range.max
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Decimal, from_binary, Uint128};

    fn default_init(deps: DepsMut) {
        let msg = InstantiateMsg{
            denom: "uusd".to_string(),
            start_time: 0,
            frequency: 300,
            fee_collector: Decimal::from_str("0.05").unwrap(),
            fee_collector_address: "STAKING".to_string(),
            fee_collector_drand: Decimal::from_str("0.01").unwrap(),
            fee_collector_drand_address: "TERRAND".to_string(),
            set_of_balls: 17,
            range_min: 1,
            range_max: 17,
            bonus_set_of_balls: 1,
            bonus_range_min: 1,
            bonus_range_max: 8,
            prize_rank: vec![Uint128::from(1_000_000u128), Uint128::from(5_000_000u128), Uint128::from(10_000_000u128), Uint128::from(30_000_000u128), Uint128::from(50_000_000u128), Uint128::from(150_000_000u128), Uint128::from(1_000_000_000u128), Uint128::from(10_000_000_000u128)],
            ticket_price: vec![Uint128::from(1_000_000u128), Uint128::from(2_000_000u128), Uint128::from(5_000_000u128)],
            multiplier: vec![Decimal::from_str("1").unwrap(), Decimal::from_str("2").unwrap(), Decimal::from_str("5").unwrap()]
        };

        let res = instantiate(deps, mock_env(), mock_info("creator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        default_init(deps.as_mut());


    }

}
