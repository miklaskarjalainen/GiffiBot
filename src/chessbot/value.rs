
use bitschess::prelude::*;
use super::GiffiBot;
use super::masks::PASSED_PAWN_MASK;

const DOUBLED_PAWN_PENALTY: i32 = 15; // applied per pawn a file. Doubled gets penalty applied twice and triple gets trice. 
const PASSED_PAWN_REWARD: i32 = 25;
const ROOKS_CONNECTED_REWARD: i32 = 80;
const PAWN_POSITION: [i32; 64] = [
    0,  0, 0, 0, 0, 0, 0, 0,
    100, 100, 100, 100, 100, 100, 100, 100,
    20, 10, 40, 60, 60, 40, 20, 20,
    5,  5, 25, 40, 40, 25, 5,  5,
    0, 0, 0, 35, 35, 0, 0, 0,
    5, -5,-10, 0, 0,-10, -5, 5,
    5, 10, 10,-20,-20, 10, 10, 5,
    0, 0, 0, 0, 0, 0, 0, 0,
];

const KNIGHT_POSITION: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

const BISHOP_POSITION: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10, 30,  5, 10, 10,  5,  30,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

const ROOK_POSITION: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    0,  0,  0,  25,  25,  0,  0,  0
];

const QUEEN_POSITION: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
    -5,  0,  5,  5,  5,  5,  0, -5,
    -5,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];

const KING_POSITION: [i32; 64] = [
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -20, -30, -30, -40, -40, -30, -30, -20,
    -10, -20, -20, -20, -20, -20, -20, -10,
    20,  20,   0,   0,   0,   0,  20,  20,
    20,  30,  10,   0,   0,  10,  30,  20,
];

const KING_POSITION_END: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20,
    -5,    0,   5,   5,   5,   5,   0,  -5,
    -10,  -5,  20,  30,  30,  20,  -5, -10,
    -15, -10,  35,  45,  45,  35, -10, -15,
    -20, -15,  30,  40,  40,  30, -15, -20,
    -25, -20,  25,  25,  25,  20, -20, -25,
    -30, -25,   0,   0,   0,   0, -25, -30,
    -50, -30, -30, -30, -30, -30, -30, -50
];

const CENTER_MANHATTAN_DISTANCE: [i32; 64] = [
    6, 5, 4, 3, 3, 4, 5, 6,
    5, 4, 3, 2, 2, 3, 4, 5,
    4, 3, 2, 1, 1, 2, 3, 4,
    3, 2, 1, 0, 0, 1, 2, 3,
    3, 2, 1, 0, 0, 1, 2, 3,
    4, 3, 2, 1, 1, 2, 3, 4,
    5, 4, 3, 2, 2, 3, 4, 5,
    6, 5, 4, 3, 3, 4, 5, 6
];


#[must_use]
#[inline(always)]
pub const fn get_piece_value(piece_type: PieceType) -> i32 {
    const PIECE_VALUE:[i32; 7] = [0, 100, 300, 320, 500, 900, 0];
    PIECE_VALUE[piece_type as usize]
}

impl GiffiBot {
    pub fn evaluate(&self) -> i32 {
        let mut eval = 0i32;

        let mut all_pieces = self.board.side_bitboards[0] | self.board.side_bitboards[1];
        let end_game = self.is_end_game();

        while all_pieces != 0 {
            let square = BoardHelper::bitscan_forward(all_pieces);
            all_pieces ^= 1u64 << square;

            let piece = self.board.get_piece(square);
            let position = if piece.get_color() == PieceColor::Black { square } else { 63-square };
            let positional_scoring;

            match piece.get_piece_type() {
                PieceType::Pawn => {
                    let color = piece.get_color();
                    let penalty = if self.contains_multiple_pawns_this_file(color, square) { DOUBLED_PAWN_PENALTY } else { 0 };
                    let passed = if self.is_passed_pawn(color, square) {PASSED_PAWN_REWARD} else { 0 };
                    
                    positional_scoring = PAWN_POSITION[position as usize] + passed - penalty;
                }
                PieceType::Knight => {
                    positional_scoring = KNIGHT_POSITION[position as usize];
                }
                PieceType::Bishop => {
                    positional_scoring = BISHOP_POSITION[position as usize];
                }
                PieceType::Rook => {
                    positional_scoring = ROOK_POSITION[position as usize];//+ bonus;
                }
                PieceType::Queen => {
                    positional_scoring = QUEEN_POSITION[position as usize]; //+ bonus;
                }
                PieceType::King => {
                    if !end_game {
                        positional_scoring = KING_POSITION[position as usize];
                    }
                    else {
                        // In endgames, prefer having king in the middle and forcing the enemy king into the corner or edge.
                        positional_scoring = KING_POSITION_END[position as usize] + CENTER_MANHATTAN_DISTANCE[self.board.get_king_square(piece.get_color().flipped()) as usize] * 10;
                    }
                }
                _ => { positional_scoring = 0; }
            }

            if piece.is_black() {
                eval -= get_piece_value(piece.get_piece_type()) + positional_scoring;
            }
            else {
                eval += get_piece_value(piece.get_piece_type()) + positional_scoring;
            }
        }

        let perspective = if self.board.get_turn() == PieceColor::White {1} else {-1};
        return eval*perspective;
    }

    #[inline(always)]
    pub fn contains_multiple_pawns_this_file(&self, color: PieceColor, square: i32) -> bool {
        let file = BoardHelper::get_file(square);
        let mask = (A_FILE << file) ^ (1 << square);
        let pawns = self.board.bitboards[(color as usize) * 6];
        (mask & pawns) != 0
    }

    #[inline(always)]
    pub fn is_passed_pawn(&self, color: PieceColor, square: i32) -> bool {
        let mask = PASSED_PAWN_MASK[color as usize][square as usize];
        let enemy_pawns = self.board.bitboards[(color.flipped() as usize) * 6];
        (mask & enemy_pawns) == 0
    }

    // TODO
    pub fn rooks_connected(&self, color: PieceColor, square: i32) -> bool {
        let color_index = (color as usize) * 6;

        let ally_rooks_and_queens = self.board.bitboards[color_index + 3] | self.board.bitboards[color_index + 4];
        let blockers = (self.board.side_bitboards[0] | self.board.side_bitboards[1]) ^ ally_rooks_and_queens;
        let mask = magics::get_rook_magic(square, blockers);
        (mask & ally_rooks_and_queens) != 0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};

    #[test]
    fn is_passed_pawn_test1() {
        let mut board = ChessBoard::new();
        board.parse_fen("k7/3p3p/8/2p5/2P5/8/5P2/K7 w - - 0 1").expect("valid fen");

        let stop = Arc::new(AtomicBool::new(false));
        let bot = GiffiBot::new(board, stop);
        
        assert_eq!(bot.is_passed_pawn(PieceColor::White, Square::F2 as i32), true);
        assert_eq!(bot.is_passed_pawn(PieceColor::White, Square::C4 as i32), false);

        assert_eq!(bot.is_passed_pawn(PieceColor::Black, Square::D7 as i32), false);
        assert_eq!(bot.is_passed_pawn(PieceColor::Black, Square::C5 as i32), false);
        assert_eq!(bot.is_passed_pawn(PieceColor::Black, Square::H2 as i32), true);
    }

    #[test]
    fn contains_multiple_pawns_this_file_test1() {
        let mut board = ChessBoard::new();
        board.parse_fen("k7/4ppp1/5pp1/8/8/5P2/4PPP1/K7 w - - 0 1").expect("valid fen");

        let stop = Arc::new(AtomicBool::new(false));
        let bot = GiffiBot::new(board, stop);
        
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::White, Square::E2 as i32), false);
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::White, Square::F2 as i32), true);
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::White, Square::F3 as i32), true);
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::White, Square::G2 as i32), false);
        
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::Black, Square::E7 as i32), false);
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::Black, Square::F6 as i32), true);
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::Black, Square::F7 as i32), true);
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::Black, Square::G6 as i32), true);
        assert_eq!(bot.contains_multiple_pawns_this_file(PieceColor::Black, Square::G7 as i32), true);
    }

}
