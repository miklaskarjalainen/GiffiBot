use bitschess::prelude::*;

use crate::chessbot::GiffiBot;

pub trait UCI {
    fn execute_dev(&mut self, cmd: &String);
    fn execute_cmd(&mut self, message: &String);
    fn parse_position(&mut self, args: &mut Vec<&str>) -> Option<()>;
}

impl UCI for GiffiBot {
    fn execute_dev(&mut self, cmd: &String) {
        let mut args: Vec<&str> = cmd.split(' ').collect();
        args.reverse();

        // move cmd
        if args.len() == 1 {
            if cmd == "undo" {
                self.board.unmake_move();
            }
            else if cmd == "eval" {
                println!("eval: {}", self.evaluate());
            }
            else if cmd == "hash" {
                todo!();
            }
            else if cmd == "fen" {
                todo!();
            }
            else {
                let mut moves: Vec<Move> = self.board.get_legal_moves().into_iter().filter(|m| &m.to_uci() == cmd).collect();
                if let Some(m) = moves.pop() {
                    self.board.make_move(m, false);
                    println!("Moved {}", cmd);
                }
                else {
                    println!("Illegal move!");
                }
            }
        }
        else if args.len() == 2 {
            let command = args.pop().unwrap();
            if command == "moves" {
                let square = BoardHelper::text_to_square(&args.last().unwrap()[0..2]);
                let moves = self.board.get_legal_moves_for_square(square);

                let mut str = String::from("");
                for y in (0..=7).rev() {
                    str.push('|');
                    for x in 0..=7 {
                        str.push(self.board.get_piece(y * CHESSBOARD_WIDTH + x).to_char());
                        for m in &moves {
                            if m.get_to_idx() == (y*CHESSBOARD_WIDTH+x) {
                                str.pop().unwrap();
                                str.push('*');
                                break;
                            }
                        }
                        str.push('|');
                    }
                    str.push('\n');
                }
                println!("{}", str);
            }
            else if command == "perft" {
                match args.last().expect(":^(").parse::<u32>() {
                    Ok(depth) => {
                        let begin = std::time::Instant::now();
                        self.board.perft(depth, true);
                        let duration = std::time::Instant::now() - begin;
    
                        println!("perft took: {:?}", duration);
                    }
                    Err(_) => {
                        println!("error while parsing numerical value")
                    }
                }
            }
            else {
                println!("invalid command");
            }
        }
        else {
            println!("invalid command");
        }
    }

    // cmd "d1d2" -> move d1 to d2
    fn execute_cmd(&mut self, message: &String) {
        let mut args: Vec<&str> = message.split(' ').collect();
        args.reverse();

        if let Some(cmd) = args.pop() {
            match cmd {
                "uci" => {
                    println!("id name GiffiBot");
                    println!("id author Miklas ('Giffi') Karjalainen");
                    println!("uciok");
                }
                "isready" => {
                    println!("readyok");
                }
                "ucinewgame" => {
                    self.board.clear();
                }
                "position" => {
                    self.parse_position(&mut args);
                }
                "go" => {
                    self.calculate_time(std::time::Duration::from_millis(500));
                }
                "stop" => {}

                _ => { 
                    self.execute_dev(message);
                }
            }
        }

    }

    fn parse_position(&mut self, args: &mut Vec<&str>) -> Option<()> {
        let mut arg_iter = args.iter().rev().peekable();
        
        // startpos or fen
        if let Some(arg1) = arg_iter.next() {
            match *arg1 {
                "startpos" => { 
                    self.board.parse_fen(STARTPOS_FEN).expect("valid fen");
                }
                "fen" => {
                    let mut whole_fen = String::from("");

                    while let Some(fen_portion) = arg_iter.peek() {
                        if **fen_portion == "moves" { break; }
                        whole_fen.push_str(arg_iter.next().unwrap());
                        whole_fen.push(' ');
                    }

                    if let Err(error) = self.board.parse_fen(&whole_fen) {
                        println!("FEN PARSE ERROR: {:?}", error);
                    }

                }
                _ => { return None; }
            }
        }

        // startpos or fen
        if let Some(arg1) = arg_iter.next() {
            match *arg1 {
                // position startpos moves e2e4 b7b5 d2d3 b8a6 glf3 e7e5 c2c3 d8g5 h2h4
                "moves" => { 
                    for chessmove in arg_iter {
                        if chessmove.is_empty() { continue; }

                        self.board.make_move_uci(chessmove);                        
                    }
                }
                _ => { return None; }
            }
        }

        Some(())
    }

}