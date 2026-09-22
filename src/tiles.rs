//! 牌编码（34 种）
//! 0..8   万1-9
//! 9..17  条1-9
//! 18..26 筒1-9
//! 27东 28南 29西 30北 31中 32发 33白

pub const NUM_TILES: usize = 34;

pub type Counts = [u8; NUM_TILES];
pub type Tiles = Vec<usize>;

pub fn counts() -> Counts {
    [0u8; NUM_TILES]
}

pub fn suit(t: usize) -> usize {
    if t < 27 {
        t / 9
    } else {
        3
    }
}

/// 数牌 1..9；字牌返回 0
pub fn num(t: usize) -> usize {
    if t < 27 {
        t % 9 + 1
    } else {
        0
    }
}

pub fn is_suited(t: usize) -> bool {
    t < 27
}
pub fn is_honor(t: usize) -> bool {
    t >= 27
}
pub fn is_wind(t: usize) -> bool {
    (27..=30).contains(&t)
}
pub fn is_dragon(t: usize) -> bool {
    (31..=33).contains(&t)
}
pub fn is_terminal(t: usize) -> bool {
    t < 27 && (num(t) == 1 || num(t) == 9)
}
pub fn is_terminal_or_honor(t: usize) -> bool {
    t >= 27 || is_terminal(t)
}

const SUIT_NAMES: [&str; 3] = ["万", "条", "筒"];
const HONOR_NAMES: [&str; 7] = ["东", "南", "西", "北", "中", "发", "白"];

pub fn tile_name(t: usize) -> String {
    if t < 27 {
        format!("{}{}", num(t), SUIT_NAMES[t / 9])
    } else {
        HONOR_NAMES[t - 27].to_string()
    }
}

pub fn total(c: &Counts) -> u32 {
    c.iter().map(|&x| x as u32).sum()
}

pub fn to_tiles(c: &Counts) -> Tiles {
    let mut v = Vec::new();
    for (i, &n) in c.iter().enumerate() {
        for _ in 0..n {
            v.push(i);
        }
    }
    v
}

/// 解析简写：如 "123m 456s 789p 东东东" / "123456789m 白白"
pub fn parse_hand(s: &str) -> Counts {
    let mut c = counts();
    let bytes: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut digits: Vec<usize> = Vec::new();
    while i < bytes.len() {
        let ch = bytes[i];
        match ch {
            '0'..='9' => {
                digits.push(ch as usize - '0' as usize);
                i += 1;
            }
            'm' | '万' => {
                for &d in &digits {
                    if d >= 1 && d <= 9 {
                        c[d - 1] += 1;
                    }
                }
                digits.clear();
                i += 1;
            }
            's' | '条' => {
                for &d in &digits {
                    if d >= 1 && d <= 9 {
                        c[9 + d - 1] += 1;
                    }
                }
                digits.clear();
                i += 1;
            }
            'p' | '筒' => {
                for &d in &digits {
                    if d >= 1 && d <= 9 {
                        c[18 + d - 1] += 1;
                    }
                }
                digits.clear();
                i += 1;
            }
            '东' | '南' | '西' | '北' | '中' | '发' | '白' => {
                let idx = match ch {
                    '东' => 27,
                    '南' => 28,
                    '西' => 29,
                    '北' => 30,
                    '中' => 31,
                    '发' => 32,
                    _ => 33,
                };
                c[idx] += 1;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    c
}
