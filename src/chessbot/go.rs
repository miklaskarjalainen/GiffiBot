use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::GiffiBot;
use bitschess::prelude::*;

pub const MAX_DEPTH: usize = 256; // there is no way we're reaching depth 256 in our lifetime :D

impl GiffiBot {

    /// Calculates until search_cancelled is set to true
    #[inline(always)]
    pub fn go_infinite(&mut self) {
        self.go_depth(MAX_DEPTH);
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
        
        self.go_depth(MAX_DEPTH);
        let _ = handle.join();
    }

    pub fn go_depth(&mut self, depth: usize) {
        use super::DequeType;

        self.iterations = 0;
        self.search_begin = std::time::Instant::now();
        self.completed_depth = 0;
        let mut best_completed_line = DequeType::new();
        
        for depth in 1..=depth {
            let mut line = DequeType::new();
            let perspective = if self.board.get_turn() == PieceColor::White { 1 } else { -1 };
            let score = self.search(-i32::MAX, i32::MAX, depth as i32, 0, &mut line, 0) * perspective;
            
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
            print!("info depth {} score cp {} currmove {} nodes {} duration_from_go {} nps {}",
                depth, score, self.pv.front().unwrap().to_uci(), self.iterations, duration.as_secs_f32(), (self.iterations as f32 / duration.as_secs_f32()) as i32
            ); 
            // pv
            print!(" pv ");
            for m in &self.pv {
                print!("{} ", m.to_uci());
            }
            print!("\n");
        }
        self.pv = best_completed_line;
        
        println!("bestmove {}", self.pv.front().cloned().unwrap().to_uci());
    }

}