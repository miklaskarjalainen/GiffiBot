# GiffiBot
GiffiBot is a [UCI](https://www.chessprogramming.org/UCI) chess engine written in Rust. The main chess logic is imported from the [BitsChess](https://github.com/miklaskarjalainen/BitsChess) repository. This repository purely for the "bot" code. 

The bot is currently very much Work In Progress and is very primitive and lacks a lot of features.

GiffiBot is (if you're lucky that is) available to play against at [Lichess](https://lichess.org/@/GiffiBot).
You can also use a [Chess GUI](https://www.chessprogramming.org/GUI) locally like [Arena](http://www.playwitharena.de/) can be used to play against the AI.

## Building

You need a working rust development environment.  
After which you can clone and build the repository as follows.
```bash
git clone https://github.com/miklaskarjalainen/GiffiBot
cd GiffiBot
cargo build --release
```

## Usage (Command Line)

After launching the engine you're proceeded with the a console. The bot communicates through [UCI](https://www.chessprogramming.org/UCI).  
Using the engine in it's simplest form through the console is done as follows.
```bash
# Set initial position with the following command.
# position [startpos|fen <fenposition>] moves <move-uci1> <move-uci2> <move-uci3>...

# Examples: startpos
position startpos # startpos with no moves made
position startpos moves e2e4 e7e5
# fen
position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 # fen with no moves made
position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves e2e4 e7e5

# "go" command outputs "bestmove <ucimove>", but doesn't make the move by itself
# let GiffiBot until depth 5 is calculated
go depth 5 
# let GiffiBot calculate for 100ms
go movetime 100
```

Example of usage:
```bash
position startpos # initializes startpos
go movetime 1000 # let the bot think for a move for 1000ms = 1s
# engine outputs: "bestmove e2e4" which means that giffibot thinks that the best move for this position is e2->e4 

position startpos moves e2e4 
go movetime 1000
# engine outputs: "bestmove e7e5"

position startpos moves e2e4 e7e5
go movetime 1000
# engine outputs: "bestmove d2d4"
```

A complete list of UCI commands is in [here](https://gist.github.com/DOBRO/2592c6dad754ba67e6dcaec8c90165bf)