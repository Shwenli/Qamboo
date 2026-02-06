# Filter 使用示例

## 基本用法 - 使用谓词和 valid_column

```rust
use table::{ShareTable, ShareColumn};
use table::table_operator::Filter;
use table::predicate::Predicate;
use protocols::protocols::rep3_ring::Rep3RingShare;
use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::protocols::rep3_ring::ring::bit::Bit;

// 假设你有一个 ShareTable，包含一个 valid 列用于标记行的有效性
let mut table: ShareTable<Rep3RingShare<u32>> = /* ... */;
// valid 列应该是 Rep3RingShare<Bit> 类型，初始值全为 1（表示所有行都有效）

// 示例 1: 与公开值比较 - 等于过滤 (age == 18)
table.filter_public(
    &table["age"],
    Predicate::eq(),
    &RingElement::from(18u32),
    "valid",  // valid_column 的名称
    &net,
    &mut state,
)?;

// 示例 2: 与公开值比较 - 大于过滤 (age > 18)
table.filter_public(
    &table["age"],
    Predicate::gt(),
    &RingElement::from(18u32),
    "valid",
    &net,
    &mut state,
)?;

// 示例 3: 两列共享值比较 (salary > bonus)
table.filter_shared(
    &table["salary"],
    Predicate::gt(),
    &table["bonus"],
    "valid",
    &net,
    &mut state,
)?;
```

## 支持的谓词

| 谓词 | 函数 | 说明 |
|------|------|------|
| `Predicate::Equal` | `Predicate::eq()` | 等于 (==) |
| `Predicate::NotEqual` | `Predicate::ne()` | 不等于 (!=) |
| `Predicate::GreaterThan` | `Predicate::gt()` | 大于 (>) |
| `Predicate::GreaterOrEqual` | `Predicate::ge()` | 大于等于 (>=) |
| `Predicate::LessThan` | `Predicate::lt()` | 小于 (<) |
| `Predicate::LessOrEqual` | `Predicate::le()` | 小于等于 (<=) |

## 链式过滤

```rust
// 过滤 age > 18 且 age <= 65 的行
let mut table: ShareTable<Rep3RingShare<u32>> = /* ... */;

// 第一次过滤: age > 18
table.filter_public(
    &table["age"],
    Predicate::gt(),
    &RingElement::from(18u32),
    "valid",
    &net,
    &mut state,
)?;

// 第二次过滤: age <= 65
table.filter_public(
    &table["age"],
    Predicate::le(),
    &RingElement::from(65u32),
    "valid",
    &net,
    &mut state,
)?;

// 现在 valid 列标记了满足 18 < age <= 65 的行
```

## 两种过滤方式

### 1. `filter_public` - 与公开值比较
- 比较共享列与公开值
- 适用于已知的过滤条件（如 `age > 18`）
- 使用 `apply_public` 方法

### 2. `filter_shared` - 两列共享值比较
- 比较两列共享值
- 适用于列间比较（如 `salary > bonus`）
- 使用 `apply_shared` 方法
- **不需要打开任何值，保持隐私性**

## 工作原理

1. **谓词选择**：根据传入的 `Predicate` 枚举，选择相应的比较函数
2. **批量比较**：使用 `primitives::batched_compare` 中的批量比较函数进行比较
3. **生成掩码**：生成一个布尔掩码（`Rep3RingShare<Bit>`），标记哪些行满足谓词条件
4. **更新 valid 列**：使用 AND 操作将掩码应用到 valid_column 上
   - `valid_column = valid_column AND mask_bits`
   - 保持满足条件的行为有效（1），不满足的设为无效（0）
5. **保留所有行**：不实际删除行，只是标记有效性

## 注意事项

- 过滤操作**不删除行**，而是通过 `valid_column` 标记行的有效性
- `valid_column` 必须是 `Rep3RingShare<Bit>` 类型
- 链式过滤会累积条件：每次过滤都会进一步缩小有效行的范围
- `filter_shared` 不需要打开任何值，保持完全的隐私性
- 所有谓词都使用 `batched_compare` 模块的优化实现，支持批量处理
