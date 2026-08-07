use bitschess::{prelude::ChessBoard, Move, MoveContainer, MoveFlag, PieceType};

use super::value::get_piece_value;
use crate::chessbot::transposition_table::TranspositionTable;

pub struct MoveOrdering;

impl MoveOrdering {
    pub fn order_moves(
        board: &ChessBoard,
        tt: &TranspositionTable,
        moves: &mut MoveContainer,
        pv_move: Option<Move>,
        killer_move: Move,
    ) {
        if moves.is_empty() {
            return;
        }

        let hash_move = tt.get_entry_by_hash(board.zobrist_hash);
        let mut scores = [0i32; 218];

        // Higher score -> will be searched first
        let mut i = 0;
        for m in moves.iter() {
            if Some(*m) == pv_move {
                scores[i] = 5000;
                i += 1;
                continue;
            }
            if m == &hash_move {
                scores[i] = 4500;
                i += 1;
                continue;
            }
            if m == &killer_move {
                scores[i] += 70;
                i += 1;
                continue;
            }

            let mut move_score = 0i32;
            let move_piece = board.get_piece(m.get_from_idx());
            let capture_piece = board.get_piece(m.get_to_idx());
            let move_type = move_piece.get_piece_type();

            if !capture_piece.is_none() {
                move_score = get_piece_value(capture_piece.get_piece_type()) * 16
                    - get_piece_value(move_type);
            }

            if m.get_flag() == MoveFlag::PromoteQueen {
                move_score += get_piece_value(PieceType::Queen);
            }
            if move_type == PieceType::King {
                move_score -= 5;
            }

            scores[i] = move_score;
            i += 1;
        }

        Self::quick_sort(moves, &mut scores, 0, moves.len() - 1);
    }

    fn partition(moves: &mut MoveContainer, scores: &mut [i32], low: usize, high: usize) -> usize {
        let pivot = high;
        let mut i = low as isize - 1;

        for j in low..high {
            if scores[j] > scores[pivot] {
                i += 1;
                moves.swap(i as usize, j);
                scores.swap(i as usize, j);
            }
        }
        moves.swap((i + 1) as usize, pivot);
        scores.swap((i + 1) as usize, pivot);
        (i + 1) as usize
    }

    fn quick_sort(moves: &mut MoveContainer, scores: &mut [i32], low: usize, high: usize) {
        if low < high {
            let pivot = Self::partition(moves, scores, low, high);
            if pivot > 0 {
                Self::quick_sort(moves, scores, low, pivot - 1)
            }
            Self::quick_sort(moves, scores, pivot + 1, high);
        }
    }
}
