use bitschess::Move;
use std::cell::Cell;

const SIZE_IN_MB: u64 = 64;
const ENTRY_COUNT: u64 =
    (1024 * 1024 * SIZE_IN_MB) / (std::mem::size_of::<TranspositionEntry>() as u64);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum NodeKind {
    /// The stored evaluation can not be lower (score >= beta)
    LowerBound,
    /// True minmax value (alpha < score < beta)
    Exact,
    /// The stored evaluation can not be higher (score <= alpha)
    UpperBound,
}

#[derive(Debug, Clone, Copy)]
struct TranspositionEntry {
    zobrist_hash: u64,
    kind: NodeKind,
    score: i32,
    depth: i32,
    best_move: Move,
}

impl Default for TranspositionEntry {
    fn default() -> Self {
        TranspositionEntry {
            zobrist_hash: 0,
            kind: NodeKind::Exact,
            score: 0,
            depth: 0,
            best_move: Move(0),
        }
    }
}

impl TranspositionEntry {
    fn new(hash: u64, kind: NodeKind, score: i32, depth: i32, best_move: Move) -> Self {
        Self {
            zobrist_hash: hash,
            kind,
            score,
            depth,
            best_move,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranspositionTable {
    table: Vec<TranspositionEntry>, // test static arrays and vectors

    pub writes: Cell<u64>,
    pub lookups: Cell<u64>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            table: vec![TranspositionEntry::default(); ENTRY_COUNT as usize],

            writes: Cell::new(0),
            lookups: Cell::new(0),
        }
    }

    pub fn store_evaluation(
        &mut self,
        kind: NodeKind,
        hash: u64,
        depth: i32,
        score: i32,
        best_move: Move,
    ) {
        let key: usize = (hash % ENTRY_COUNT) as usize;
        self.table[key] = TranspositionEntry::new(hash, kind, score, depth, best_move);
        self.writes.set(self.writes.get() + 1);
    }

    /// # Returns
    /// The move if in transposition table. If not returns NULL move
    pub fn get_entry_by_hash(&self, hash: u64) -> Move {
        let index: usize = (hash % ENTRY_COUNT) as usize;
        let entry: TranspositionEntry = self.table[index];
        if entry.zobrist_hash == hash {
            entry.best_move
        } else {
            Move(0)
        }
    }

    pub fn probe_hash(&self, hash: u64, depth: i32, alpha: i32, beta: i32) -> Option<(i32, Move)> {
        self.lookups.set(self.lookups.get() + 1);
        let index: usize = (hash % ENTRY_COUNT) as usize;
        let entry: TranspositionEntry = self.table[index];

        if entry.zobrist_hash == hash && entry.depth >= depth {
            match entry.kind {
                NodeKind::Exact => {
                    if entry.score > alpha && entry.score < beta {
                        return Some((entry.score, entry.best_move));
                    }
                }

                NodeKind::UpperBound => {
                    // We know the true score is ≤ entry.score
                    // If entry.score is already ≤ alpha, this position is hopeless
                    if entry.score <= alpha {
                        return Some((entry.score, entry.best_move));
                    }
                }

                NodeKind::LowerBound => {
                    // We know the true score is ≥ entry.score
                    // If entry.score is already ≥ beta, this move is too good
                    if entry.score >= beta {
                        return Some((entry.score, entry.best_move));
                    }
                }
            }
        }
        None
    }
}
