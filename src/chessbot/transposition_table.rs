const SIZE_IN_MB: u64 = 64;
const ENTRY_COUNT: u64 = (1024*1024*SIZE_IN_MB) / (std::mem::size_of::<TranspositionEntry>() as u64);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum NodeKind {
    LowerBound,
    Exact,
    UpperBound
}

#[derive(Debug, Clone, Copy)]
struct TranspositionEntry {
    zobrist_hash: u64,
    kind: NodeKind,
    score: i32,
    depth: i32
}

impl Default for TranspositionEntry {
    fn default() -> Self {
        TranspositionEntry {
            zobrist_hash: 0,
            kind: NodeKind::Exact,
            score: 0,
            depth: 0,
        }
    }
}

impl TranspositionEntry {
    fn new(hash: u64, kind: NodeKind, score: i32, depth: i32) -> Self {
        Self {
            zobrist_hash: hash,
            kind: kind,
            score: score,
            depth: depth
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranspositionTable {
    table: Vec<TranspositionEntry>, // test static arrays and vectors

    pub writes: u64,
    pub lookups: u64
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            table: vec![TranspositionEntry::default(); ENTRY_COUNT as usize],
            
            writes: 0,
            lookups: 0
        }
    }

    pub fn store_evaluation(&mut self, kind: NodeKind, hash: u64, depth: i32, score: i32) {
        let key: usize = (hash % ENTRY_COUNT) as usize;
        self.table[key] = TranspositionEntry::new(hash, kind, depth, score);
        self.writes += 1;
    }

    pub fn probe_hash(&mut self, hash: u64, depth:i32, alpha: i32, beta: i32) -> Option<i32> {
        let index: usize = (hash % ENTRY_COUNT) as usize;
        let entry: TranspositionEntry = self.table[index];


        if entry.zobrist_hash == hash {
            if entry.depth >= depth {

                match entry.kind {
                    NodeKind::Exact => {
                        self.lookups += 1;
                        return Some(entry.score);
                    }

                    NodeKind::UpperBound => {
                        if entry.score <= alpha {
                            self.lookups += 1;
                            return Some(entry.score);
                        }   
                    }

                    NodeKind::LowerBound => {
                        if entry.score >= beta {
                            self.lookups += 1;
                            return Some(entry.score);
                        }
                    }

                    _ => {}
                }
            }
        }
        None
    }
}
