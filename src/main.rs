use gbmj::calc::analyze;
use gbmj::model::{validate_counts, Meld};
use gbmj::plan::MAX_DIST;
use gbmj::tiles::*;

fn first_tile(c: &Counts) -> Option<usize> {
    (0..34usize).find(|&t| c[t] > 0)
}

fn wind_idx(s: &str) -> Option<usize> {
    match s {
        "东" | "east" | "E" => Some(27),
        "南" | "south" | "S" => Some(28),
        "西" | "west" | "W" => Some(29),
        "北" | "north" | "N" => Some(30),
        _ => None,
    }
}

/// 统一错误通道（C1）：stderr 提示 + 用法提示 + 退出码 2
fn die(msg: &str) -> ! {
    eprintln!("错误: {}", msg);
    eprintln!("用法: gbmj <手牌> [副露...] [圈门...]（gbmj --help 查看完整说明）");
    std::process::exit(2);
}

fn parse_or_die(s: &str) -> Counts {
    match try_parse_hand(s) {
        Ok(c) => c,
        Err(e) => die(&e.to_string()),
    }
}

fn help() {
    println!(
        "国标麻将算番器 (听牌算番)\n\
         \n\
         用法:\n\
         gbmj <手牌> [副露...] [圈门...]\n\
         \n\
         手牌: 13 - 3×副露数 张暗牌，如 123m 456s 789p 东东\n\
         副露:\n\
           --chi  123m    吃\n\
           --pon  5p      碰\n\
           --kan  5p      明杠\n\
           --ankan 5p     暗杠\n\
         圈门:\n\
           --round 东     圈风\n\
           --seat  南     门风\n\
         扫描:\n\
           --plan        未听牌方向扫描（离各番种还差几张，最多 {MAX_DIST} 张）\n\
         \n\
         例: gbmj 123456789m 11p 白白 --round 东 --seat 南"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        help();
        return;
    }

    let mut hand = String::new();
    let mut melds: Vec<Meld> = Vec::new();
    let mut round = 27usize;
    let mut seat = 27usize;
    let mut plan_mode = false;
    let mut round_set = false;
    let mut seat_set = false;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--chi" | "--pon" | "--kan" | "--ankan" => {
                i += 1;
                if i >= args.len() {
                    die(&format!("缺少参数: {}", a));
                }
                let c = parse_or_die(&args[i]);
                let t = match first_tile(&c) {
                    Some(t) => t,
                    None => die(&format!("{} 后面没有有效牌", a)),
                };
                let meld = match a {
                    "--chi" => Meld::chi(t),
                    "--pon" => Meld::pon(t),
                    "--kan" => Meld::kan(t, true),
                    _ => Meld::kan(t, false),
                };
                melds.push(meld);
            }
            "--round" => {
                i += 1;
                if i >= args.len() {
                    die("缺少参数: --round");
                }
                if round_set {
                    die("--round 重复给出");
                }
                match wind_idx(&args[i]) {
                    Some(w) => {
                        round = w;
                        round_set = true;
                    }
                    None => die(&format!("无效圈风: {}", args[i])),
                }
            }
            "--plan" => {
                plan_mode = true;
            }
            "--seat" => {
                i += 1;
                if i >= args.len() {
                    die("缺少参数: --seat");
                }
                if seat_set {
                    die("--seat 重复给出");
                }
                match wind_idx(&args[i]) {
                    Some(w) => {
                        seat = w;
                        seat_set = true;
                    }
                    None => die(&format!("无效门风: {}", args[i])),
                }
            }
            other => {
                if other.starts_with('-') {
                    die(&format!("未知参数: {}", other));
                }
                hand.push_str(other);
                hand.push(' ');
            }
        }
        i += 1;
    }

    let concealed = parse_or_die(&hand);
    if total(&concealed) == 0 {
        die("没有解析出任何牌");
    }
    if melds.len() > 4 {
        die("副露面子数不能超过 4 副（杠也算一副）");
    }
    if let Err(e) = validate_counts(&concealed, &melds) {
        die(&e.to_string());
    }

    if plan_mode {
        let dirs = gbmj::plan::plan(&concealed, &melds, MAX_DIST);
        println!(
            "暗牌 {} 张 | 副露 {} 副   方向扫描（离某个番种还差几张，最多 {MAX_DIST} 张）",
            total(&concealed),
            melds.len()
        );
        if dirs.is_empty() {
            println!("  没有 {MAX_DIST} 张以内能凑到的番种方向");
            return;
        }
        for d in dirs.iter() {
            println!(
                "  {:<12}{:>3}番  差{}张   补: {:<16}换掉: {}{}{}",
                d.name,
                d.value,
                d.distance,
                d.need_str(),
                d.drop_str(),
                if d.tag.is_empty() {
                    String::new()
                } else {
                    format!("   【{}】", d.tag)
                },
                if d.covered_by.is_empty() {
                    String::new()
                } else {
                    format!("   【已被 {} 覆盖】", d.covered_by.join("、"))
                }
            );
        }
        return;
    }

    let an = analyze(&concealed, &melds, round, seat);

    let n = total(&concealed);
    println!(
        "暗牌 {} 张 | 副露 {} 副 | {}",
        n,
        an.meld_count,
        if an.menqing { "门清" } else { "非门清" }
    );

    if an.structural_waits.is_empty() {
        println!("未听牌（无和牌型）");
        return;
    }

    let wait_str: Vec<String> = an.structural_waits.iter().map(|&t| tile_name(t)).collect();
    println!("结构听牌: {}", wait_str.join(" "));
    println!();
    println!(
        "{:<6}{:>8}{:>8}{:>10}{:>12}   够和",
        "和牌张", "点和", "自摸", "点和绝张", "自摸绝张"
    );
    let mut has_non_win = false;
    for w in an.waits.iter() {
        let verdict = w.verdict();
        if verdict != gbmj::calc::WinVerdict::Win {
            has_non_win = true;
        }
        // 三态短标记（D2）：✔ 能和 / △ 条件可和 / ✘ 不可和
        let ok = match verdict {
            gbmj::calc::WinVerdict::Win => "✔",
            gbmj::calc::WinVerdict::ConditionalLast => "△",
            gbmj::calc::WinVerdict::NoWin => "✘",
        };
        // 「仅自摸 / 仅绝张」标签仅在确定能和的 Win 行保留
        let tag = if verdict == gbmj::calc::WinVerdict::Win {
            if w.only_tsumo() {
                " ✔仅自摸"
            } else if w.only_last() {
                " ✔仅绝张"
            } else {
                ""
            }
        } else {
            ""
        };
        println!(
            "{:<7}{:>8}{:>8}{:>10}{:>12}   {}{}",
            tile_name(w.tile),
            w.normal,
            w.tsumo,
            if w.can_last {
                w.last.to_string()
            } else {
                "—".to_string()
            },
            if w.can_last {
                w.tsumo_last.to_string()
            } else {
                "—".to_string()
            },
            ok,
            tag
        );
    }
    if has_non_win {
        println!("图例：✔ 可和；△ 条件可和（仅非绝张世界，点和）；✘ 不可和");
    }
    println!();
    for w in an.waits.iter() {
        // 番表明细：未听牌行在本函数更早处已 return，循环内均为听牌行——除未听牌外全部打印明细
        let names: Vec<String> = w
            .fans
            .iter()
            .map(|f| format!("{}{}", f.name, f.value))
            .collect();
        println!("{}: {}", tile_name(w.tile), names.join(" "));
    }
}
