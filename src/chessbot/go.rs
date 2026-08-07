use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use super::GiffiBot;
use bitschess::prelude::*;

use super::{MATE, MATE_THRESHOLD};

impl GiffiBot {
    /// Calculates until search_cancelled is set to true
    #[inline(always)]
    pub fn go_infinite(&mut self) {
        self.go_depth(super::MAX_DEPTH);
    }

    #[inline(always)]
    pub fn go_timed(&mut self, time: Duration) {
        const CHECK_INTERVAL: Duration = Duration::from_millis(10);

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

        self.go_depth(super::MAX_DEPTH);
        let _ = handle.join();
    }

    pub fn go_depth(&mut self, depth: i32) {
        self.iterations = 0;
        self.search_begin = std::time::Instant::now();
        self.completed_depth = 0;
        let mut best_completed_line = VecDeque::new();

        for depth in 1..=depth {
            let mut line = VecDeque::new();
            let perspective = if self.board.get_turn() == PieceColor::White {
                1
            } else {
                -1
            };
            let cancellable = depth > 1;
            let score = self.search(-i32::MAX, i32::MAX, depth, 0, &mut line, 0, cancellable);
            let score_with_perspective = score * perspective;

            // Calculate at least one move
            if cancellable && self.search_cancelled.load(Ordering::Relaxed) {
                break;
            }

            // if search was cancelled, the line is going to be incomplete
            best_completed_line = line;
            self.pv = best_completed_line.clone();
            self.completed_depth = depth;

            // Stats
            let end = std::time::Instant::now();
            let duration = end - self.search_begin;

            // Break if mate or centipawn score
            if let Some(chess_move) = self.pv.front() {
                print!("info depth {} ", depth);
                if score.abs() >= MATE_THRESHOLD {
                    let plies = MATE - score.abs();
                    let mate_in = (plies + 1) / 2;
                    let mate_in = if score > 0 { mate_in } else { -mate_in };
                    print!("score mate {} ", mate_in);
                } else {
                    print!("score cp {} ", score_with_perspective);
                }
                print!(
                    "currmove {} nodes {} time {} nps {} ",
                    chess_move.to_uci(),
                    self.iterations,
                    duration.as_millis(),
                    (self.iterations as f32 / duration.as_secs_f32()) as i32
                );

                // The full calcualted line
                print!("pv ");
                for m in &self.pv {
                    print!("{} ", m.to_uci());
                }
                println!();

                // Forced mate, we can stop calculating
                if score.abs() >= MATE_THRESHOLD {
                    break;
                }
            }
        }

        // last resort: make sure we always have a move to play
        if best_completed_line.is_empty() {
            if let Some(m) = self.board.get_legal_moves().get(0) {
                best_completed_line.push_front(m);
            }
        }
        self.pv = best_completed_line;

        if let Some(chess_move) = self.pv.front() {
            println!("bestmove {}", chess_move.to_uci());
        } else {
            println!("bestmove 0000");
        }
    }
}
