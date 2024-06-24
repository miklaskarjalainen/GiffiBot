pub mod go;
pub mod masks;
pub mod value;

mod transposition_table;

use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::collections::VecDeque;

use bitschess::prelude::*;
use transposition_table::{TranspositionTable, NodeKind};

const MAX_MOVE_EXTENSIONS: u8 = 15;

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

    pub const fn is_end_game(&self) -> bool {
        let bishops = self.board.bitboards[PieceType::Bishop.get_side_index(PieceColor::White)] | self.board.bitboards[PieceType::Bishop.get_side_index(PieceColor::Black)];
        let rooks   = self.board.bitboards[PieceType::Rook  .get_side_index(PieceColor::White)] | self.board.bitboards[PieceType::Rook  .get_side_index(PieceColor::Black)];
        let queens  = self.board.bitboards[PieceType::Queen .get_side_index(PieceColor::White)] | self.board.bitboards[PieceType::Queen .get_side_index(PieceColor::Black)];

        // Trigger under 4 rooks
        const MATERIAL_4_ROOKS: i32 = value::get_piece_value(PieceType::Rook) * 4;
        let material_count =
            (bishops.count_ones() as i32 * value::get_piece_value(PieceType::Bishop)) +
            (rooks.count_ones() as i32 * value::get_piece_value(PieceType::Rook)) +
            (queens.count_ones() as i32 * value::get_piece_value(PieceType::Queen));
        material_count < MATERIAL_4_ROOKS
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
            let _ = self.board.unmake_move();
            
            if eval >= beta {
                return beta;
            }
            alpha = std::cmp::max(alpha, eval);
        }

        alpha
    }

    fn order_moves(&mut self, moves: &mut MoveContainer) {
        // if PV used, keep PV first and order the moves starting from the second index.
        let start_index = !self.pv.is_empty() as usize;
        if let Some(pv_move) = self.pv.pop_front() {
            if let Some(position) = moves.iter().position(|m| m == &pv_move) {
                unsafe { moves.swap_unchecked(0, position) };
            }
        }
        
        let mut current_best = 0;
        for idx in start_index..moves.len() {
            let m = unsafe { moves.get_unchecked(idx) };
            let move_piece = self.board.get_piece(m.get_from_idx());
            let capture_piece = self.board.get_piece(m.get_to_idx());
            
            let mut move_scope_guess = 0;
            
            if !capture_piece.is_none() {
                move_scope_guess = value::get_piece_value(capture_piece.get_piece_type()) - value::get_piece_value(move_piece.get_piece_type());
            }
            if m.get_flag() == MoveFlag::PromoteQueen {
                move_scope_guess = value::get_piece_value(PieceType::Queen);
            }

            // perform a swap
            if current_best <= move_scope_guess {
                current_best = move_scope_guess;
                unsafe {
                    moves.swap_unchecked(start_index, idx);
                }
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
            let _ = self.board.unmake_move();
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
            let _ = self.board.unmake_move();

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
}