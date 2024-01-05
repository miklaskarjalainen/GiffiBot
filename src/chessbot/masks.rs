use lazy_static::lazy_static;

use bitschess::chessboard::piece::PieceColor;
use bitschess::chessboard::bitboard::*;
use bitschess::chessboard::board_helper::BoardHelper;


lazy_static! {
    pub static ref PASSED_PAWN_MASK: [[u64; 64]; 2] = {
        let mut map = [[0; 64]; 2];
        for square in 0..64 {
            map[0][square] = generate_passed_pawn_mask(PieceColor::White, square as i32);
            map[1][square] = generate_passed_pawn_mask(PieceColor::Black, square as i32);
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


