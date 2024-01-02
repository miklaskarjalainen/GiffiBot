# Tests
Used to test Bot's performance against other bots or against older versions of itself.

## Equal FEN positions
Simple ```generate_chess_positions.py``` script that i wrote can be used to generate new positions.  
Although shouldn't be need since i already generated them in the ```positions.txt```.
The positions are generated from [Lichess OpenDatabase](https://database.lichess.org/) (2015 - november).

### Generating new positions
Using ```generate_chess_positions.py``` new fen positions can be made.
The python script can be given a huge database of games in PGN format to test positions for constrains. Like the game needs to be X moves from the starting position (for diversity), and stockfish evaluation has to be +/- 50 centipawns.

Notable settings to change in ```generate_chess_positions.py```.
```py
# Filepaths
PGN_FILE = ""
STOCKFISH_PATH = ""
OUTPUT_FILE = "./positions.txt"

# Search settings
SEARCH_COUNT   = 50000  # max positions to search
STOP_SEARCH_AT = 1000   # after finding X positions, stop searching.
GAME_MIN_DEPTH = 20 # inclusive, min moves had to be made from the starting position
GAME_MAX_DEPTH = 40 # inclusive
EVAL_THRESHOLD = 35 # the position's evaluation (centipawn) has to be +/- EVAL_THRESHOLD to be included in the positions. Lower number means more equal positions
```

Installing dependencies
```
pip install chess stockfish
```

Generating the positions
```
py ./generate_chess_positions.py
```

## Testing
Using the fen positions to test the bot. Programs like [c-chess-cli](https://github.com/lucasart/c-chess-cli) can be used to run multiple chess matches at the same time. It outputs the wins/losses/draws of the matches and output file for PGN of the games can be set.
