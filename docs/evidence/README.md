# GUI 点验证据目录

本目录存放 **GUI 点验（人工/半自动视觉核验）的证据图与真值文件**，供独立复核 `docs/测试评估报告.md`
中关于界面状态与输入守卫的判定（§二十二 / §二十二.9 / §二十三 / §二十四）。

> **性质声明**：这些文件是**点验证据**，**不是自动化测试**。本项目**本轮不引入任何 UI 自动化测试框架**
> （`Q9` / `C10` 口径不变：不使用 SendKeys、UI Automation 或播放/回放框架；`Cargo.toml` 未增依赖）。
> 截图由「脚本化夹具 + 窗口级截屏」确定性生成，**判定仍由人（或读图模型）完成**。

## 一、命名规范（重要）

- 一律 `docs/evidence/<描述性名>.png|txt`。
- **严禁 `_` 前缀**：仓库根 `.gitignore` 有**全局模式 `_*.png`**（`_*.txt` 同样面临此风险），
  带下划线的截图/文本会被忽略而**无法入库**——这正是「引用是死链」的历史成因。
  若生成脚本产出 `_` 前缀文件，**请改名**，**不要**去改 `.gitignore`。
- 本目录下 25 个文件已**逐一实测未被忽略**（可用 `git check-ignore -v <file>` 复核，期望无输出）。

## 二、生成方式（可复现）

前置：夹具已入库（`GBMJ_DEMO` 分支位于 `src/bin/gbmj_gui.rs` 的 `load_sample()`，编号 1~15）。

1. **先重建**（陈旧产物 = 假证据）：
   ```powershell
   cargo build --release --bin gbmj-gui      # 不要加 --no-default-features（GUI 需 gui feature）
   ```
2. **逐夹具启动 + 窗口级截屏**（禁止整屏 BitBlt）：
   ```powershell
   $env:GBMJ_DEMO = '7'                       # 1..15 逐个
   $p = Start-Process .\target\release\gbmj-gui.exe -PassThru
   # 轮询等待 $p.MainWindowHandle 非 0，再稳定 ~1.1s
   # GetWindowRect 取窗口矩形（本机 1136x799）；Bitmap + Graphics.FromImage + GetHdc
   # PrintWindow(hwnd, hdc, 2)  ← PW_RENDERFULLCONTENT，只抓本窗口，不含桌面内容
   # 存为 docs/evidence/demo-NN-<slug>.png；随后 Stop-Process 关闭进程
   ```
3. **放大裁剪**（供人眼复核数值与状态列）：
   - `crop-demoN-lower2x.png`：源矩形 **x=0, y=265, w=1136, h=430**（结果区带、全宽）→ 2× ⇒ **2272×860**。
   - `crop-demoN-rowtail4x.png`：源矩形 **x=0, y=265, w=576, h=430**（同一结果区带的左半，含行尾/状态列）→ 4× ⇒ **2304×1720**。
   - 尺寸与报告 **§22.1「放大裁剪」清单**记录的尺寸一致；缩放用最近邻（`NearestNeighbor`）以免糊字。
4. **探针图**：600×300、**白底 + 左半红矩形（x=20..259, y=20..279, RGB 255,0,0）+ 中央黑色 4 位随机数字 + 右下蓝方块（x=520..579, y=220..279, RGB 0,0,255）**；
   真值单独存 `*-truth.txt`（格式：UTF-8 BOM + `code=NNNN` + CRLF，14 B），**不展示给读图方**。
5. **每次截图前重建**；**用完 `Stop-Process`**（防僵尸窗口）。运行期 hwnd 每次不同，故只记录在当次回报中，不写入本文件。

## 三、文件 ↔ 报告引用/判定 映射

报告 **§22.1「放大裁剪」清单**使用**花括号记法** `_crop_demo{1,4}_rowtail4x.png` 表示**两个文件**，下表已**逐条展开为真实文件名**。

| 文件 | 生成方式 | 对应报告引用处 | 该图要证的判定 |
|---|---|---|---|
| `probe.png` | 探针图（**同构重建**，真值 `1489`） | §22.2 `gui-probe`（报告原名 `_probe.png`） | 读图方**读出 `1489`** + 左红/右下蓝 ⇒ 读图链路有效、非猜测 |
| `probe-truth.txt` | 真值（`code=1489`） | §22.2（报告原名 `_probe_truth.txt`） | 与 `probe.png` 互证（真值不展示给读图方） |
| `probe2.png` | 探针图（**T14 原件，字节保留**，真值 `6663`） | §22.9 `gui-probe2`（报告原名 `_probe2.png`） | 第二批探针：读出 `6663` + 左红/右下蓝 ⇒ 可复现、非常识命中 |
| `probe2-truth.txt` | 真值（`code=6663`） | §22.9（报告原名 `_probe2_truth.txt`） | 与 `probe2.png` 互证 |
| `cli-baseline.txt` | CLI 直接输出（`HEAD=b1cd00f`，4 例） | §二十二 / `UI-07` 交叉核对 | GUI 与 CLI 在**同语义**下数值/番名/状态一致（措辞形式允许不同） |
| `demo-01-baseline.png` | `GBMJ_DEMO=1` | §22.1（`_gui_demo1.png`）/ §22 正文「6 张 demo1..6」 | 门清基线；含 `—` 空栏的表格渲染 |
| `demo-02-melds.png` | `GBMJ_DEMO=2` | §22.1（`_gui_demo2.png`）——**仅被 §22 泛指覆盖**（T12 的 L3 `gui-demo-b` 曾逐张读取，未在报告中单列结论） | 带副露（碰 + 吃）渲染；`7筒 19/20` |
| `demo-03-meld-reset.png` | `GBMJ_DEMO=3` | §22.1（`_gui_demo3.png`）——**仅被 §22 泛指覆盖**（同上，逐张读取未单列） | 脚本化点击后**副露按钮复位**（状态机） |
| `demo-04-bunnyaku.png` | `GBMJ_DEMO=4` | §22.1（`_gui_demo4.png`） | 四归一自检；含**不可和**行（`2万 6/7 ✘`、`3万 7/8 11/12 ✔仅自摸`） |
| `demo-05-double-pung.png` | `GBMJ_DEMO=5` | §22.1（`_gui_demo5.png`）——**仅被 §22 泛指覆盖**（T12 的 L3 `gui-demo-a` 曾逐张读取，未在报告中单列结论） | 双碰听 → 点和/自摸番表不同（`6条 12/28`、`东 8/24`） |
| `demo-06-dirty.png` | `GBMJ_DEMO=6` | §22.1（`_gui_demo6.png`） | 改动后未确认 → `已修改，点【确认】重新计算` |
| `crop-demo1-lower2x.png` | 由 `demo-01-baseline.png` 裁剪 2× | §22.1「放大裁剪」清单 | 结果区带放大，供人眼复核 demo1 的数值/`—` 栏 |
| `crop-demo4-lower2x.png` | 由 `demo-04-bunnyaku.png` 裁剪 2× | §22.1「放大裁剪」清单 | 同上，复核 demo4 的不可和行 |
| `crop-demo1-rowtail4x.png` | 由 `demo-01-baseline.png` 裁剪 4× | §22.1「放大裁剪」清单（花括号展开第 1 个） | **行尾/状态列** 4× 放大，供人眼读状态 |
| `crop-demo4-rowtail4x.png` | 由 `demo-04-bunnyaku.png` 裁剪 4× | §22.1「放大裁剪」清单（花括号展开第 2 个） | 同上 |
| `demo-07-conditional.png` | `GBMJ_DEMO=7` | §22.9（T14）/ `UI-07` | `6条 … 不可和`、`9条 … 条件可和（仅非绝张·点和）`（异常态才标注） |
| `demo-08-noten.png` | `GBMJ_DEMO=8` | §22.9（T14） | 结果区 `当前不是听牌型（无任何结构构成和张）`（GUI 文案；CLI 为「未听牌（无和牌型）」，措辞允许不同） |
| `demo-09-only-last.png` | `GBMJ_DEMO=9` | §22.9（T14） | `6万 4/6/8/10`、`9万 5/7/9/11`，**两行均 `（仅绝张可和）`** |
| `demo-10-invalid-count.png` | `GBMJ_DEMO=10` | §22.9（T14） | 非法输入：`暗牌张数不对，无法计算（应为 13 − 3×副露 张）`，**不崩溃** |
| `demo-11-tile-full.png` | `GBMJ_DEMO=11` | **§二十四**（T17 夹具的点验） | 守卫「1万 已用满 4 张」（`click_tile` 的 `MeldMode::None` 分支）。**⚠️ 真实 UI 不可达**：牌库点击门禁 `if resp.clicked() && !full` 会吞掉满格点击 ⇒ 只能由夹具直接调用触达；**保留作回归保护**（用户裁定） |
| `demo-12-pon-over4.png` | `GBMJ_DEMO=12` | **§二十四**（T17 夹具的点验） | 守卫「碰 2万 会超过 4 张上限」（`MeldMode::Pon` 分支）。**真实 UI 可达**（目标牌未满，点击可通过门禁） |
| `demo-13-kan-over4.png` | `GBMJ_DEMO=13` | **§二十四**（T17 夹具的点验） | 守卫「杠 2万 会超过 4 张上限」（`MingKan` / `AnKan` 分支）。**真实 UI 可达** |
| `demo-14-chi-shape.png` | `GBMJ_DEMO=14` | **§二十四**（T17 夹具的点验） | 守卫「吃：需同花色三张连续（如 2万3万4万）」（`Chi` 形状校验）。**真实 UI 可达** |
| `demo-15-chi-over4.png` | `GBMJ_DEMO=15` | **§二十四**（T17 夹具的点验） | 守卫「吃 超出 4 张上限」（`Chi` 四张上限校验）。**⚠️ 真实 UI 不可达**：需点击已满格（`used>=4`）的牌，被同一门禁 `if resp.clicked() && !full` 吞掉 ⇒ 仅夹具可触达；**保留作回归保护**（用户裁定） |
| `probe3.png` / `probe3-truth.txt` | 测试方生成（真值 `code=5350`） | **§二十五**（T22 续做夹具的点验） | 读图链路探针：L3 读出 `5350` + 左红/右下蓝 ⇒ 读图有效、非常识命中 |
| `demo-16-ankan-ok.png` | `GBMJ_DEMO=16` | **§二十五**（T22 续做夹具的点验） | `AnKan` 成功：`暗杠 9筒×4`、`门清：是`、`√ 张数正确`、**无 hint**（红 0px） |
| `demo-17-ankan-guard.png` | `GBMJ_DEMO=17` | **§二十五** | `AnKan` 守卫「杠 1万 会超过 4 张上限」（与明杠共用同一判定语句） |
| `demo-18-add-tile-ok.png` | `GBMJ_DEMO=18` | **§二十五** | `MeldMode::None` 成功加牌：12→13 张、`√ 张数正确`、出四栏、无 hint |
| `demo-19-chi-progress.png` | `GBMJ_DEMO=19` | **§二十五** | `Chi` 累积中态：`吃：1万 5万（再点 1 张）`、无 hint（该标签颜色**未做像素取色**，见 §25.5） |
| `demo-20-stale-hint.png` | `GBMJ_DEMO=20` | **§二十五** | **陈旧 hint 滞留（只登记不修）**：形状错误文案与 `吃：1万（再点 2 张）` 同时出现 |
| `demo-21-meld-cap.png` | `GBMJ_DEMO=21` | **§二十五** | 副露 5 组：`副露最多 4 组（当前 5 组），无法计算` 出现**三处**；不 panic、无「应为 0 张」 |
| `demo-22-hidden-count-with-meld.png` | `GBMJ_DEMO=22` | **§二十五** | 带副露张数不符变体：`注意：暗牌应为 10 张（13 - 3×1 组副露；杠算 1 个面子），当前 12 张` + `（暗牌需 10 张）` + `暗牌张数不对，无法计算（应为 13 − 3×副露 张）` |
| `probe4.png` / `probe4-truth.txt` | 测试方生成（真值 `code=1817`） | **§二十六**（T24 代跑） | 读图探针：L3 读出 `1817` + 左红/右下蓝 ⇒ 读图链路有效 |
| `t24-h0-no-hover.png` / `t24-h0b-control-after.png` | T24 代跑（`GBMJ_DEMO=11`，无悬停） | **§二十六** | **悬停阴性对照**：无提示框（其它悬停图与之比对） |
| `t24-h1-hover-full-tile.png` | T24 代跑（悬停满格 `1万`） | **§二十六** | 实测 tooltip **`已用满 4 张`**（L3 逐字 + 像素 +1256 B） |
| `t24-h2-hover-hand-tile.png` | T24 代跑（悬停落点为「暗杠」按钮） | **§二十六** | **未出现 tooltip**（仅按钮悬停高亮）⇒ 说明该处无 `on_hover_text` |
| `t24-h3-hover-confirm.png` / `t24-h4-hover-mode-btn.png` | T24 代跑（悬停「确认」/模式按钮） | **§二十六** | **sha 同基线、无变化** ⇒ 该两处无 tooltip（与代码仅两处 `on_hover_text` 吻合） |
| `t24-h5-hand-candidate1/2/3.png` | T24 代跑（悬停手牌区 3 处） | **§二十六** | candidate1/2 实测 tooltip **`点击移除一张`**（含悬停高亮框）；candidate3 无 |
| `t24-m1-synth-click-hover-only.png` / `t24-m2-synth-click-confirm-{before,after}.png` | T24 代跑（**合成点击无效**留证） | **§二十六** | 证明 `PostMessage` 无效、`mouse_event` 仅产生**悬停态**变化（**不构成激活**）⇒ 该项**仍需人工** |
| `t24-r1-resize-800x600.png` / `t24-r2-resize-1280x900.png` | T24 代跑（`SetWindowPos`） | **§二十六** | 两档尺寸布局正常、无裁切/重叠、文字可读（L3 双图确认） |
| `demo-23-delete-meld-pending.png` | `GBMJ_DEMO=23`（T32 批；真人点击路径见 `docs/GUI人工点验清单.md` 的 **R3 专项**） | **§三十二**（T31/T32 R3 修复验证） | `×` 删副露后的状态：副露区「（无副露）」、**吃按钮仍高亮**、**无进度行**、**无红/黄提示行**（L3 盲转写 + 像素双证） |
| `diff-cli-cases.tsv` | T34 生成（测试方 / `scripts/diff-cli.ps1` 的输入） | **§三十三**（T34 差分覆盖扩展） | **可重复差分用例集：269 例**（`id ⇥ 来源 ⇥ 期望 ⇥ args`；来源含 BASELINE 9 / GBMJ_DEMO 21 / `tests/*.rs` 143 / 全番型样本 73 / `--plan` 10 / 预期差异 5 / 手工构造 7 + 更正 1，均可追溯） |
| `diff-cli-results.tsv` | T34 生成（`diff-cli.ps1` 输出） | **§三十三** | **逐例差分结果**（verdict / exitOld-New / stdout·stderr 的 sha / 耗时）⇒ 末轮 **264 零差异 + 5 预期差异 + 0 意外差异**（exit 0） |
| `diff-cli-summary.txt` | T34 生成（`diff-cli.ps1` 输出） | **§三十三** | **差分摘要**：旧/新二进制路径 + mtime + sha256、用例总数、零/预期/意外计数、最慢 5 例、噪声剥离规则说明 |
| `diff-cli-fan-coverage.txt` | T34 生成（测试方重放全部用例后实测 stdout） | **§三十三** | **番型覆盖清单**：分母 A=73（静态表）/ B=77（含 4 个动态番名），**覆盖 71/73**，未覆盖 2 项及结构性原因 + 全文清单 |

**泛指表述的对应**：报告 §22 正文有「6 张 demo1..6」「全部 10 图」这类**泛指**写法 ⇒ 对应本目录
`demo-01`~`demo-06` 与 `demo-01`~`demo-10` 的**全套**；其中 `demo-02/03/05` **仅被 §22 泛指覆盖**（无单列结论，已在表中如实标注）；
`demo-11`~`demo-15` 的**逐条结论**见报告 **§二十四**（T17 夹具的独立点验）。**本目录一律按内容引用（函数/分支/文案），不写行号**（行号会随插入漂移，见手册 §9.3；`scripts\gate.ps1` 已加 advisory 回扫自动提示）。

> **2026-10-12 更正（T20 / `task-28`，测试方经授权修正本文件）**：本表原把 `demo-11`~`demo-15` 指向「§二十三（T17）」——
> **错指**：报告 `## 二十三` 实为 T15（`U-14`/`U-15`/幻影）章节，且报告内**从无 T17 章节**（T17 交付的是**夹具代码** +
> dev 的夹具自证，**不构成独立点验**）。现改指 **§二十四**；同时把守卫/颜色的**源码行号引用改为内容引用**
> （守卫按**文案 + `click_tile` 分支**定位，提示色按 **`from_rgb(220,90,90)`** 定位）——原表所引的行号（守卫与颜色）
> **均已随夹具插入漂移**，这也是本文件不再写行号的原因。

## 四、重建与保真说明（诚实声明）

- `probe2.png` / `probe2-truth.txt`：**T14 原件，字节保留**（`probe2.png` 600×300、真值文件 14 B `BOM+code=6663+CRLF`）。
- `probe.png` / `probe-truth.txt`：**同构重建**——原 `_probe.png`/`_probe_truth.txt` 已在更早的临时产物清理中删除。
  真值 `1489` 由报告 **§22.2「读图探针」**的结论固化；本目录按 `probe2.png` 的**几何与配色镜像**重绘（红矩形 20,20,240,260 / 蓝方块 520,220,60,60），
  真值文件采用与 `probe2-truth.txt` **完全相同的字节格式**（BOM + CRLF，14 B）。
- 4 张 `crop-*`：**重建**——原物已删除，报告仅记录尺寸（`lower2x` 2272×860、`rowtail4x` 2304×1720）。
  本目录按第二节所列**源矩形**从库内 `demo-01`/`demo-04` 截图裁出，尺寸与报告记录一致；**源矩形选择是本次约定的**（报告未记录原矩形），
  如需与原始裁剪逐像素对比需回溯当时的裁图脚本（已不可得）。
- `demo-01`~`demo-15`：**本轮重新生成**（原图已删除；生成方式见第二节，确定性可复现）。

## 五、「已脚本化」与「仍须人工」的边界

- **已脚本化（本目录取证范围）**：`demo-01`~`demo-15` 共 15 个界面状态；其中 `demo-10`、`demo-11`~`demo-15`
  覆盖 **6 条非法输入路径**（张数不符、点牌已用满 4、碰超 4、杠超 4、吃形状非法、吃超 4）。
- **仍须人工（不在本目录范围）**：① 副露按钮**真实点击**与高亮复位的渲染（`demo-03` 仅覆盖状态机）；
  ② 「确认」「清空」按钮点击；③ 窗口缩放 / DPI；④ 悬停提示；⑤ 其它需真实鼠标事件的交互。
  这些保持人工点验，**不得**为此引入自动化框架。（用户 2026-10-12 裁定：人工类项豁免、不再作为缺口跟踪；`C10` 不变）

## 六、复核方法（独立复核者）

```powershell
# 1) 证据文件均已入库且未被忽略（期望：无输出）
Get-ChildItem docs/evidence -File | ForEach-Object { git check-ignore -v $_.FullName }
# 2) 已入库清单
git ls-files docs/evidence
# 3) 重新生成并逐字对比（可选，见第二节）
cargo build --release --bin gbmj-gui
# 4) 引擎侧护栏不受本目录影响
cargo test --no-default-features        # 期望 160 passed / 0 failed
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\gate.ps1   # 期望 PASS 8 / WARN 0 / FAIL 0
```
