# Filter Shared API 使用说明

## 概述

新的 `filter_shared` trait 定义支持可变数量的网络连接,从而支持灵活的并行度配置。

## Trait 定义

```rust
fn filter_shared<N: Network>(
    &mut self,
    lhs_column: &ShareColumn<T>,
    predicate: Predicate,
    rhs_column: &ShareColumn<T>,
    valid_column: &mut ShareColumn<T>,
    nets: &[&N],                    // 网络连接切片
    states: &mut [&mut Rep3State],  // 状态切片
) -> eyre::Result<()>;
```

## 支持的并行度

- **单线程模式** (nets.len() == 1): 顺序执行,适合小数据集或调试
- **双线程模式** (nets.len() == 2): 使用 `net::join` 并行执行,适合大数据集
- **多线程模式** (nets.len() > 2): 未来可扩展支持

## 使用示例

### 单线程模式

```rust
// 创建单个网络和状态
let [net] = TcpNetwork::networks::<1>(config)?;
let mut state = Rep3State::new(&net, A2BType::default())?;

// 调用 filter_shared
table.filter_shared(
    &lhs_column,
    Predicate::GreaterThan,
    &rhs_column,
    &mut valid_column,
    &[&net],           // 单个网络
    &mut [&mut state], // 单个状态
)?;
```

### 双线程模式

```rust
// 创建两个网络连接
let [net0] = TcpNetwork::networks::<1>(config0)?;
let [net1] = TcpNetwork::networks::<1>(config1)?;

// 创建两个状态
let mut state0 = Rep3State::new(&net0, A2BType::default())?;
let mut state1 = state0.fork(0)?;

// 调用 filter_shared,数据会自动分成两部分并行处理
table.filter_shared(
    &lhs_column,
    Predicate::GreaterThan,
    &rhs_column,
    &mut valid_column,
    &[&net0, &net1],              // 两个网络
    &mut [&mut state0, &mut state1], // 两个状态
)?;
```

## 性能建议

1. **小数据集** (< 100K 行): 使用单线程模式,避免线程切换开销
2. **中等数据集** (100K - 1M 行): 使用双线程模式,获得最佳性能
3. **大数据集** (> 1M 行): 考虑扩展到更多线程(需要实现)

## 注意事项

- `nets` 和 `states` 的长度必须相同,否则会返回错误
- 数据会被平均分配到各个线程,对于不能整除的情况会自动处理
- 各个线程的网络连接必须独立,避免资源竞争
- 状态对象应该通过 `fork()` 方法创建,以保持正确的 MPC 协议状态

## 未来扩展

可以通过实现以下功能来支持更多线程:

```rust
// 使用 crossbeam 或 rayon 进行 N 路并行
if num_threads > 2 {
    crossbeam::scope(|s| {
        // 启动 N 个线程并行处理
    })?;
}
```
