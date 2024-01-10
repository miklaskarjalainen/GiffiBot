mod transposition_table;
pub mod masks;

use std::sync::{ Arc, atomic::{AtomicBool, Ordering}};
use std::collections::VecDeque;
use std::time::Duration;

use bitschess::prelude::*;
use transposition_table::{TranspositionTable, NodeKind};
use masks::PASSED_PAWN_MASK;

const MAX_DEPTH: i32 = 256; // there is no way we're reaching depth 256 in our lifetime :D

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
const ROOKS_CONNECTED_REWARD: i32 = 80;

#[derive(Debug, Clone)]
pub struct GiffiBot {
    pub board: ChessBoard,
    search_cancelled: Arc<AtomicBool>,

    iterations: u64,
    completed_depth: i32,
    pv: VecDeque<Move>,

    search_begin: std::time::Instant,
}

impl GiffiBot {
    pub fn new(board: ChessBoard, stop_search: Arc<AtomicBool>) -> Self {
        Self {
            board: board,
            
            iterations: 0,
            search_cancelled: stop_search,
            completed_depth: 0,
            pv: VecDeque::new(),

            search_begin: std::time::Instant::now(),
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
                    positional_scoring = ROOK_POSITION[position as usize];//+ bonus;
                }
                PieceType::Queen => {
                    positional_scoring = QUEEN_POSITION[position as usize]; //+ bonus;
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

    // TODO
    pub fn rooks_connected(&self, color: PieceColor, square: i32) -> bool {
        let color_index = (color as usize) * 6;

        let ally_rooks_and_queens = self.board.bitboards[color_index + 3].get_bits() | self.board.bitboards[color_index + 4].get_bits();
        let blockers = (self.board.side_bitboards[0].get_bits() | self.board.side_bitboards[1].get_bits()) ^ ally_rooks_and_queens;
        let mask = magics::get_rook_magic(square, blockers);
        (mask & ally_rooks_and_queens) != 0
    }

    fn search_all_captures(&mut self, mut alpha: i32, beta: i32) -> i32 {
        if self.search_cancelled.load(Ordering::Relaxed) {
            return 0;
        }

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
        if self.search_cancelled.load(Ordering::Relaxed) {
            return 0;
        }

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
        if self.search_cancelled.load(Ordering::Relaxed) {
            return 0;
        }

        if depth == 0 {
            line.clear();
            return self.search_all_captures(alpha, beta);
        }

        if self.board.is_draw() {
            return 0;
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

            if self.search_cancelled.load(Ordering::Relaxed) {
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
    
    /// Calculates until search_cancelled is set to true
    #[inline(always)]
    pub fn calculate(&mut self) {
        self.calculate_depth(MAX_DEPTH);
    }

    #[inline(always)]
    pub fn calculate_time(&mut self, time: std::time::Duration) {
        const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);

        let copy_cancel = Arc::clone(&self.search_cancelled);

        let handle = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            
            loop {
                std::thread::sleep(CHECK_INTERVAL);
                
                // already set to true, before time ran out (most likely user manually called 'stop')
                if copy_cancel.load(Ordering::Relaxed) {
                    break;
                }

                // slept for the target amount
                let slept_for = std::time::Instant::now() - start;
                if slept_for >= time {
                    copy_cancel.store(true, Ordering::Relaxed);
                    break;
                }
            }
        });

        self.calculate_depth(MAX_DEPTH);
        let _ = handle.join();
    }

    pub fn calculate_depth(&mut self, depth: i32) {
        self.iterations = 0;
        self.search_begin = std::time::Instant::now();
        self.completed_depth = 0;
        let mut best_completed_line = VecDeque::new();

        for depth in 1..=depth {
            let mut line = VecDeque::new();
            let perspective = if self.board.get_turn() == PieceColor::White { 1 } else { -1 };
            let score = self.search(-i32::MAX, i32::MAX, depth, 0, &mut line, 0) * perspective;

            if self.search_cancelled.load(Ordering::Relaxed) {
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

        println!("bestmove {}", self.pv.front().cloned().unwrap().to_uci());
    }

}