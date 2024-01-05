mod transposition_table;
pub mod masks;

use std::collections::VecDeque;
use std::time::Duration;
use bitschess::prelude::*;
use transposition_table::{TranspositionTable, NodeKind};
use masks::PASSED_PAWN_MASK;

const MIN_DEPTH: i32 = 1;
const THINK_TIME_MS: u64 = 1_000;

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

const MAX_MOVE_EXTENSIONS: u8 = 15;
const DOUBLED_PAWN_PENALTY: i32 = 15; // applied per pawn a file. Doubled gets penalty applied twice and triple gets trice. 
const PASSED_PAWN_REWARD: i32 = 25;

pub struct GiffiBot {
    pub board: ChessBoard,

    iterations: u64,
    completed_depth: i32,
    search_cancelled: bool,
    pv: VecDeque<Move>,

    search_begin: std::time::Instant,
    transposition_table: TranspositionTable // TODO
}

impl GiffiBot {
    pub fn new() -> Self {
        Self {
            board: ChessBoard::new(),
            
            iterations: 0,
            search_cancelled: false,
            completed_depth: 0,
            pv: VecDeque::new(),

            search_begin: std::time::Instant::now(),
            transposition_table: TranspositionTable::new()
        }
    }

    pub fn evaluate(&self) -> i32 {
        let mut eval = 0i32;

        let mut all_pieces = self.board.side_bitboards[0].get_bits() | self.board.side_bitboards[1].get_bits();

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
                    positional_scoring = ROOK_POSITION[position as usize];
                }
                PieceType::Queen => {
                    positional_scoring = QUEEN_POSITION[position as usize];
                }
                PieceType::King => {
                    positional_scoring = KING_POSITION[position as usize];
                }
                _ => { positional_scoring = 0; }
            }

            if piece.is_black() {
                eval -= piece.get_piece_value() + positional_scoring;
            }
            else {
                eval += piece.get_piece_value() + positional_scoring;
            }
        }
        
        let perspective = if self.board.get_turn() == PieceColor::White {1} else {-1};
        return eval*perspective;
    }

    // r1bq1rk1/pp1p1ppp/2p2n2/2b5/3NP3/N1P2P2/PP4PP/R1BQK2R b KQ - 1 10 
    pub fn contains_multiple_pawns_this_file(&self, color: PieceColor, square: i32) -> bool {
        let file = BoardHelper::get_file(square);
        let mask = (A_FILE << file) ^ (1 << square);
        let pawns = self.board.bitboards[(color as usize) * 6].get_bits();
        (mask & pawns) != 0
    }

    pub fn is_passed_pawn(&self, color: PieceColor, square: i32) -> bool {
        let mask = PASSED_PAWN_MASK[color as usize][square as usize];
        let enemy_pawns = self.board.bitboards[(color.flipped() as usize) * 6].get_bits();
        (mask & enemy_pawns) == 0
    }

    fn search_all_captures(&mut self, mut alpha: i32, beta: i32) -> i32 {
        if self.search_cancelled && self.completed_depth >= MIN_DEPTH { return 0; }

        let mut eval = self.evaluate();
        if eval >= beta {
            return beta;
        }
        alpha = std::cmp::max(alpha, eval);

        let mut captures = self.board.get_legal_captures();
        self.order_moves(&mut captures);

        for m in captures {
            self.iterations += 1;
            self.board.make_move(m, true);
            eval = -self.search_all_captures(-beta, -alpha);
            self.board.unmake_move();
            
            if eval >= beta {
                return beta;
            }
            alpha = std::cmp::max(alpha, eval);
        }

        alpha
    }

    fn order_moves(&mut self, moves: &mut Vec<Move>) {
        let mut current_best = 0;
        for idx in 0..moves.len() {
            let m = moves.get(idx).unwrap();
            let move_piece = self.board.get_piece(m.get_from_idx());
            let capture_piece = self.board.get_piece(m.get_to_idx());

            let mut move_scope_quess = 0;
            
            if !capture_piece.is_none() {
                move_scope_quess = 10 * (capture_piece.get_piece_value() - move_piece.get_piece_value());
            }
            if m.get_flag() == MoveFlag::PromoteQueen {
                move_scope_quess = 10 * PieceType::Queen.get_value();
            }
            if current_best <= move_scope_quess {
                current_best = move_scope_quess;
                let m = moves.remove(idx);
                moves.insert(0, m);
            }
        }

        // Use the moves from the PV to order them on top.
        if let Some(pv_move) = self.pv.pop_front() {
            if let Some(position) = moves.iter().position(|m| m == &pv_move) {
                moves.remove(position);
                moves.insert(0, pv_move);
            }
        }
    }

    fn zw_search(&mut self, beta: i32, depth: i32) -> i32 {
        if self.search_cancelled && self.completed_depth >= MIN_DEPTH { return 0; }

        if depth == 0 { 
            return self.search_all_captures(beta-1, beta);
        }
        
        let mut moves = self.board.get_legal_moves();
        self.order_moves(&mut moves);
        for m in moves  {
            self.iterations += 1;
            self.board.make_move(m, true);
            let eval = -self.zw_search(1-beta, depth - 1);
            self.board.unmake_move();
            if eval >= beta {
                return beta;   // fail-hard beta-cutoff
            }
        }
        return beta-1; // fail-hard, return alpha
    }

    pub fn get_extension(&self, chess_move: Move, extension_count: u8) -> u8 {
        if extension_count > MAX_MOVE_EXTENSIONS {
            return 0;
        }

        if self.board.is_king_in_check(self.board.turn) {
            return 1;
        }
        if chess_move.get_flag() == MoveFlag::PromoteQueen {
            return 1;
        }

        0
    }
    
    // https://www.reddit.com/r/chessprogramming/comments/m2m048/how_does_a_triangular_pvtable_work/
    fn search(&mut self, mut alpha: i32, beta:i32, depth: i32, ply_from_root: i32, line: &mut VecDeque<Move>, extension_count: u8) -> i32 {
        if self.search_cancelled && self.completed_depth >= MIN_DEPTH { return 0; }

        if depth == 0 {
            line.clear();
            return self.search_all_captures(alpha, beta);
        }
        let mut moves = self.board.get_legal_moves();

        // Game Ended?
        if moves.len() == 0 {
            if self.board.is_king_in_check(self.board.get_turn()) {
                return -i32::MAX + ply_from_root; // adding the distance from root, favours a mate which is closer in moves.
            }
            return 0; // draw
        }

        self.order_moves(&mut moves);

        let mut pv = VecDeque::new();
        let mut do_pv_search = true;
        for m in moves {
            let extension = self.get_extension(m, extension_count);

            self.iterations += 1;
            self.board.make_move(m, true);
            let mut eval;
            if do_pv_search {
                eval = -self.search(-beta, -alpha, depth - 1 + (extension as i32), ply_from_root + 1, &mut pv, extension_count + extension);
                // give a little bonus for castling
                if m.get_flag() == MoveFlag::Castle { 
                    eval -= 80;
                }
            }
            else {
                // proof that the move is bad
                eval = -self.zw_search(-alpha, depth - 1);
                if eval > alpha {
                    eval = -self.search(-beta, -alpha, depth - 1 + (extension as i32), ply_from_root + 1, &mut pv, extension_count + extension);
                }
            }
            self.board.unmake_move();

            if self.search_cancelled && self.completed_depth >= MIN_DEPTH {
                return 0;
            }
            
            if eval >= beta {
                return beta;
            }
            if eval > alpha {
                do_pv_search = false;
                alpha = eval;
                pv.insert(0, m.clone());
            }
        }

        *line = pv;
        alpha
    }
    
    pub fn get_best_move(&mut self) -> Move {
        // Opening Book
        /*
        if self.full_moves < 12 {
            let fen = Fen::to_fen(self);
            
            if let Some(opening) = self.opening_book.get_opening(&fen) {
                if let Some(cmd) = opening.first() {
                    let moves = self.get_legal_moves();
                    let mut moves_filtered: Vec<Move> = moves.into_iter().filter(|m| &m.to_string() == cmd).collect();
                    if let Some(m) = moves_filtered.pop() {
                        println!("Chose a move from the opening book!");
                        return Some(m);
                    }
                }
            }
        }
        */

        // Rust thread ptr casting trickery to avoid using mutexes :D
        // TODO: Just learnt about async functions in rust, they might be the "correct answer here". Gotta test the performance on those.
        self.search_cancelled = false;

        let ptr: *mut bool = &mut self.search_cancelled;
        let ptr_casted = ptr as usize;
        std::thread::spawn(move || {
            let p = ptr_casted as *mut bool;
            std::thread::sleep(Duration::from_millis(THINK_TIME_MS));
            unsafe {
                *p = true;
            }
        });

        self.iterations = 0;
        self.search_begin = std::time::Instant::now();
        self.completed_depth = 0;
        let mut best_completed_line = VecDeque::new();

        for depth in 1..=64 {
            let mut line = VecDeque::new();
            let perspective = if self.board.get_turn() == PieceColor::White { 1 } else { -1 };
            let score = self.search(-i32::MAX, i32::MAX, depth, 0, &mut line, 0) * perspective;

            if self.search_cancelled && self.completed_depth >= MIN_DEPTH {
                break;
            }
            
            // if search was cancelled, the line is going to be incomplete
            best_completed_line = line;
            self.pv = best_completed_line.clone();
            self.completed_depth = depth;
            
            // Stats
            let end = std::time::Instant::now();
            let duration = end - self.search_begin;
            println!("info depth {} score cp {} currmove {} iterations {} duration_from_go {}", depth, score, self.pv.front().unwrap().to_uci(), self.iterations, duration.as_secs_f32()); 
        }
        self.pv = best_completed_line;

        self.pv.front().cloned().unwrap()
    }

}