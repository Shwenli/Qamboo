# Operator Module

## Overview

The `operator` module implements privacy-preserving relational database operators for secure multi-party computation. These operators form the computational foundation for SQL query execution on secretly shared data, enabling complex analytics without revealing underlying values.

## Supported Operators

### 1. Join (`join.rs`)

Secure equi-join between two relations on shared key columns.

**Algorithm:** Sort-Merge Join with Permutation Networks

```rust
pub fn join_on_multithreads<T, N>(
    keys_m: &[Rep3RingShare<T>],      // Keys from first relation
    keys_n: &[Rep3RingShare<T>],      // Keys from second relation
    vals_m: &[Rep3RingShare<T>],      // Values from first relation
    vals_n: &[Rep3RingShare<T>],      // Values from second relation
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<T>>)>
```

**Steps:**

1. Generate sorting permutation for combined keys
2. Apply permutation to both relations
3. Compute join indicator using prefix sums
4. Filter and output joined tuples

**Complexity:** O((m+n) log(m+n)) communication

### 2. GroupBy (`group_by.rs`)

Secure grouping and aggregation with multiple group keys.

```rust
pub fn table_group_by_common_multithreads<T, N>(
    keys: Vec<&[Rep3RingShare<T>]>,   // Group key columns
    vals: Vec<&[Rep3RingShare<T>]>,   // Value columns
    valid: &[Rep3RingShare<T>],        // Valid row indicator
    order: bool,                       // ASC (true) or DESC (false)
    sort_bitsize: usize,               // Bit size for sorting
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<GroupByOutput<T>>
```

**Output:**

- Grouped keys
- Aggregation values
- Permutation for aggregation
- Equality bits for group boundaries

**Algorithm:**

1. Sort by composite group key
2. Compute group boundary indicators
3. Apply prefix sums for aggregation

### 3. Aggregation Functions (`agg_func.rs`)

Implements SQL aggregation operations on grouped data.

```rust
// COUNT(*) - Count rows per group
pub fn table_agg_count_multithreads<T, N>(...)

// COUNT(DISTINCT) - Count distinct values
pub fn table_agg_count_by_valid_multithreads<T, N>(...)

// SUM - Sum values per group
pub fn table_agg_sum_multithreads<T, N>(...)

// AVG - Average (SUM / COUNT)
pub fn table_agg_avg_multithreads<T, N>(...)

// MIN/MAX - Using comparison primitives
pub fn table_agg_min_max_multithreads<T, N>(...)
```

### 4. Sort (`sort.rs`)

Secure sorting of shared values.

```rust
pub fn radix_sort_multithreads<T, N>(
    keys: &[Rep3RingShare<T>],
    order: bool,           // ASC or DESC
    bitsize: usize,        // Sort by most significant bits
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<Vec<Rep3RingShare<u32>>>  // Returns permutation
```

**Algorithm:** Bitwise radix sort using permutation networks

### 5. Distinct (`distinct.rs`)

Removes duplicate rows from a relation.

```rust
pub fn distinct_multithreads<T, N>(
    keys: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<(Vec<Rep3RingShare<T>>, Vec<Rep3RingShare<Bit>>)>
```

**Algorithm:** Sort + deduplication using comparison

### 6. Left-to-Right Copy (`from_l_to_r.rs`)

Copies values from left relation to right based on join condition.

```rust
pub fn from_l_to_r_other_multithreads<T, N>(
    len_m: usize,
    len_n: usize,
    perm: &[Rep3RingShare<PermRing>],
    l: &[Rep3RingShare<T>],
    nets: &[&N],
    states: &mut [&mut Rep3State],
) -> Result<Vec<Rep3RingShare<T>>>
```

Used internally by join and aggregation operators.

## Architecture

```
operator/
├── src/
│   ├── lib.rs              # Module exports
│   ├── join.rs             # Secure equi-join
│   ├── group_by.rs         # GroupBy operator
│   ├── agg_func.rs         # Aggregation functions
│   ├── sort.rs             # Secure sorting
│   ├── distinct.rs         # Distinct/Deduplication
│   └── from_l_to_r.rs      # L-to-R value copying
```

## SQL Operator Mapping

| SQL Operator | Module Function | Complexity |
| -------------- | ----------------- | ------------ |
| `JOIN` | `join_on_multithreads` | O(n log n) |
| `GROUP BY` | `table_group_by_common_multithreads` | O(n log n) |
| `COUNT(*)` | `table_agg_count_multithreads` | O(n) |
| `SUM()` | `table_agg_sum_multithreads` | O(n) |
| `AVG()` | `table_agg_avg_multithreads` | O(n) |
| `ORDER BY` | `radix_sort_multithreads` | O(n log n) |
| `DISTINCT` | `distinct_multithreads` | O(n log n) |

## Multi-Key GroupBy

For queries with multiple group keys:

```sql
SELECT region, year, SUM(sales)
FROM transactions
GROUP BY region, year
```

```rust
// Create composite key from multiple columns
let keys = vec![
    region_column.as_slice(),
    year_column.as_slice(),
];

// Perform multi-key groupby
let (grouped_keys, _, perm, eq_bits, ...) = 
    table_group_by_common_multithreads(
        keys,
        vec![sales_column.as_slice()],
        &valid_column,
        true,    // ASC order
        64,      // bit size
        nets,
        states,
    )?;

// Compute SUM aggregation
let sum_result = table_agg_sum_multithreads(
    &sales_column,
    &eq_bits,
    &perm,
    nets,
    states,
)?;
```

## Aggregation Pipeline

```rust
// Complete aggregation workflow
let (k_out, v_out, e, perm_e, eq_bits, k_sorted, perm_k, ...) = 
    table_group_by_common_multithreads(keys, vals, valid, true, 64, nets, states)?;

// Apply different aggregations
let count = table_agg_count_multithreads(&e, &perm_e, nets, states)?;
let sum = table_agg_sum_multithreads(&v_sorted, &e, &perm_e, nets, states)?;
let avg = table_agg_avg_multithreads(&v_sorted, &e, &perm_e, nets, states)?;
```

## Performance Considerations

### Batch Processing

All operators support batch processing for better communication efficiency:

```rust
// Process 1000 rows at a time
for chunk in data.chunks(1000) {
    let result = join_on_multithreads(...)?;
}
```

### Multi-Threading

Use multiple threads for CPU-intensive operations:

```rust
// Fork states for parallel processing
let mut forked = state.fork(4)?;
let states_refs: Vec<_> = forked.iter_mut().collect();

// Execute with multiple threads
let result = join_on_multithreads(..., &states_refs)?;
```

### Memory Usage

Operators use O(n) additional memory for permutations and intermediate results.

## Integration with Table Module

The `operator` module is designed to be used by the higher-level `table` module:

```rust
// In table module
trait Join<T> {
    fn join<N: Network>(
        &self,
        other: &ShareTable<T>,
        left_key: &str,
        right_key: &str,
        netstate_args: &mut NetStateArgs<N>,
    ) -> Result<ShareTable<T>> {
        // Calls operator::join::join_on_multithreads internally
    }
}
```
