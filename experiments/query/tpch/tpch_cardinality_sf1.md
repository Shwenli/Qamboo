# TPC-H SF=1 总基数对比：Qamboo 优化计划 vs 未优化计划（均为 oblivious，精确静态值）

## 口径定义

**"执行过程中参与的总行数" = 流水线每个关系算子阶段的输出基数之和**（对每个查询求和）。

两侧都是 oblivious 执行，语义相同、数值精确静态（只取决于模式，不需要执行）：

- `filter` / 列级谓词：输出行数不变（只更新 `valid` 列），计 n。同一张表同一位置的多个谓词合并为一个 filter 阶段。
- `inner_join(L_m, R_n)`：输出行数 = **右表行数 n**（FromLtoR，`operator/src/join.rs`；封装 `table/src/table_operator/table_join.rs`）。
- `semi_join` / `anti_join`：左表行数不变，计 m。
- `group_by` + 聚合：输出 = 输入行数 n（`operator/src/group_by.rs`）。
- `order_by`（radix sort）：输出 = n。
- `.head(k)`（secure cut）：物理截断到公开上界 k，之后算子作用于 k 行。
- 只计 main 中 MPC 执行路径；验证段的 `order_by("valid")` + `head(open_valid)`（数据依赖截断）不计入（Q3、Q8 主路径内含 `order_by("valid")`，已计入）。

**两侧唯一的差别是查询计划**：

- **Qamboo 列**：`experiments/query/tpch/qN.rs` 主版本，带全部三个优化——join reorder（小维表链式 join、中间结果逐步放大）、secure cut（group-by 后 `head()` 截断到组数公开上界）、semi-join（Q4 的 EXISTS）。
- **未优化列**：同一 oblivious 算子集，但 join 按 FROM 书写顺序左深执行（大表尽早成为右表）、无 secure cut、无 semi-join。优先采用仓库自带的消融变体作为权威依据：`experiments/query/optimization/no_join_reorder/`（Q2/Q5/Q7/Q8/Q9/Q10）、`no_secure_cut/`（Q2/Q3/Q5/Q8/Q13/Q17/Q18/Q20/Q21）、`no_semi_join/`（Q4）；同时受多个优化的查询按变体差异叠加推导（见各查询备注）。无变体且不涉及三优化的查询（Q6/Q12/Q14/Q15/Q16/Q19/Q22）两侧相同。

## SF=1 表大小

| 表 | 行数 | 符号 |
|---|---|---|
| lineitem | 6,000,000 | L |
| orders | 1,500,000 | O |
| customer | 150,000 | C |
| part | 200,000 | P |
| supplier | 10,000 | S |
| partsupp | 800,000 | PS |
| nation | 25 | N |
| region | 5 | R |

（`experiments/src/tpch_database_gen.rs` 的 `get_*_table_size`，与标准 TPC-H 一致。）

## Qamboo（优化计划）阶段序列与总行数

| Q | 阶段输出行数序列（主路径） | 符号合计 | 总行数 |
|---|---|---|---|
| 1 | filter(L), groupby(L), head(6) | 2L + 6 | 12,000,006 |
| 2 | filter(P), filter(R), join(R,N)→N, join(N,S)→S, join(S,PS)→PS, join(P,PS)→PS, groupby(PS), head(P), join(P,PS)→PS, filter(PS), sort(PS)×4 | 9PS + 2P + S + N + R | 7,610,030 |
| 3 | filter(C), filter(O), filter(L), join(C,O)→O, join(O,L)→L, groupby(L), head(O), sort(O)×3 | 3L + 6O + C | 27,150,000 |
| 4 | filter(L), filter(O), semi(O,L)→O, groupby(O), sort(O) | L + 4O | 12,000,000 |
| 5 | filter(O), filter(R), join(R,N)→N, join(N,S)→S, join(S,L)→L, join(C,O)→O, join(O,L)→L, filter(L), groupby(L), head(N), sort(N) | 4L + 2O + 2N + S + R | 27,010,055 |
| 6 | filter(L) | L | 6,000,000 |
| 7 | filter(L), join(N,C)→C, join(C,O)→O, join(N,S)→S, join(S,L)→L, join(O,L)→L, filter(L), groupby(L) | 5L + O + C + S | 31,660,000 |
| 8 | filter(R), filter(P), filter(O), join(R,N)→N, join(N,C)→C, join(C,O)→O, join(N,S)→S, join(P,L)→L, join(S,L)→L, join(O,L)→L, groupby(L), head(O), sort(O) | 4L + 4O + P + C + S + N + R | 30,360,030 |
| 9 | filter(P), join(N,S)→S, join(P,PS)→PS, join(S,PS)→PS, join(O,L)→L, join_multikey(PS,L)→L, groupby(L), sort(L)×2 | 5L + 2PS + P + S | 31,810,000 |
| 10 | filter(O), filter(L), join(N,C)→C, join(C,O)→O, join(O,L)→L, groupby(L), sort(L) | 4L + 2O + C | 27,150,000 |
| 11 | filter(N), join(N,S)→S, join(S,PS)→PS, groupby(PS), having_filter(PS), sort(PS) | 4PS + S + N | 3,210,025 |
| 12 | filter(L), join(O,L)→L, groupby(L) | 3L | 18,000,000 |
| 13 | filter(O), groupby(O), head(C), join(C,C)→C, groupby(C), sort(C)×2 | 2O + 5C | 3,750,000 |
| 14 | filter(L), join(P,L)→L, filter(L) | 3L | 18,000,000 |
| 15 | filter(L), groupby(L), sort(L)×2（求 max）, join(S,L)→L, filter(L), sort(L) | 7L | 42,000,000 |
| 16 | filter(P), filter(S), anti(PS,S)→PS, join(P,PS)→PS, groupby+count-distinct(PS), sort(PS) | 4PS + P + S | 3,410,000 |
| 17 | filter(P), join(P,L)→L, groupby(L), head(P), join(P,L)→L, filter(L) | 4L + 2P | 24,400,000 |
| 18 | groupby(L), head(O), filter(O), join(C,O)→O, join(O,O)→O, groupby(O), sort(O)×2 | L + 7O | 16,500,000 |
| 19 | filter(L), filter(P), join(P,L)→L, filter(L) | 3L + P | 18,200,000 |
| 20 | filter(P), join(P,PS)→PS, filter(L), groupby(L), head(PS), join_multikey(PS,PS)→PS, filter(PS), filter(N), join(N,S)→S, semi(S,PS)→S, sort(S) | 2L + 4PS + P + 3S + N | 15,430,025 |
| 21 | filter(N), filter(O), filter(L), groupby(L)×2, head(O), groupby(L), head(O), join(O,L)→L×2, filter(L), join(N,S)→S, join(S,L)→L, join(O,L)→L, groupby(L), head(S), sort(S) | 10L + 3O + 3S + N | 64,530,025 |
| 22 | filter(C), filter(C), anti(C,O)→C, groupby(C) | 4C | 600,000 |
| **合计** | | | **440,780,196** |

secure cut 上界：Q1=6（写死常数），Q2=P，Q3=O，Q5=N，Q8=O，Q13=C，Q17=P，Q18=O，Q20=PS，Q21=O×2 与 S。

## 未优化计划（oblivious）阶段序列与总行数

| Q | 阶段输出行数序列 | 符号合计 | 总行数 | 依据 |
|---|---|---|---|---|
| 1 | filter(L), groupby(L)（无 head(6)） | 2L | 12,000,000 | 去 secure cut |
| 2 | filter(P), filter(R), join(P,PS)→PS, join(S,PS)→PS, join(N,PS)→PS, join(R,PS)→PS, groupby(PS), join(PS,PS)→PS, filter(PS), sort(PS)×4 | 11PS + P + R | 9,000,005 | no_join_reorder 变体 + 去 head(P)（回连输出仍为右表 PS，数值不变） |
| 3 | filter(C), filter(O), filter(L), join(C,O)→O, join(O,L)→L, groupby(L), sort(L)×3 | 6L + 2O + C | 39,150,000 | no_secure_cut 变体（join 顺序本就等于 FROM 序） |
| 4 | filter(L), filter(O), join(O,L)→L, groupby(L)（semi 退化为裸 inner join；变体未实现末尾 order_by） | 3L + O | 19,500,000 | no_semi_join 变体 |
| 5 | filter(O), filter(R), join(C,O)→O, join(O,L)→L, join(S,L)→L, join(N,L)→L, join(R,L)→L, filter(L), groupby(L), sort(L) | 7L + 2O + R | 45,000,005 | no_join_reorder 链 + 去 head(N)（sort 在 L 上） |
| 6 | filter(L) | L | 6,000,000 | 无优化适用，同左 |
| 7 | filter(L), join(S,L)→L, join(O,L)→L, join(C,L)→L, join(N,L)→L, join(N,L)→L, filter(L), groupby(L) | 8L | 48,000,000 | no_join_reorder 变体 |
| 8 | filter(R), filter(P), filter(O), join(P,L)→L, join(S,L)→L, join(O,L)→L, join(C,L)→L, join(N,L)→L, join(R,L)→L, join(N,L)→L, groupby(L), sort(L) | 9L + O + P + R | 55,700,005 | no_join_reorder 链（7 join 全→L）+ 去 head(O)（主路径 sort("valid") 在 L 上） |
| 9 | filter(P), join(S,L)→L, join_multikey(PS,L)→L, join(P,L)→L, join(O,L)→L, join(N,L)→L, groupby(L), sort(L)×2 | 8L + P | 48,200,000 | no_join_reorder 变体 |
| 10 | filter(O), filter(L), join(C,O)→O, join(O,L)→L, join(N,L)→L, groupby(L), sort(L) | 4L + 2O | 27,000,000 | no_join_reorder 变体 |
| 11 | filter(N), join(S,PS)→PS, join(N,PS)→PS, groupby(PS), having_filter(PS), sort(PS) | 5PS + N | 4,000,025 | 无变体；按 FROM 序（partsupp,supplier,nation）左深推导 |
| 12 | filter(L), join(O,L)→L, groupby(L) | 3L | 18,000,000 | 同左 |
| 13 | filter(O), groupby(O), join(C,O)→O, groupby(O), sort(O)×2 | 6O | 9,000,000 | no_secure_cut 变体（无 head(C)，后续全在 O 上） |
| 14 | filter(L), join(P,L)→L, filter(L) | 3L | 18,000,000 | 同左 |
| 15 | filter(L), groupby(L), sort(L)×2, join(S,L)→L, filter(L), sort(L) | 7L | 42,000,000 | 同左（无三优化适用） |
| 16 | filter(P), filter(S), anti(PS,S)→PS, join(P,PS)→PS, groupby+count-distinct(PS), sort(PS) | 4PS + P + S | 3,410,000 | 同左（anti-join 不在消融范围） |
| 17 | filter(P), join(P,L)→L, groupby(L), join(L,L)→L, filter(L) | 4L + P | 24,200,000 | no_secure_cut 变体（无 head(P)，回连两侧皆 L） |
| 18 | groupby(L), filter(L), join(C,O)→O, join(O,L)→L, groupby(L), sort(L)×2 | 5L + O | 31,500,000 | no_secure_cut 变体（无 head(O)） |
| 19 | filter(L), filter(P), join(P,L)→L, filter(L) | 3L + P | 18,200,000 | 同左 |
| 20 | filter(P), join(P,PS)→PS, filter(L), groupby(L), join_multikey(PS,L)→L, filter(L), filter(N), join(N,S)→S, semi(S,L)→S, sort(S) | 3L + PS + P + 3S + N | 19,030,025 | no_secure_cut 变体（无 head(PS)，多键回连右表为 L） |
| 21 | filter(N), filter(O), filter(L), groupby(L)×3, join(L,L)→L×2, filter(L), join(N,S)→S, join(S,L)→L, join(O,L)→L, groupby(L), sort(L) | 11L + O + S + N | 67,510,025 | no_secure_cut 变体（去 3 处 head） |
| 22 | filter(C), filter(C), anti(C,O)→C, groupby(C) | 4C | 600,000 | 同左 |
| **合计** | | | **565,000,090** | |

## 对比总表

| Q | Qamboo（优化） | 未优化（oblivious） | 未优化/优化 | 主要获益来源 |
|---|---:|---:|---:|---|
| 1 | 12,000,006 | 12,000,000 | 1.00× | — |
| 2 | 7,610,030 | 9,000,005 | 1.18× | join reorder |
| 3 | 27,150,000 | 39,150,000 | 1.44× | secure cut（L→O 后排序） |
| 4 | 12,000,000 | 19,500,000 | 1.63× | semi-join |
| 5 | 27,010,055 | 45,000,005 | 1.67× | join reorder + cut |
| 6 | 6,000,000 | 6,000,000 | 1.00× | — |
| 7 | 31,660,000 | 48,000,000 | 1.52× | join reorder |
| 8 | 30,360,030 | 55,700,005 | 1.83× | join reorder + cut |
| 9 | 31,810,000 | 48,200,000 | 1.52× | join reorder |
| 10 | 27,150,000 | 27,000,000 | 0.99× | ≈无差异（见备注） |
| 11 | 3,210,025 | 4,000,025 | 1.25× | join 顺序（N⋈S 先行） |
| 12 | 18,000,000 | 18,000,000 | 1.00× | — |
| 13 | 3,750,000 | 9,000,000 | 2.40× | secure cut（O→C） |
| 14 | 18,000,000 | 18,000,000 | 1.00× | — |
| 15 | 42,000,000 | 42,000,000 | 1.00× | — |
| 16 | 3,410,000 | 3,410,000 | 1.00× | — |
| 17 | 24,400,000 | 24,200,000 | 0.99× | ≈无差异（见备注） |
| 18 | 16,500,000 | 31,500,000 | 1.91× | secure cut（L→O） |
| 19 | 18,200,000 | 18,200,000 | 1.00× | — |
| 20 | 15,430,025 | 19,030,025 | 1.23× | secure cut（L→PS） |
| 21 | 64,530,025 | 67,510,025 | 1.05× | secure cut（3 处） |
| 22 | 600,000 | 600,000 | 1.00× | — |
| **总计** | **440,780,196** | **565,000,090** | **1.28×** | 优化总体减少约 22% 参与行数 |

## 备注

- 两侧均为精确静态值。唯一数据依赖点是各查询验证段的 `head(open_valid)`，已排除。
- 该口径（输出基数之和）会**低估** join reorder 与 secure cut 的真实收益：join 内部 FromLtoR 排序规模是 2m+n，左表缩小能省的工作量在本口径下不可见；secure cut 若只截断 join 左输入，甚至不改变后续阶段输出（Q17 因此出现 0.99×——Qamboo 多了 head(P) 这个阶段输出 P，而下游 join 输出只取决于右表 L）。Q10 的 0.99× 同理：Qamboo 先 N⋈C 产生一个 C 阶段，未优化版把 nation 推迟到最后、输出本来就是 L。
- Q4 未优化变体（`q4_no_semi.rs`）未实现末尾的 order_by，按变体原样计入；若补上 sort(L)，未优化侧再加 6M（25.5M，比值 2.13×）。
- Q11 无官方 no_join_reorder 变体，未优化列按 FROM 序（partsupp, supplier, nation）左深推导。
- Q15 的 max 子查询用两次全表排序实现、Q21 的三棵 L 规模子树等，两侧计划相同，差异为零。
