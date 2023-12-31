
mod chessbot;
mod uci;

use bitschess::prelude::*;
use chessbot::GiffiBot;
use uci::UCI;

fn main() {
    let mut bot = GiffiBot::new();
    
    println!("GiffiBot!");
    bot.board.parse_fen(STARTPOS_FEN);
    
    loop {

        let line = std::io::stdin().lines().next().unwrap().unwrap();

        if &line == "exit" {
            break;
        }
        else if &line == "board" {
            println!("{}", bot.board);
        }
        else if &line == "quit" {
            return;
        }
        else {
            bot.execute_cmd(&line);
        }
    }
}