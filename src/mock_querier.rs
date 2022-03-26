use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_slice, to_binary, Addr, Api, BankQuery, Binary, Coin, ContractResult, Decimal, OwnedDeps,
    Querier, QuerierResult, QueryRequest, Response, StdError, StdResult, SystemError, SystemResult,
    Uint128, WasmQuery,
};

use serde::Serialize;
use std::str::FromStr;
use terra_cosmwasm::{
    ExchangeRateItem, ExchangeRatesResponse, TaxCapResponse, TaxRateResponse, TerraQuery,
    TerraQueryWrapper, TerraRoute,
};
//pub const MOCK_HUB_CONTRACT_ADDR: &str = "hub";
//pub const MOCK_CW20_CONTRACT_ADDR: &str = "lottery";
//pub const MOCK_REWARD_CONTRACT_ADDR: &str = "reward";
//pub const MOCK_TOKEN_CONTRACT_ADDR: &str = "token";

pub fn custom_mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}
#[derive(Clone, Default, Serialize)]
pub struct HolderResponse {
    pub address: String,
    pub balance: Uint128,
    pub index: Decimal,
    pub pending_rewards: Decimal,
}

impl HolderResponse {
    pub fn new(address: String, balance: Uint128, index: Decimal, pending_rewards: Decimal) -> Self {
        HolderResponse { address, balance, index, pending_rewards }
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    holder: HolderResponse,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                println!("{}", contract_addr);
                println!("{:?}", msg);
                if contract_addr == &"TERRAND".to_string() {
                    let msg_terrand = terrand::msg::GetRandomResponse {
                        randomness: Binary::from(
                            "OdRl+j6PHnN84dy12n4Oq1BrGktD73FW4SKPihxfB9I=".as_bytes(),
                        ),
                        worker: "worker".to_string(),
                    };
                    return SystemResult::Ok(ContractResult::Ok(to_binary(&msg_terrand).unwrap()));
                }else if contract_addr == &"STAKING".to_string() {
                    let msg_holder = HolderResponse {
                        address: "0".to_string(),
                        balance: self.holder.balance,
                        index: self.holder.index,
                        pending_rewards: self.holder.pending_rewards
                    };
                    return SystemResult::Ok(ContractResult::Ok(to_binary(&msg_holder).unwrap()));
                }
                panic!("DO NOT ENTER HERE")
            }
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => match query_data {
                TerraQuery::TaxRate {} => {
                    let res = TaxRateResponse {
                        rate: Decimal::percent(1),
                    };
                    SystemResult::Ok(ContractResult::from(to_binary(&res)))
                }
                TerraQuery::TaxCap { denom: _ } => {
                    let cap = Uint128::from(1000000u128);
                    let res = TaxCapResponse { cap };
                    SystemResult::Ok(ContractResult::from(to_binary(&res)))
                }
                _ => panic!("DO NOT ENTER HERE"),
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            holder: HolderResponse::default()
        }
    }
    pub fn set_holder(&mut self, address: String, balance: Uint128, index: Decimal, pending_rewards: Decimal) {
        self.holder = HolderResponse::new(address, balance, index, pending_rewards);
    }
}
