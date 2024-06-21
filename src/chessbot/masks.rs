use lazy_static::lazy_static;

use bitschess::prelude::*;


lazy_static! {
    pub static ref PASSED_PAWN_MASK: [[u64; 64]; 2] = {
        let mut map = [[0; 64]; 2];
        for square in 0..64 {
            map[0][square] = generate_passed_pawn_mask(PieceColor::White, square as i32);
            map[1][square] = generate_passed_pawn_mask(PieceColor::Black, square as i32);
        }
        map
    };

    pub static ref MANHATTAN_DISTANCE: [[i32; 64]; 64] = {
        let mut map = [[0; 64]; 64];
        for from in 0..64 {
            for to in 0..64 {
                map[from as usize][to as usize] = manhattan_distance(from, to);
            }
        }
        map
    };
}


fn generate_passed_pawn_mask(color: PieceColor, square: i32) -> u64 {
    let rank = BoardHelper::get_rank(square) as u32;
    let file = BoardHelper::get_file(square);
    
    let mut file_mask = A_FILE << file;
    if file != 7 {
        file_mask |= A_FILE << (file+1);
    }
    if file != 0 {
        file_mask |= A_FILE << (file-1);
    }
    
    
    let rank_mask: u64;
    if color == PieceColor::White {
        rank_mask = (!0u64).wrapping_shl((rank+1)*8);
    }
    else {
        rank_mask = (!0u64).wrapping_shr((7 - rank + 1)*8);
    }

    rank_mask & file_mask
}

// https://www.chessprogramming.org/Manhattan-Distance
fn manhattan_distance(from_square: i32, to_square: i32) -> i32 {
    let file1 = BoardHelper::get_file(from_square);
    let file2 = BoardHelper::get_file(to_square);
    let rank1 = BoardHelper::get_rank(from_square);
    let rank2 = BoardHelper::get_rank(to_square);

    let rank_distance = rank2.wrapping_sub(rank1).abs();
    let file_distance = file2.wrapping_sub(file1).abs();
    rank_distance + file_distance
}

