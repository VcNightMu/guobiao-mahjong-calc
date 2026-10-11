use gbmj::calc::analyze;
use gbmj::model::Meld;
use gbmj::tiles::*;

fn first_tile(c: &Counts) -> usize {
    (0..34usize).find(|&t| c[t] > 0).expect("空牌")
}

fn wind_idx(s: &str) -> usize {
    match s {
        "东" | "east" | "E" => 27,
        "南" | "south" | "S" => 28,
        "西" | "west" | "W" => 29,
        "北" | "north" | "N" => 30,
        _ => 27,
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

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--chi" | "--pon" | "--kan" | "--ankan" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("缺少参数: {}", a);
                    return;
                }
                let c = parse_hand(&args[i]);
                let t = first_tile(&c);
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
                round = wind_idx(&args[i]);
            }
            "--plan" => {
                plan_mode = true;
            }
            "--seat" => {
                i += 1;
                seat = wind_idx(&args[i]);
            }
            other => {
                hand.push_str(other);
                hand.push(' ');
            }
        }
        i += 1;
    }

    let concealed = parse_hand(&hand);

    if plan_mode {
        let dirs = gbmj::plan::plan(&concealed, &melds, 2);
        println!(
            "暗牌 {} 张 | 副露 {} 副   方向扫描（离某个番种还差几张，最多 2 张）",
            total(&concealed),
            melds.len()
        );
        if dirs.is_empty() {
            println!("  没有 2 张以内能凑到的番种方向");
            return;
        }
        for d in dirs.iter() {
            println!(
                "  {:<12}{:>3}番  差{}张   补: {:<16}换掉: {}{}",
                d.name,
                d.value,
                d.distance,
                d.need_str(),
                d.drop_str(),
                if d.tag.is_empty() {
                    String::new()
                } else {
                    format!("   【{}】", d.tag)
                }
            );
        }
        return;
    }

    let an = analyze(&concealed, &melds, round, seat);

    let n = total(&concealed);
    println!("暗牌 {} 张 | 副露 {} 副 | {}", n, an.meld_count,
        if an.menqing { "门清" } else { "非门清" });

    if an.structural_waits.is_empty() {
        println!("未听牌（无和牌型）");
        return;
    }

    let wait_str: Vec<String> = an.structural_waits.iter().map(|&t| tile_name(t)).collect();
    println!("结构听牌: {}", wait_str.join(" "));
    println!();
    println!("{:<6}{:>8}{:>8}{:>10}{:>12}   够和", "和牌张", "点和", "自摸", "点和绝张", "自摸绝张");
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
            if w.can_last { w.last.to_string() } else { "—".to_string() },
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
        let names: Vec<String> =
            w.fans.iter().map(|f| format!("{}{}", f.name, f.value)).collect();
        println!("{}: {}", tile_name(w.tile), names.join(" "));
    }
}
