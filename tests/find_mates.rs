use std::sync::{atomic::AtomicBool, Arc};

use giffibot::*;

fn get_best_move_for_position(fen: &str) -> Move {
    let mut board = ChessBoard::new();
    board.parse_fen(fen).expect("Invalid FEN");

    let stop = Arc::new(AtomicBool::new(false));
    let mut engine = GiffiBot::new(board.clone(), stop.clone());
    engine.go_infinite();
    let chess_move = *engine.pv.front().expect("?");
    chess_move
}

#[test]
fn test_mate_in_ones() {
    // as black
    let m1 = get_best_move_for_position("8/7R/1Rbk4/2p5/2Pp2p1/1N1P2P1/3KP2P/1r3r2 b - - 0 38");
    assert_eq!(m1.to_uci(), "b1b2");

    // as white
    let m2 = get_best_move_for_position("1kb5/1p1p3p/2p4p/4ppq1/5n2/Q1P5/2P2PPP/R5K1 w - - 4 29");
    assert_eq!(m2.to_uci(), "a3d6");
}
