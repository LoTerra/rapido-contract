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

pub fn bonus_number(number: &char) -> Result<u8, ContractError>  {
    let bonus_number = match number {
        '0' => 1u8,
        '1' => 1u8,
        '2' => 2u8,
        '3' => 2u8,
        '4' => 3u8,
        '5' => 3u8,
        '6' => 4u8,
        '7' => 4u8,
        '8' => 1u8,
        '9' => 1u8,
        'a' => 2u8,
        'b' => 2u8,
        'c' => 3u8,
        'd' => 3u8,
        'e' => 4u8,
        'f' => 4u8,
        _ => {return Err(ContractError::Unauthorized {})}
    };

    Ok(bonus_number)
}

pub fn winning_number(number: Vec<char>) -> Result<Vec<u8>, ContractError>  {

    let mut winning_number: Vec<u8> = vec![];
    for n in number {
        let number = match n {
            '1' => 1u8,
            '2' => 2u8,
            '3' => 3u8,
            '4' => 4u8,
            '5' => 5u8,
            '6' => 6u8,
            '7' => 7u8,
            '8' => 8u8,
            '9' => 9u8,
            'a' => 10u8,
            'b' => 11u8,
            'c' => 12u8,
            'd' => 13u8,
            'e' => 14u8,
            'f' => 15u8,
            '0' => 16u8,
            _ => {return Err(ContractError::Unauthorized {})}
        };

        if !winning_number.contains(&number) && winning_number.len() != 4 {
            winning_number.push(number);
        }
    }

    Ok(winning_number)
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
