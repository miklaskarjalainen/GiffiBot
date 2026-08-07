pub mod go;
pub mod masks;
pub mod value;

mod transposition_table;
use transposition_table::{NodeKind, TranspositionTable};

use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use bitschess::prelude::*;

const MAX_MOVE_EXTENSIONS: u8 = 15;

pub(crate) const MATE: i32 = 30_000;
pub(crate) const MATE_THRESHOLD: i32 = MATE - 1000;

#[derive(Debug, Clone)]
pub struct GiffiBot {
    pub board: ChessBoard,
    search_cancelled: Arc<AtomicBool>,

    iterations: u64,
    completed_depth: i32,
    pub pv: VecDeque<Move>,

    tt: TranspositionTable,

    search_begin: std::time::Instant,
}

impl GiffiBot {
    pub fn new(board: ChessBoard, stop_search: Arc<AtomicBool>) -> Self {
        Self {
            board,

            iterations: 0,
            search_cancelled: stop_search,
            completed_depth: 0,
            pv: VecDeque::new(),

            tt: TranspositionTable::new(),

            search_begin: std::time::Instant::now(),
        }
    }

    pub const fn is_end_game(&self) -> bool {
        let bishops = self.board.bitboards[PieceType::Bishop.get_side_index(PieceColor::White)]
            | self.board.bitboards[PieceType::Bishop.get_side_index(PieceColor::Black)];
        let rooks = self.board.bitboards[PieceType::Rook.get_side_index(PieceColor::White)]
            | self.board.bitboards[PieceType::Rook.get_side_index(PieceColor::Black)];
        let queens = self.board.bitboards[PieceType::Queen.get_side_index(PieceColor::White)]
            | self.board.bitboards[PieceType::Queen.get_side_index(PieceColor::Black)];

        // Trigger under 4 rooks
        const MATERIAL_4_ROOKS: i32 = value::get_piece_value(PieceType::Rook) * 4;
        let material_count = (bishops.count_ones() as i32
            * value::get_piece_value(PieceType::Bishop))
            + (rooks.count_ones() as i32 * value::get_piece_value(PieceType::Rook))
            + (queens.count_ones() as i32 * value::get_piece_value(PieceType::Queen));
        material_count < MATERIAL_4_ROOKS
    }

    fn search_all_captures(&mut self, mut alpha: i32, beta: i32, cancellable: bool) -> i32 {
        if cancellable && self.search_cancelled.load(Ordering::Relaxed) {
            return 0;
        }

        let mut eval = self.evaluate();
        if eval >= beta {
            return beta;
        }
        alpha = std::cmp::max(alpha, eval);

        let mut captures = self.board.get_legal_captures();
        self.order_moves(&mut captures, Move(0));

        for m in captures {
            self.iterations += 1;
            self.board.make_move(m, true);
            eval = -self.search_all_captures(-beta, -alpha, cancellable);
            let _ = self.board.unmake_move();

            if eval >= beta {
                return beta;
            }
            alpha = std::cmp::max(alpha, eval);
        }

        alpha
    }

    fn order_moves(&mut self, moves: &mut MoveContainer, hash_move: Move) {
        // search the hash move first, then the PV move, then the rest ordered by capture value.
        let mut start_index = 0;
        if hash_move != Move(0) {
            if let Some(position) = moves.iter().position(|m| m == &hash_move) {
                unsafe { moves.swap_unchecked(0, position) };
                start_index = 1;
            }
        }
        if let Some(pv_move) = self.pv.pop_front() {
            if start_index < moves.len() {
                if let Some(position) = moves.iter().position(|m| m == &pv_move) {
                    unsafe { moves.swap_unchecked(start_index, position) };
                    start_index += 1;
                }
            }
        }

        let mut current_best = 0;
        for idx in start_index..moves.len() {
            let m = unsafe { moves.get_unchecked(idx) };
            let move_piece = self.board.get_piece(m.get_from_idx());
            let capture_piece = self.board.get_piece(m.get_to_idx());

            let mut move_scope_guess = 0;

            if !capture_piece.is_none() {
                move_scope_guess = value::get_piece_value(capture_piece.get_piece_type())
                    - value::get_piece_value(move_piece.get_piece_type());
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

    fn zw_search(&mut self, beta: i32, depth: i32, ply_from_root: i32, cancellable: bool) -> i32 {
        if self.search_cancelled.load(Ordering::Relaxed) {
            return 0;
        }

        let hash = self.board.zobrist_hash;

        if depth > 0 {
            if let Some((score, _)) = self.tt.probe_hash(
                hash,
                depth,
                (beta - 1).saturating_add(ply_from_root),
                beta.saturating_add(ply_from_root),
            ) {
                return score.saturating_sub(ply_from_root);
            }
        }
        if depth == 0 {
            return self.search_all_captures(beta - 1, beta, cancellable);
        }

        if self.board.is_draw() {
            return 0;
        }

        let mut moves = self.board.get_legal_moves();
        if moves.is_empty() {
            if self.board.is_king_in_check(self.board.get_turn()) {
                return -MATE + ply_from_root;
            }
            return 0; // draw
        }

        let hash_move = self.tt.get_entry_by_hash(hash);
        self.order_moves(&mut moves, hash_move);
        let mut best_move = Move(0);
        for m in moves {
            self.iterations += 1;
            self.board.make_move(m, true);
            let eval = -self.zw_search(1 - beta, depth - 1, ply_from_root + 1, cancellable);
            let _ = self.board.unmake_move();
            if eval >= beta {
                best_move = m;
                self.tt.store_evaluation(
                    NodeKind::LowerBound,
                    hash,
                    depth,
                    beta.saturating_add(ply_from_root),
                    best_move,
                );
                return beta; // fail-hard beta-cutoff
            }
        }
        self.tt.store_evaluation(
            NodeKind::UpperBound,
            hash,
            depth,
            (beta - 1).saturating_add(ply_from_root),
            best_move,
        );
        beta - 1 // fail-hard, return alpha
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
    fn search(
        &mut self,
        mut alpha: i32,
        beta: i32,
        depth: i32,
        ply_from_root: i32,
        line: &mut VecDeque<Move>,
        extension_count: u8,
        cancellable: bool,
    ) -> i32 {
        if cancellable && self.search_cancelled.load(Ordering::Relaxed) {
            return 0;
        }

        let original_alpha = alpha;
        let hash = self.board.zobrist_hash;

        if depth > 0 {
            if let Some((score, tt_move)) = self.tt.probe_hash(
                hash,
                depth,
                alpha.saturating_add(ply_from_root),
                beta.saturating_add(ply_from_root),
            ) {
                if tt_move != Move(0) {
                    line.push_front(tt_move);
                }
                return score.saturating_sub(ply_from_root);
            }
        }

        if depth == 0 {
            line.clear();
            return self.search_all_captures(alpha, beta, cancellable);
        }

        if self.board.is_draw() {
            return 0;
        }

        let mut moves = self.board.get_legal_moves();
        // Game Ended?
        if moves.is_empty() {
            if self.board.is_king_in_check(self.board.get_turn()) {
                return -MATE + ply_from_root; // adding the distance from root, favours a mate which is closer in moves.
            }
            return 0; // draw
        }

        let hash_move = self.tt.get_entry_by_hash(hash);
        self.order_moves(&mut moves, hash_move);

        let mut best_move = Move(0);
        let mut pv = VecDeque::new();
        let mut do_pv_search = true;
        for m in moves.iter() {
            let extension = self.get_extension(*m, extension_count);

            self.iterations += 1;
            self.board.make_move(*m, true);
            let mut eval;
            if do_pv_search {
                pv.clear();
                eval = -self.search(
                    -beta,
                    -alpha,
                    depth - 1 + (extension as i32),
                    ply_from_root + 1,
                    &mut pv,
                    extension_count + extension,
                    cancellable,
                );
                // give a little bonus for castling
                if m.get_flag() == MoveFlag::Castle {
                    eval -= 80;
                }
            } else {
                // proof that the move is bad
                eval = -self.zw_search(-alpha, depth - 1, ply_from_root + 1, cancellable);
                if eval > alpha {
                    let mut re_pv = VecDeque::new();
                    eval = -self.search(
                        -beta,
                        -alpha,
                        depth - 1 + (extension as i32),
                        ply_from_root + 1,
                        &mut re_pv,
                        extension_count + extension,
                        cancellable,
                    );
                    if eval > alpha {
                        pv = re_pv;
                    }
                }
            }
            let _ = self.board.unmake_move();

            if self.search_cancelled.load(Ordering::Relaxed) {
                return 0;
            }

            if eval >= beta {
                best_move = *m;
                self.tt.store_evaluation(
                    NodeKind::LowerBound,
                    hash,
                    depth,
                    beta.saturating_add(ply_from_root),
                    best_move,
                );
                return beta;
            }
            if eval > alpha {
                do_pv_search = false;
                alpha = eval;
                best_move = *m;
                pv.insert(0, *m);
            }
        }

        let kind = if alpha > original_alpha {
            NodeKind::Exact
        } else {
            NodeKind::UpperBound
        };
        self.tt.store_evaluation(
            kind,
            hash,
            depth,
            alpha.saturating_add(ply_from_root),
            best_move,
        );

        *line = pv;
        alpha
    }
}
