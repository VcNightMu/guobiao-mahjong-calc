//! 手牌分解：标准型（4 面子 + 1 将）与特殊型（七对 / 十三幺 / 全不靠）

use crate::tiles::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetKind {
    Run,
    Triplet,
}

#[derive(Clone, Copy, Debug)]
pub struct Set {
    pub kind: SetKind,
    /// Run 取最小牌；Triplet 取该牌
    pub tile: usize,
    pub is_kan: bool,
    /// 是否来自副露
    pub open: bool,
}

impl Set {
    pub fn run(tile: usize) -> Self {
        Set { kind: SetKind::Run, tile, is_kan: false, open: false }
    }
    pub fn triplet(tile: usize) -> Self {
        Set { kind: SetKind::Triplet, tile, is_kan: false, open: false }
    }
    pub fn is_run(&self) -> bool {
        self.kind == SetKind::Run
    }
    pub fn is_triplet(&self) -> bool {
        self.kind == SetKind::Triplet
    }
    pub fn tiles(&self) -> Vec<usize> {
        match self.kind {
            SetKind::Run => vec![self.tile, self.tile + 1, self.tile + 2],
            SetKind::Triplet => vec![self.tile; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    SevenPairs,
    SevenPairsStraight,
    ThirteenOrphans,
    BuKao,
    SevenStars,
}

#[derive(Clone, Debug)]
pub struct Decomp {
    /// 将（特殊型为 None）
    pub pair: Option<usize>,
    /// 面子（含副露面子）
    pub sets: Vec<Set>,
    pub special: Option<Special>,
}

impl Decomp {
    pub fn standard(pair: usize, sets: Vec<Set>) -> Self {
        Decomp { pair: Some(pair), sets, special: None }
    }
    pub fn special(s: Special) -> Self {
        Decomp { pair: None, sets: Vec::new(), special: Some(s) }
    }
}

fn take_sets(c: &mut Counts, k: usize, cur: &mut Vec<Set>, out: &mut Vec<Vec<Set>>) {
    if k == 0 {
        if c.iter().all(|&x| x == 0) {
            out.push(cur.clone());
        }
        return;
    }
    let f = match c.iter().position(|&x| x > 0) {
        Some(i) => i,
        None => return,
    };
    // 刻子
    if c[f] >= 3 {
        c[f] -= 3;
        cur.push(Set::triplet(f));
        take_sets(c, k - 1, cur, out);
        cur.pop();
        c[f] += 3;
    }
    // 顺子
    if is_suited(f) && f % 9 <= 6 && c[f + 1] > 0 && c[f + 2] > 0 {
        c[f] -= 1;
        c[f + 1] -= 1;
        c[f + 2] -= 1;
        cur.push(Set::run(f));
        take_sets(c, k - 1, cur, out);
        cur.pop();
        c[f] += 1;
        c[f + 1] += 1;
        c[f + 2] += 1;
    }
}

/// 所有 (将, k 面子) 分解
pub fn decompose_standard(c: &Counts, k: usize) -> Vec<Decomp> {
    let mut res = Vec::new();
    for p in 0..34usize {
        if c[p] >= 2 {
            let mut cc = *c;
            cc[p] -= 2;
            let mut outs = Vec::new();
            let mut cur = Vec::new();
            take_sets(&mut cc, k, &mut cur, &mut outs);
            for s in outs {
                res.push(Decomp::standard(p, s));
            }
        }
    }
    res
}

pub fn is_seven_pairs(c: &Counts) -> bool {
    if total(c) != 14 {
        return false;
    }
    let mut pairs = 0u32;
    for &x in c.iter() {
        if x % 2 != 0 {
            return false;
        }
        pairs += (x / 2) as u32;
    }
    pairs == 7
}

pub fn is_seven_pairs_straight(c: &Counts) -> bool {
    if !is_seven_pairs(c) {
        return false;
    }
    for s in 0..3usize {
        let base = s * 9;
        for start in 0..=2usize {
            let mut ok = true;
            for n in 0..7usize {
                if c[base + start + n] != 2 {
                    ok = false;
                    break;
                }
            }
            if ok {
                let mut others = 0u32;
                for i in 0..34usize {
                    if !(i >= base + start && i < base + start + 7) {
                        others += c[i] as u32;
                    }
                }
                if others == 0 {
                    return true;
                }
            }
        }
    }
    false
}

pub const YAOJIU: [usize; 13] = [0, 8, 9, 17, 18, 26, 27, 28, 29, 30, 31, 32, 33];

pub fn is_thirteen_orphans(c: &Counts) -> bool {
    if total(c) != 14 {
        return false;
    }
    let mut pairs = 0;
    for &t in YAOJIU.iter() {
        match c[t] {
            1 => {}
            2 => pairs += 1,
            _ => return false,
        }
    }
    if pairs != 1 {
        return false;
    }
    for i in 0..34usize {
        if !YAOJIU.contains(&i) && c[i] > 0 {
            return false;
        }
    }
    true
}

const KNIT_COLS: [[usize; 3]; 3] = [[1, 4, 7], [2, 5, 8], [3, 6, 9]];
const PERMS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

pub fn is_bu_kao(c: &Counts) -> bool {
    if total(c) != 14 {
        return false;
    }
    for &x in c.iter() {
        if x > 1 {
            return false; // 必须全不同
        }
    }
    for perm in PERMS.iter() {
        let mut ok = true;
        for s in 0..3usize {
            let col = KNIT_COLS[perm[s]];
            for n in 1..=9usize {
                if c[s * 9 + n - 1] > 0 && !col.contains(&n) {
                    ok = false;
                    break;
                }
            }
            if !ok {
                break;
            }
        }
        if ok {
            return true;
        }
    }
    false
}

pub fn is_seven_stars(c: &Counts) -> bool {
    if !is_bu_kao(c) {
        return false;
    }
    (27..=33usize).all(|t| c[t] == 1)
}
