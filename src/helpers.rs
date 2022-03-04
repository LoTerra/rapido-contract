use crate::state::{Game, GameStats, GAMES, GAMES_STATS};
use crate::ContractError;
use cosmwasm_std::{CanonicalAddr, Decimal, Storage};

// There is probably some built-in function for this, but this is a simple way to do it
pub fn is_lower_hex(combination: &str, len: u8) -> bool {
    if combination.len() != (len as usize) {
        return false;
    }
    if !combination
        .chars()
        .all(|c| ('a'..='f').contains(&c) || ('0'..='9').contains(&c))
    {
        return false;
    }
    true
}

pub fn save_game(
    storage: &mut dyn Storage,
    round: u64,
    address_raw: &CanonicalAddr,
    numbers: Vec<Vec<u8>>,
    multiplier: Decimal,
    game: Option<GameStats>,
) -> Result<(), ContractError> {
    let mut stats = match game {
        None => 0,
        Some(game_stats) => game_stats.total_ticket,
    };

    for number in numbers {
        GAMES.save(
            storage,
            (
                &round.to_be_bytes(),
                address_raw.as_slice(),
                &stats.to_be_bytes(),
            ),
            &Game {
                number: vec![number[0], number[1], number[2], number[3]],
                bonus: number[4],
                multiplier,
                resolved: false,
            },
        )?;
        stats += 1;
    }

    Ok(())
}
