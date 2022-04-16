use crate::state::{Game, GameStats, GAMES};
use crate::ContractError;
use cosmwasm_std::{CanonicalAddr, Decimal, Storage};

pub fn bonus_number(number: &char) -> Result<u8, ContractError> {
    let bonus_number = match number {
        '0' => 1u8,
        '1' => 1u8,
        '2' => 2u8,
        '3' => 2u8,
        '4' => 3u8,
        '5' => 3u8,
        '6' => 4u8,
        '7' => 4u8,
        '8' => 5u8,
        '9' => 5u8,
        'a' => 6u8,
        'b' => 6u8,
        'c' => 7u8,
        'd' => 7u8,
        'e' => 8u8,
        'f' => 8u8,
        _ => return Err(ContractError::Unauthorized {}),
    };

    Ok(bonus_number)
}

pub fn winning_number(number: Vec<char>, set_of_balls: u8) -> Result<Vec<u8>, ContractError> {
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
            _ => return Err(ContractError::Unauthorized {}),
        };

        if
        /* !winning_number.contains(&number) && */
        winning_number.len() != set_of_balls as usize {
            winning_number.push(number);
        }
    }

    Ok(winning_number)
}

/*
   Deprecated since non deterministic suspicion
*/
// pub fn random_number(randomness_hash: String, set_of_balls: u8, range_max: u8) -> Vec<u8>{
//
//     let mut winning_numbers: Vec<u8> = vec![];
//
//     let mut counter = 0;
//     while winning_numbers.len() != 8 {
//         let mut new_hash = format!("{}",counter.to_string());
//         new_hash.push_str(&randomness_hash);
//
//         let mut arr = [0u8; 8];
//         for (place, element) in arr.iter_mut().zip(
//             /*hash.iter()*/ Sha256::digest(new_hash.as_bytes()).iter(),
//         ) {
//             *place = *element;
//         }
//
//         let number = u64::from_be_bytes(arr) % range_max.checked_sub(1).unwrap() as u64;
//         let number_to_u8 = number as u8;
//         if !winning_numbers.contains(&number_to_u8.checked_add(1).unwrap()){
//             winning_numbers.push(number.checked_add(1).unwrap() as u8)
//         }
//         counter +=1;
//     }
//
//
//     winning_numbers
// }
pub fn save_game(
    storage: &mut dyn Storage,
    round: u64,
    address_raw: &CanonicalAddr,
    numbers: Vec<u8>,
    multiplier: Decimal,
    game: Option<GameStats>,
) -> Result<(), ContractError> {
    let stats = match game {
        None => 0,
        Some(game_stats) => game_stats.total_ticket,
    };

    // for number in numbers {
    GAMES.save(
        storage,
        (
            &round.to_be_bytes(),
            address_raw.as_slice(),
            &stats.to_be_bytes(),
        ),
        &Game {
            number: vec![numbers[0], numbers[1], numbers[2], numbers[3], numbers[4]],
            bonus: numbers[5],
            multiplier,
            resolved: false,
        },
    )?;
    //stats += 1;
    // }
    Ok(())
}

pub fn count_match(game: &[u8], lottery: &[u8], set_of_balls: u8) -> u8 {
    let mut count = 0;

    for i in 0..set_of_balls as usize {
        if lottery.contains(&game[i]) {
            count += 1
        }
        // if game[i] == lottery[i] {
        //     count += 1
        // }
    }

    count
}
