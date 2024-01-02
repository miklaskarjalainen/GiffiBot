import random
import math
import chess
import chess.pgn
from stockfish import Stockfish

# Parses a PGN file and from the games, creates fairly evaluated FEN positions. 
# Used to test the chess bot. Designed to work with https://github.com/lucasart/c-chess-cli.

# Filepaths
PGN_FILE = ""
STOCKFISH_PATH = ""
OUTPUT_FILE = "./positions.txt"

# Search settings
SEARCH_COUNT    = 50000  # max positions to search
STOP_SEARCH_AT  = 1000   # after finding X positions, stop searching.
GAME_MIN_DEPTH = 20 # inclusive, min moves had to be made from the starting position
GAME_MAX_DEPTH = 40 # inclusive
EVAL_THRESHOLD = 35 # the position's evaluation (centipawn) has to be +/- EVAL_THRESHOLD to be included in the positions. Lower number means more equal positions

# Position settings
EVAL_DEPTH     = 16 # evaluation depth
FEN_PIECES      = "rnbq" # not including kings or pawns.
MIN_PIECE_COUNT = 8      # black + white pieces. Only including pieces in 'FEN_PIECES'

pgn = open(PGN_FILE)
fen_positions: [str] = []
chess_engine: Stockfish = Stockfish(path=STOCKFISH_PATH)
chess_engine.set_depth(EVAL_DEPTH)

for i in range(SEARCH_COUNT):
    if len(fen_positions) == STOP_SEARCH_AT:
        break

    game = chess.pgn.read_game(pgn)
    if game == None:
        print("no more games to evaluate, ended at " + str(i))
        break

    moves = game.mainline_moves()
    board: chess.Board = game.board()

    move_count_to_play = random.randrange(GAME_MIN_DEPTH, GAME_MAX_DEPTH+1)
    # We want the turn to be for white, so even move counts.
    if move_count_to_play % 2 != 0:
        move_count_to_play -= 1
    
    # Loop through moves
    moves_played = 0
    current_fen = ""
    for m in moves:
        board.push(m)
        moves_played += 1

        if board.is_game_over():
            break

        if moves_played == move_count_to_play:
            current_fen = board.fen()
            break

    if moves_played < GAME_MIN_DEPTH:
        continue
    if fen_positions.count(current_fen) != 0:
        continue

    # 
    piece_count = sum(current_fen.lower().count(char) for char in FEN_PIECES)
    if piece_count < MIN_PIECE_COUNT:
        continue

    # Evaluate
    chess_engine.set_fen_position(current_fen)
    eval = chess_engine.get_evaluation()
    if eval["type"] == "mate":
        continue

    if abs(eval["value"]) <= EVAL_THRESHOLD:
        fen_positions.append(current_fen)
        print(str.format("[{}] Found position '{}' eval {}", len(fen_positions), current_fen, eval["value"]))

# Write to 
output = open(OUTPUT_FILE, "w")
for position in fen_positions:
    output.write(position + ";\n")
output.flush()
