#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! gbmj-gui —— 国标麻将算番器 图形界面（egui / eframe）
//!
//! 布局：上=副露区|手牌区，中=牌库+圈门风，下=和牌张结果（五列）
//! 口径见 `界面格式-v0.1.md`。

use eframe::egui;
use gbmj::calc::{analyze, Analysis};
use gbmj::model::Meld;
use gbmj::tiles::{counts, parse_hand, tile_name, total, Counts, NUM_TILES};

const WIN_THRESHOLD: u32 = 8;

#[derive(PartialEq, Eq, Clone, Copy)]
enum MeldMode {
    None,
    Chi,
    Pon,
    MingKan,
    AnKan,
}

struct App {
    hand: Counts,
    melds: Vec<Meld>,
    mode: MeldMode,
    pending_chi: Vec<usize>,
    round_wind: usize, // 27..=30
    seat_wind: usize,
    hint: String,
    analysis: Option<Analysis>,
    dirty: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            hand: counts(),
            melds: Vec::new(),
            mode: MeldMode::None,
            pending_chi: Vec::new(),
            round_wind: 27,
            seat_wind: 27,
            hint: String::new(),
            analysis: None,
            dirty: true,
        }
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let mut app = Self::default();
        if std::env::var("GBMJ_DEMO").is_ok() {
            app.load_sample();
        }
        app
    }

    /// 载入示例。GBMJ_DEMO=1 门清示例；=2 带副露示例；=3 脚本化点击（自检副露按钮复位）
    fn load_sample(&mut self) {
        self.hand = counts();
        self.melds.clear();
        self.mode = MeldMode::None;
        self.pending_chi.clear();
        self.hint.clear();
        match std::env::var("GBMJ_DEMO").as_deref() {
            Ok("2") => {
                // 带副露示例：碰 3筒 + 吃 123条；暗牌 456s 789s 7p，和 7p → 清龙
                self.hand = parse_hand("456s789s 7p");
                self.melds = vec![Meld::pon(20), Meld::chi(9)];
            }
            Ok("3") => {
                // 脚本化点击：每组副露做完后 mode 应自动复位；结束时四个按钮都不应高亮
                self.mode = MeldMode::Pon;
                self.click_tile(20); // 碰 3筒
                self.mode = MeldMode::Chi;
                self.click_tile(9);
                self.click_tile(10);
                self.click_tile(11); // 吃 1条2条3条
                self.mode = MeldMode::MingKan;
                self.click_tile(26); // 明杠 9筒
            }
            Ok("4") => {
                // 四归一自检：吃 123万 + 吃 123条，暗牌 1112567万
                self.hand = parse_hand("1112567m");
                self.melds = vec![Meld::chi(0), Meld::chi(9)];
            }
            Ok("5") => {
                // 双碰听：点炮补的刻子算明刻（双暗刻）、自摸才是三暗刻 → 点和/自摸番表不同
                self.hand = parse_hand("555m66s777p123m东东");
            }
            Ok("6") => {
                // 改动后未点确认的状态：结果区应提示「已修改，点【确认】重新计算」
                self.hand = parse_hand("555p666p777p 555s 6s");
            }
            Ok("7") => {
                // 条件可和态（测试方案 §3.1 族④ / N-04）：9条 行应显示「条件可和（仅非绝张·点和）」、6条 行显示「不可和」
                // CLI 实测：gbmj 78s 东东 --chi 234m --pon 5p --pon 6s ⇒ 6条 2/3/6/7 ✘、9条 8/1/4/5 △
                self.hand = parse_hand("78s 东东");
                self.melds = vec![Meld::chi(1), Meld::pon(22), Meld::pon(14)];
            }
            Ok("8") => {
                // 未听牌态：结构听牌为空 ⇒ 结果区应提示「未听牌（无和牌型）」，不显示任何四栏行
                // CLI 实测：gbmj 123456789m 1357s ⇒ 未听牌（无和牌型）
                self.hand = parse_hand("123456789m 1357s");
            }
            Ok("9") => {
                // 仅绝张可和态（GUI 标记「（仅绝张可和）」）：6万/9万 两行均为 4/6/8/10 与 5/7/9/11
                // CLI 实测：gbmj 123m 78m 234p 567s 55p ⇒ 6万、9万 均 ✔仅绝张
                self.hand = parse_hand("123m 78m 234p 567s 55p");
            }
            Ok("10") => {
                // 非法输入（可脚本化）：暗牌 10 张 ≠ 13 ⇒ 结果区应提示「暗牌张数不对，无法计算（应为 13 − 3×副露 张）」，且不崩溃
                self.hand = parse_hand("123m 456s 789p 1p");
            }
            Ok("11") => {
                // 覆盖守卫「{牌} 已用满 4 张」（click_tile 的 MeldMode::None 分支）：手牌 1万×4（used==4），None 模式再点 1万
                self.hand = parse_hand("1111m 234m 567m 99s 5p");
                self.mode = MeldMode::None;
                self.click_tile(0); // 1万
            }
            Ok("12") => {
                // 覆盖守卫「碰 {牌} 会超过 4 张上限」（click_tile 的 MeldMode::Pon 分支）：2万 已占 2 张，Pon 模式再点它 ⇒ 2+3>4
                self.hand = parse_hand("22m 345m 678m 123s 99s");
                self.mode = MeldMode::Pon;
                self.click_tile(1); // 2万
            }
            Ok("13") => {
                // 覆盖守卫「杠 {牌} 会超过 4 张上限」（click_tile 的 MingKan | AnKan 分支）：2万 已占 1 张，明杠再点它 ⇒ 1+4>4（暗杠共用同一守卫与文案）
                self.hand = parse_hand("2m 345m 678m 123s 999s");
                self.mode = MeldMode::MingKan;
                self.click_tile(1); // 2万
            }
            Ok("14") => {
                // 覆盖守卫「吃：需同花色三张连续（如 2万3万4万）」（click_tile 的 Chi 形状校验）：Chi 模式依次点 1万/3万/5万（同花色但不连续）
                self.hand = parse_hand("123m 456m 789m 123s 5s");
                self.mode = MeldMode::Chi;
                self.click_tile(0); // 1万
                self.click_tile(2); // 3万
                self.click_tile(4); // 5万
            }
            Ok("15") => {
                // 覆盖守卫「吃 超出 4 张上限」（click_tile 的 Chi 四张上限校验）：2万 已占 4 张，Chi 点 2万3万4万（形状合法但 2万再 +1 超 4）
                self.hand = parse_hand("2222m 34m 567m 99s 55p");
                self.mode = MeldMode::Chi;
                self.click_tile(1); // 2万
                self.click_tile(2); // 3万
                self.click_tile(3); // 4万
            }
            Ok("16") => {
                // 覆盖「暗杠成功」路径（MeldMode::AnKan，open=false）与副露标签「暗杠 {}×4」、门清判断
                // 期望：副露区「暗杠 9筒×4」、手牌区「√ 张数正确」、门清：是、无 hint
                self.hand = parse_hand("123m456m789m 5s");
                self.mode = MeldMode::AnKan;
                self.click_tile(26); // 9筒（used==0 ⇒ +4<=4 通过）
            }
            Ok("17") => {
                // 覆盖守卫「杠 {牌} 会超过 4 张上限」的 AnKan 侧（与 MingKan 共用同一判定语句）
                // 期望 hint：「杠 1万 会超过 4 张上限」；手牌 13 张「√ 张数正确」
                self.hand = parse_hand("1m 234m 567m 99s 5588p");
                self.mode = MeldMode::AnKan;
                self.click_tile(0); // 1万（used==1 ⇒ 1+4>4）
            }
            Ok("18") => {
                // 覆盖主交互路径「MeldMode::None 成功点牌」：hand+1 / hint.clear() / dirty=true
                // 期望：手牌 12→13 张、「√ 张数正确」、出四栏结果、无 hint
                self.hand = parse_hand("123m 456m 789m 123s");
                self.mode = MeldMode::None;
                self.click_tile(9); // 1条 → 13 张
            }
            Ok("19") => {
                // 覆盖「吃」的累积中间态与进度标签「吃：{名} {名}（再点 N 张）」
                // 期望：吃累积提示「吃：1万 5万（再点 1 张）」、无 hint
                self.hand = parse_hand("123m 456m 789m 123s 5s");
                self.mode = MeldMode::Chi;
                self.click_tile(0); // 1万
                self.click_tile(4); // 5万 ⇒ pending 两张，还差 1 张
            }
            Ok("20") => {
                // 已知瑕疵复现（只登记不修）：吃累积期不 clear hint ⇒ 上一次的形状错误提示会「滞留」到下一次累积
                // 期望：hint 显示「吃：需同花色三张连续（如 2万3万4万）」，同时吃累积提示显示「吃：1万（再点 2 张）」
                self.hand = parse_hand("123m 456m 789m 123s 5s");
                self.mode = MeldMode::Chi;
                self.click_tile(0); // 1万
                self.click_tile(2); // 3万
                self.click_tile(4); // 5万 ⇒ 形状非法：置形状文案并清空 pending
                self.click_tile(0); // 再点 1万 ⇒ pending=[1万]，而 hint 未被清（滞留）
            }
            Ok("21") => {
                // L1 缺口（已由 L3-C 修复）：副露 5 组（真实 UI 可达：反复碰不同牌；per-tile 限 4 张但不限组数）
                // 期望（落定后的真实期望文案）：结果区显示「副露最多 4 组（当前 5 组），无法计算」、
                //   手牌区显示同一明确错误（不再出现「应为 0 张」这类静默饱和值）、
                //   确认按钮不可点（ready=false，即 expected_hidden()==None）、弱提示「（暗牌需 N 张）」不再出现、不 panic。
                self.hand = counts();
                self.melds = vec![
                    Meld::pon(0),
                    Meld::pon(1),
                    Meld::pon(2),
                    Meld::pon(3),
                    Meld::pon(4),
                ];
            }
            Ok("22") => {
                // L2：带副露的张数不符变体（exp=10；demo 10 只覆盖 exp=13）
                // 期望：「注意：暗牌应为 10 张（13 - 3×1 组副露；杠算 1 个面子），当前 12 张」
                //      +「暗牌张数不对，无法计算（应为 13 − 3×副露 张）」+「（暗牌需 10 张）」
                self.hand = parse_hand("123m 456m 789m 123s"); // 12 张（应 10）
                self.melds = vec![Meld::kan(26, false)]; // 暗杠 9筒
            }
            Ok("23") => {
                // R3：吃累积中点「×」删一副露 ⇒ `pending_chi` / `hint` 必须清空；
                // 期望读图：无「吃：…（再点 N 张）」进度行、无提示行、模式高亮仍在「吃」（半复位裁定）、不 panic。
                self.hand = parse_hand("123m 456m 789m 123s 5s"); // 13 张（删完副露后正好）
                self.melds = vec![Meld::pon(0)]; // 待删的那一组（碰 1万）
                self.mode = MeldMode::Chi;
                self.click_tile(0); // 吃：1万 ⇒ 产生 pending_chi=[0] 与进度 hint
                self.remove_meld(0); // 与 UI「×」同一条代码路径
            }
            _ => {
                self.hand = parse_hand("555p666p777p 555s 6s");
            }
        }
        self.recompute();
        self.dirty = std::env::var("GBMJ_DEMO").as_deref() == Ok("6");
    }

    /// 某张牌已被占用的张数（手牌 + 副露）
    fn used(&self, t: usize) -> usize {
        let mut n = self.hand[t] as usize;
        for m in &self.melds {
            for x in m.tiles() {
                if x == t {
                    n += 1;
                }
            }
        }
        n
    }

    /// 暗牌应有的张数：13 − 3×副露组数。
    /// 副露超过 4 组时 13 − 3n 下溢（旧实现用 saturating_sub 静默饱和成 0）⇒ 返回 None，
    /// 由调用方给出明确错误并让「确认」不可点，绝不把饱和出来的 0 当作正常预期值。
    fn expected_hidden(&self) -> Option<usize> {
        13usize.checked_sub(3 * self.melds.len())
    }

    fn recompute(&mut self) {
        match self.expected_hidden() {
            // 只有期望张数存在且暗牌张数吻合时才送进引擎
            Some(exp) if self.hand.iter().map(|&x| x as usize).sum::<usize>() == exp => {
                self.analysis = Some(analyze(
                    &self.hand,
                    &self.melds,
                    self.round_wind,
                    self.seat_wind,
                ));
            }
            // 副露 >4 组（None）或张数不符 ⇒ 不计算，绝不再把 5 组副露送进 analyze
            _ => self.analysis = None,
        }
    }

    /// 删除第 i 组副露：与「清空」保持一致的清理集合（`pending_chi`/`hint` 必须清），
    /// 但**按既有半复位裁定不复位 `mode`**。UI 的「×」与 `GBMJ_DEMO=23` 都走这里。
    fn remove_meld(&mut self, i: usize) {
        if i >= self.melds.len() {
            return;
        }
        self.melds.remove(i);
        self.pending_chi.clear();
        self.hint.clear();
        self.dirty = true;
    }

    fn click_tile(&mut self, t: usize) {
        match self.mode {
            MeldMode::None => {
                if self.used(t) >= 4 {
                    self.hint = format!("{} 已用满 4 张", tile_name(t));
                    return;
                }
                self.hand[t] += 1;
                self.hint.clear();
                self.dirty = true;
            }
            MeldMode::Pon => {
                if self.used(t) + 3 > 4 {
                    self.hint = format!("碰 {} 会超过 4 张上限", tile_name(t));
                    return;
                }
                self.melds.push(Meld::pon(t));
                self.hint.clear();
                self.dirty = true;
                self.mode = MeldMode::None; // 做成一组后复位
            }
            MeldMode::MingKan | MeldMode::AnKan => {
                let open = self.mode == MeldMode::MingKan;
                if self.used(t) + 4 > 4 {
                    self.hint = format!("杠 {} 会超过 4 张上限", tile_name(t));
                    return;
                }
                self.melds.push(Meld::kan(t, open));
                self.hint.clear();
                self.dirty = true;
                self.mode = MeldMode::None; // 做成一组后复位
            }
            MeldMode::Chi => {
                self.pending_chi.push(t);
                if self.pending_chi.len() == 3 {
                    let mut v = self.pending_chi.clone();
                    v.sort_unstable();
                    let suited = v[2] < 27 && v[0] / 9 == v[1] / 9 && v[1] / 9 == v[2] / 9;
                    let run = suited && v[1] == v[0] + 1 && v[2] == v[1] + 1;
                    if run {
                        let mut need = [0usize; NUM_TILES];
                        for &x in &v {
                            need[x] += 1;
                        }
                        if (0..NUM_TILES).all(|x| self.used(x) + need[x] <= 4) {
                            self.melds.push(Meld::chi(v[0]));
                            self.hint.clear();
                            self.dirty = true;
                            self.pending_chi.clear();
                            self.mode = MeldMode::None; // 做成一组后复位
                            return;
                        } else {
                            self.hint = "吃 超出 4 张上限".to_string();
                        }
                    } else {
                        self.hint = "吃：需同花色三张连续（如 2万3万4万）".to_string();
                    }
                    self.pending_chi.clear();
                }
            }
        }
    }

    fn meld_label(m: &Meld) -> String {
        match m.kind {
            gbmj::model::MeldKind::Chi => {
                let t = m.tile;
                format!(
                    "吃 {}{}{}",
                    tile_name(t),
                    tile_name(t + 1),
                    tile_name(t + 2)
                )
            }
            gbmj::model::MeldKind::Pon => format!("碰 {}×3", tile_name(m.tile)),
            gbmj::model::MeldKind::Kan => {
                if m.open {
                    format!("明杠 {}×4", tile_name(m.tile))
                } else {
                    format!("暗杠 {}×4", tile_name(m.tile))
                }
            }
        }
    }

    /// 副露组数超过 4 时的明确错误文案（不再出现「应为 0 张」这类静默饱和值）
    fn meld_cap_error(&self) -> String {
        format!("副露最多 4 组（当前 {} 组），无法计算", self.melds.len())
    }

    // ---------- UI 段 ----------

    fn ui_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_min_width(430.0);
                ui.strong("副露区");
                ui.horizontal(|ui| {
                    for (m, label) in [
                        (MeldMode::Chi, "吃"),
                        (MeldMode::Pon, "碰"),
                        (MeldMode::MingKan, "明杠"),
                        (MeldMode::AnKan, "暗杠"),
                    ] {
                        let sel = self.mode == m;
                        if ui.selectable_label(sel, label).clicked() {
                            self.mode = if sel { MeldMode::None } else { m };
                            self.pending_chi.clear();
                        }
                    }
                });
                if !self.pending_chi.is_empty() {
                    let names: Vec<String> =
                        self.pending_chi.iter().map(|&t| tile_name(t)).collect();
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 140, 0),
                        format!(
                            "吃：{}（再点 {} 张）",
                            names.join(" "),
                            3 - self.pending_chi.len()
                        ),
                    );
                }
                let mut remove: Option<usize> = None;
                for (i, m) in self.melds.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.small_button("×").clicked() {
                            remove = Some(i);
                        }
                        ui.label(Self::meld_label(m));
                    });
                }
                if let Some(i) = remove {
                    self.remove_meld(i);
                }
                if self.melds.is_empty() {
                    ui.weak("（无副露）");
                }
            });

            ui.add_space(18.0);

            ui.vertical(|ui| {
                ui.set_min_width(430.0);
                ui.horizontal(|ui| {
                    ui.strong(format!("手牌区（{} 张）", total(&self.hand)));
                    if ui.small_button("示例").clicked() {
                        self.load_sample();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let mut rm: Option<usize> = None;
                    for t in 0..NUM_TILES {
                        for _ in 0..self.hand[t] {
                            if ui
                                .button(tile_name(t))
                                .on_hover_text("点击移除一张")
                                .clicked()
                            {
                                rm = Some(t);
                            }
                        }
                    }
                    if let Some(t) = rm {
                        self.hand[t] -= 1;
                        self.dirty = true;
                    }
                });
                let now = total(&self.hand) as usize;
                match self.expected_hidden() {
                    // 副露 >4 组：明确错误，绝不显示「应为 0 张」
                    None => {
                        ui.colored_label(
                            egui::Color32::from_rgb(220, 90, 90),
                            self.meld_cap_error(),
                        );
                    }
                    Some(exp) if now != exp => {
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 120, 0),
                            format!(
                                "注意：暗牌应为 {} 张（13 - 3×{} 组副露；杠算 1 个面子），当前 {} 张",
                                exp,
                                self.melds.len(),
                                now
                            ),
                        );
                    }
                    Some(_) => {
                        ui.weak("√ 张数正确");
                    }
                }
            });
        });
    }

    fn ui_palette(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                for (start, label, n) in [
                    (0usize, "万", 9usize),
                    (9, "条", 9),
                    (18, "筒", 9),
                    (27, "字", 7),
                ] {
                    ui.horizontal(|ui| {
                        ui.add_sized([26.0, 34.0], egui::Label::new(label));
                        for k in 0..n {
                            let t = start + k;
                            let used = self.used(t);
                            let full = used >= 4;
                            let resp = tile_cell(ui, t, used);
                            if resp.clicked() && !full {
                                self.click_tile(t);
                            }
                            if full {
                                let _ = resp.on_hover_text("已用满 4 张");
                            }
                        }
                    });
                }
            });

            ui.add_space(18.0);

            ui.vertical(|ui| {
                ui.set_min_width(190.0);
                ui.strong("圈风 / 门风");
                ui.label("圈风");
                ui.horizontal(|ui| {
                    for (t, n) in [(27usize, "东"), (28, "南"), (29, "西"), (30, "北")] {
                        if ui.selectable_value(&mut self.round_wind, t, n).changed() {
                            self.dirty = true;
                        }
                    }
                });
                ui.label("门风");
                ui.horizontal(|ui| {
                    for (t, n) in [(27usize, "东"), (28, "南"), (29, "西"), (30, "北")] {
                        if ui.selectable_value(&mut self.seat_wind, t, n).changed() {
                            self.dirty = true;
                        }
                    }
                });
                ui.add_space(6.0);
                // 副露 >4 组（None）⇒ ready=false，确认不可点，也不显示「需 0 张」
                let ready = match self.expected_hidden() {
                    Some(exp) => total(&self.hand) as usize == exp,
                    None => false,
                };
                let confirm = egui::Button::new(
                    egui::RichText::new("确认")
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(52, 120, 200))
                .min_size(egui::vec2(72.0, 26.0));
                ui.horizontal(|ui| {
                    if ui.add_enabled(ready, confirm).clicked() {
                        self.recompute();
                        self.dirty = false;
                    }
                    if ui
                        .add(egui::Button::new("清空").min_size(egui::vec2(56.0, 26.0)))
                        .clicked()
                    {
                        self.hand = counts();
                        self.melds.clear();
                        self.pending_chi.clear();
                        self.hint.clear();
                        self.dirty = true;
                    }
                });
                match self.expected_hidden() {
                    // 副露 >4 组：明确错误，替代弱提示
                    None => {
                        ui.colored_label(
                            egui::Color32::from_rgb(220, 90, 90),
                            self.meld_cap_error(),
                        );
                    }
                    Some(exp) if !ready => {
                        ui.weak(format!("（暗牌需 {} 张）", exp));
                    }
                    Some(_) => {}
                }
                ui.add_space(6.0);
                let mq = self.melds.iter().all(|m| match m.kind {
                    gbmj::model::MeldKind::Chi | gbmj::model::MeldKind::Pon => false,
                    gbmj::model::MeldKind::Kan => !m.open,
                });
                ui.weak(if mq { "门清：是" } else { "门清：否" });
            });
        });
    }

    fn ui_results(&mut self, ui: &mut egui::Ui) {
        ui.strong("听牌 / 和牌计算");
        if self.dirty {
            ui.colored_label(
                egui::Color32::from_rgb(200, 140, 0),
                "已修改，点【确认】重新计算",
            );
            return;
        }
        let an = match &self.analysis {
            Some(a) => a,
            None => {
                // 区分两种「无结果」：副露 >4 组（明确错误）与暗牌张数不符（原弱提示）
                if self.expected_hidden().is_none() {
                    ui.colored_label(egui::Color32::from_rgb(220, 90, 90), self.meld_cap_error());
                } else {
                    ui.weak("暗牌张数不对，无法计算（应为 13 − 3×副露 张）");
                }
                return;
            }
        };
        if an.structural_waits.is_empty() {
            ui.weak("当前不是听牌型（无任何结构成和张）");
            return;
        }
        ui.weak(format!(
            "结构听牌 {} 种：{}",
            an.structural_waits.len(),
            an.structural_waits
                .iter()
                .map(|&t| tile_name(t))
                .collect::<Vec<_>>()
                .join(" ")
        ));
        ui.add_space(4.0);

        for w in &an.waits {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [52.0, 22.0],
                    egui::Label::new(egui::RichText::new(tile_name(w.tile)).strong()),
                );
                let cols = [
                    ("点和", w.normal, true),
                    ("自摸", w.tsumo, true),
                    ("点和绝张", w.last, w.can_last),
                    ("自摸绝张", w.tsumo_last, w.can_last),
                ];
                for (name, v, applicable) in cols {
                    if !applicable {
                        // 双碰/单钓等：暗牌里有该张，永远不成绝张
                        ui.colored_label(egui::Color32::from_gray(115), format!("{name} —"));
                        continue;
                    }
                    let ok = v >= WIN_THRESHOLD;
                    let color = if ok {
                        egui::Color32::from_rgb(235, 90, 80)
                    } else {
                        egui::Color32::from_gray(120)
                    };
                    ui.colored_label(color, format!("{name} {v}"));
                }
                let mark = match w.verdict() {
                    gbmj::calc::WinVerdict::Win => {
                        if w.only_last() {
                            "（仅绝张可和）"
                        } else if w.only_tsumo() {
                            "（仅自摸可和）"
                        } else {
                            ""
                        }
                    }
                    gbmj::calc::WinVerdict::ConditionalLast => "条件可和（仅非绝张·点和）",
                    gbmj::calc::WinVerdict::NoWin => "不可和",
                };
                if !mark.is_empty() {
                    let color = if w.verdict() == gbmj::calc::WinVerdict::NoWin {
                        egui::Color32::from_gray(120)
                    } else {
                        egui::Color32::from_rgb(200, 160, 60)
                    };
                    ui.colored_label(color, mark);
                }
            });
            let fmt_fans = |list: &[gbmj::fans::Fan]| {
                if list.is_empty() {
                    "（无番）".to_string()
                } else {
                    list.iter()
                        .map(|f| format!("{} {}", f.name, f.value))
                        .collect::<Vec<_>>()
                        .join("，")
                }
            };
            // 去掉和法番后比较：判断自摸是否只是「门前清 2 ↔ 不求人 4」的换算法
            let structure_of = |list: &[gbmj::fans::Fan]| -> Vec<(String, u32)> {
                list.iter()
                    .filter(|f| !matches!(f.name, "门前清" | "不求人" | "自摸" | "和绝张"))
                    .map(|f| (f.name.to_string(), f.value))
                    .collect()
            };
            let tsumo_differs = structure_of(&w.fans) != structure_of(&w.fans_tsumo);
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.weak(fmt_fans(&w.fans));
            });
            if tsumo_differs {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.weak(format!("自摸：{}", fmt_fans(&w.fans_tsumo)));
                });
            }
            ui.add_space(4.0);
        }
    }

    fn ui_hint(&self, ui: &mut egui::Ui) {
        if !self.hint.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(220, 90, 90), &self.hint);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 不再自动重算：任何改动都只标记 dirty，必须点「确认」才出结果
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.ui_header(ui);
                ui.separator();
                self.ui_palette(ui);
                self.ui_hint(ui);
                ui.separator();
                self.ui_results(ui);
            });
        });
    }
}

/// 牌库格子：自绘，三态颜色拉开（未用 / 部分用 / 用满）
fn tile_cell(ui: &mut egui::Ui, t: usize, used: usize) -> egui::Response {
    let full = used >= 4;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(46.0, 36.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let (bg, fg) = if full {
            // 用满：深红底 + 淡红字（与浅灰的“未用”反差极大）
            if hovered {
                (
                    egui::Color32::from_rgb(74, 32, 32),
                    egui::Color32::from_rgb(255, 200, 200),
                )
            } else {
                (
                    egui::Color32::from_rgb(44, 24, 24),
                    egui::Color32::from_rgb(214, 138, 138),
                )
            }
        } else if used > 0 {
            // 已用部分：黄底黑字
            if hovered {
                (
                    egui::Color32::from_rgb(232, 188, 78),
                    egui::Color32::from_rgb(24, 18, 6),
                )
            } else {
                (
                    egui::Color32::from_rgb(202, 154, 50),
                    egui::Color32::from_rgb(24, 18, 6),
                )
            }
        } else if hovered {
            (
                ui.visuals().widgets.hovered.bg_fill,
                ui.visuals().widgets.hovered.text_color(),
            )
        } else {
            (
                ui.visuals().widgets.inactive.bg_fill,
                ui.visuals().text_color(),
            )
        };
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(4), bg);
        let cx = rect.center().x;
        ui.painter().text(
            egui::pos2(cx, rect.top() + 11.0),
            egui::Align2::CENTER_CENTER,
            tile_name(t),
            egui::FontId::proportional(13.0),
            fg,
        );
        let sub = if full {
            "满".to_string()
        } else {
            format!("{}/4", used)
        };
        ui.painter().text(
            egui::pos2(cx, rect.bottom() - 9.0),
            egui::Align2::CENTER_CENTER,
            sub,
            egui::FontId::proportional(10.5),
            fg,
        );
    }
    resp
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for path in [r"C:\Windows\Fonts\simhei.ttf", r"C:\Windows\Fonts\msyh.ttc"] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("国标麻将算番器"),
        ..Default::default()
    };
    eframe::run_native(
        "国标麻将算番器",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
