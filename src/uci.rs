
use std::sync::{atomic::AtomicBool, Arc};

use bitschess::prelude::*;
use crate::chessbot::GiffiBot;

#[derive(Debug)]
pub enum UciParseError {
    InvalidSyntax,
}

pub struct UCIEngine {
    pub board: ChessBoard,
    stop_search: Arc<AtomicBool>,

    option_movetime: Option<std::time::Duration>,
}

impl UCIEngine {
    pub fn new() -> Self {
        Self {
            stop_search: Arc::new(AtomicBool::new(false)),
            board: ChessBoard::new(),
            
            option_movetime: None,
        }
    }

    pub fn execute_cmd(&mut self, message: &String) -> Result<(), UciParseError> {
        let mut args = message.split(' ').collect::<Vec<&str>>().into_iter().peekable();

        if let Some(cmd) = args.next() {
            match cmd {
                // Dev commands
                "undo" => {
                    let _ = self.board.unmake_move();
                }
                "fen" => {
                    println!("FEN {}", self.board.to_fen());
                }
                "board" | "d" => {
                    println!("{}", self.board);
                }

                // UCI commands
                "uci" => {
                    println!("id name GiffiBot");
                    println!("id author Miklas ('Giffi') Karjalainen");
                    println!("option name movetime type spin default 0 min 100 max 10000");
                    println!("uciok");
                }
                "isready" => {
                    println!("readyok");
                }
                "ucinewgame" => {
                    self.board.clear();
                }
                "position" => {
                    return self.parse_position(&mut args);
                }

                "setoption" => {
                    if args.next() != Some("name") {
                        return Err(UciParseError::InvalidSyntax);
                    }

                    let option_id = args.next();
                    if option_id.is_none() {
                        return Err(UciParseError::InvalidSyntax);
                    }
                    if args.next() != Some("value") {
                        return Ok(());
                    }
                    if args.peek().is_none() {
                        return Err(UciParseError::InvalidSyntax);
                    }

                    match option_id.unwrap() {
                        "movetime" => {
                            let arg = str::parse::<u64>(args.next().unwrap());
                            if let Ok(arg) = arg {
                                if arg == 0 {
                                    self.option_movetime = None;
                                }
                                else {
                                    self.option_movetime = Some(
                                        std::time::Duration::from_millis(arg)
                                    );
                                }
                                return Ok(());
                            }
                            return Err(UciParseError::InvalidSyntax);
                        }

                        _ => {
                            return Ok(());
                        }
                    }
                }
                "go" => {
                    // Prepare to start search
                    use std::sync::atomic::Ordering;
                    self.stop_search.store(false, Ordering::Relaxed);
                    let board_copy = self.board.clone();
                    let cancelled_copy = Arc::clone(&self.stop_search);

                    if let Some(time) = self.option_movetime {
                        let _ = std::thread::spawn(move || {
                            let mut bot = GiffiBot::new(board_copy, cancelled_copy);
                            bot.calculate_time(time);
                        });
                        return Ok(());
                    }

                    // TODO: REDO THIS
                    while args.peek().is_some() {
                        let argument = args.next().unwrap();

                        match argument {
                            "wtime" | "btime" | "winc" | "binc" | "movestogo" => {
                                if args.next().is_none() {
                                    return Err(UciParseError::InvalidSyntax);
                                }
                            }

                            "movetime" => {
                                if args.peek().is_none() {
                                    return Err(UciParseError::InvalidSyntax);
                                }

                                let time = args.next().unwrap().parse::<u64>();
                                if time.is_err() {
                                    return Err(UciParseError::InvalidSyntax);
                                }

                                let search_time = std::time::Duration::from_millis(time.unwrap());
                                let _ = std::thread::spawn(move || {
                                    let mut bot = GiffiBot::new(board_copy, cancelled_copy);
                                    bot.calculate_time(search_time);
                                });

                                return Ok(());
                            }

                            "depth" => {
                                if args.peek().is_none() {
                                    return Err(UciParseError::InvalidSyntax);
                                }

                                let depth = args.next().unwrap().parse::<i32>();
                                if depth.is_err() {
                                    return Err(UciParseError::InvalidSyntax);
                                }

                                let search_depth = depth.unwrap();
                                let _ = std::thread::spawn(move || {
                                    let mut bot = GiffiBot::new(board_copy, cancelled_copy);
                                    bot.calculate_depth(search_depth);
                                });

                                return Ok(());
                            }

                            "infinite" => {
                                /*
                                let _ = std::thread::spawn(move || {
                                    let mut bot = GiffiBot::new(board_copy, cancelled_copy);
                                    bot.calculate();
                                });
                                */
                                let search_time = std::time::Duration::from_millis(500);
                                let _ = std::thread::spawn(move || {
                                    let mut bot = GiffiBot::new(board_copy, cancelled_copy);
                                    bot.calculate_time(search_time);
                                });
                                
                                return Ok(());
                            }

                            "perft" => {
                                if args.peek().is_none() {
                                    return Err(UciParseError::InvalidSyntax);
                                }

                                let depth = args.next().unwrap().parse::<u32>();
                                if depth.is_err() {
                                    return Err(UciParseError::InvalidSyntax);
                                }

                                let search_depth = depth.unwrap();
                                self.board.perft(search_depth, true);
                                return Ok(());
                            }
                            
                            _ => {
                                println!("UCI: unsupported argument '{}'", argument);
                            }
                        }
                    }    

                    let _ = std::thread::spawn(move || {
                        let mut bot = GiffiBot::new(board_copy, cancelled_copy);
                        bot.calculate_time(std::time::Duration::from_millis(100));
                    });
                    return Ok(());                
                }
                "stop" => {
                    use std::sync::atomic::Ordering;
                    self.stop_search.store(true, Ordering::Relaxed);
                }
                _ => { 
                    return Err(UciParseError::InvalidSyntax);
                }
            }
        }
        Ok(())
    }

    fn parse_position(&mut self, arg_iter: &mut std::iter::Peekable<std::vec::IntoIter<&str>>) -> Result<(), UciParseError> {        
        // startpos or fen
        if let Some(arg1) = arg_iter.next() {
            match arg1 {
                "startpos" => { 
                    self.board.parse_fen(STARTPOS_FEN).expect("valid fen");
                }
                "fen" => {
                    let mut whole_fen = String::from("");

                    while let Some(fen_portion) = arg_iter.peek() {
                        if *fen_portion == "moves" { break; }
                        whole_fen.push_str(arg_iter.next().unwrap());
                        whole_fen.push(' ');
                    }

                    if let Err(error) = self.board.parse_fen(&whole_fen) {
                        println!("FEN PARSE ERROR: {:?}", error);
                        return Err(UciParseError::InvalidSyntax);
                    }

                }
                _ => { return Err(UciParseError::InvalidSyntax); }
            }
        }

        // startpos or fen
        if let Some(arg1) = arg_iter.next() {
            match arg1 {
                // position startpos moves e2e4 b7b5 d2d3 b8a6 glf3 e7e5 c2c3 d8g5 h2h4
                "moves" => { 
                    for chessmove in arg_iter {
                        if chessmove.is_empty() { continue; }

                        self.board.make_move_uci(chessmove);                        
                    }
                }
                _ => { return Err(UciParseError::InvalidSyntax); }
            }
        }

        Ok(())
    }

}

