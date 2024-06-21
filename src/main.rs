mod chessbot;
mod uci;

use bitschess::prelude::*;
use uci::UCIEngine;

fn main() {
    let mut uci = UCIEngine::new();
    uci.board.parse_fen(STARTPOS_FEN).expect("valid fen");

    println!("GiffiBot!");
    loop {

        let line = std::io::stdin().lines().next().unwrap().unwrap();
        
        if &line == "quit" {
            return;
        }
        else {
            if let Err(e) = uci.execute_cmd(&line) {
                println!("{:?}", e);
            }
        }
    }

}
