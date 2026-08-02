use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

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

#[test]
fn test_mate_in_threes() {
    // as black
    let m1 = get_best_move_for_position("8/7R/2r5/8/P3n3/8/3nk1PP/R5K1 b - - 0 1");
    assert_eq!(m1.to_uci(), "d2f3"); // 1... Nf3+ 2. gxf3 Rg6+ 3. Kh1 Nf2#

    // as white
    let m2 =
        get_best_move_for_position("rn1k3r/1b1q1ppp/p2P4/2B2p2/8/1QNBR3/PP3PPP/2R3K1 w - - 1 0");
    assert_eq!(m2.to_uci(), "b3b6"); // 1. Qb6+ Kc8 2. Bxf5 Bc6 3. Qc7#
}

#[test]
fn test_atleast_depth_1() {
    // The position is very complex should take more than 10ms to run.
    // Previous versions crashed if they couldn't produce a move in the time constraint.
    // Now it should calculate atleast the captures from depth 1 even if goes overtime.

    let mut engine = UCIEngine::new();
    engine
        .execute_cmd(
            "position fen r2r1bk1/2q2pp1/p1n1bn1p/2p1pN2/1p2P1P1/2P1BN1P/PPQ1BP2/3RK1R1 w - - 0 17",
        )
        .unwrap();
    engine.execute_cmd("go movetime 10").unwrap();

    std::thread::sleep(Duration::from_millis(500));
}
