mod rep3_ring_table_operator{

    use itertools::izip;
    use protocols::protocols::rep3_ring::Rep3State;
    use protocols::protocols::rep3_ring;
    use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
    use protocols::MpcState;
    use operator::sort;
    use primitives::div;
    use rand::thread_rng;
    use rand::Rng;
    use std::sync::mpsc;
    use net::tcp::{TcpNetwork, NetworkConfig, NetworkParty};
    use std::net::{ToSocketAddrs};
    use table::share_table::ShareTable;
    use table::share_column::ShareColumn;
    use table::table_operator::OrderBySingle;
    use table::share_column::ShareType;

    macro_rules! apply_to_all {
        ($expr:ident,[$($t:ty),*]) => {
            $(
                $expr::<$t>();
            )*
        };
    }
    

    //cargo test --release --package tests --test table -- table_operator_ring::rep3_ring_table_operator::tcp_fork_radix_sort_by --exact --nocapture 
    #[test]
    fn test_radix_sort_by_key() {
        const VEC_SIZE: usize = 30000;
        const CHUNK_SIZE: usize = 64;

        let mut rng = thread_rng();
        let keys: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();
        let values1: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();
        let values2: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();

        let keys_ring: Vec<RingElement<u64>> = keys.iter().map(|v| RingElement(*v)).collect();
        let values1_ring: Vec<RingElement<u64>> = values1.iter().map(|v| RingElement(*v)).collect();
        let values2_ring: Vec<RingElement<u64>> = values2.iter().map(|v| RingElement(*v)).collect();

        let key_shares = rep3_ring::share_ring_elements(&keys_ring, &mut rng);
        let value1_shares = rep3_ring::share_ring_elements(&values1_ring, &mut rng);
        let value2_shares = rep3_ring::share_ring_elements(&values2_ring, &mut rng);

        let order = true; // Ascending sort

        let mut combined_data: Vec<_> = izip!(&keys, &values1, &values2).collect();
        let mask = if CHUNK_SIZE == 64 { u64::MAX } else { (1u64 << CHUNK_SIZE) - 1 };
        combined_data.sort_by_key(|(k, _, _)| *k & mask);

        let should_result1: Vec<RingElement<u64>> = combined_data.iter().map(|(_, v1, _)| RingElement(**v1)).collect();
        let should_result2: Vec<RingElement<u64>> = combined_data.iter().map(|(_, _, v2)| RingElement(**v2)).collect();

        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8100".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8101".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8102".parse().unwrap()),
        ];
        
        
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, (key_share, value1_share, value2_share)) in izip!(0..3, [tx1, tx2, tx3], izip!(key_shares, value1_shares, value2_shares)) {
            let parties_main = parties_main.clone();
            let order = order.clone();
            
            let handle = std::thread::spawn(move || {
                let config0 = NetworkConfig::new(
                    party_id,
                    parties_main[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_main,
                    None,
                    None,
                );
                let [net] = TcpNetwork::networks::<1>(config0).unwrap();
                
                let mut state = Rep3State::new(&net).unwrap();

                let total_start = std::time::Instant::now();
                let result = sort::radix_sort_by_key(
                    &key_share,
                    order,
                    vec![&value1_share, &value2_share],
                    CHUNK_SIZE,
                    &net,
                    &mut state,
                ).unwrap();
                let total_dur = total_start.elapsed();
                eprintln!("Party {} total elapsed: {:?}", party_id, total_dur);
                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut results1 = vec![];
        let mut results2 = vec![];
        let mut results3 = vec![];

        let res1 = rx1.recv().unwrap();
        results1.push(res1[0].clone());
        results1.push(res1[1].clone());

        let res2 = rx2.recv().unwrap();
        results2.push(res2[0].clone());
        results2.push(res2[1].clone());

        let res3 = rx3.recv().unwrap();
        results3.push(res3[0].clone());
        results3.push(res3[1].clone());

        let is_result1 = rep3_ring::combine_ring_elements(&results1[0], &results2[0], &results3[0]);
        let is_result2 = rep3_ring::combine_ring_elements(&results1[1], &results2[1], &results3[1]);

        assert_eq!(is_result1, should_result1);
        assert_eq!(is_result2, should_result2);
    }

    #[test]
    fn test_order_by_single() {
        

        const VEC_SIZE: usize = 1000;
        const CHUNK_SIZE: usize = 32;

        let mut rng = thread_rng();
        
        // 生成测试数据：keys 作为排序键，values1 和 values2 是其他列
        let keys: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();
        let values1: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();
        let values2: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();

        let keys_ring: Vec<RingElement<u64>> = keys.iter().map(|v| RingElement(*v)).collect();
        let values1_ring: Vec<RingElement<u64>> = values1.iter().map(|v| RingElement(*v)).collect();
        let values2_ring: Vec<RingElement<u64>> = values2.iter().map(|v| RingElement(*v)).collect();

        // 分享数据给三方
        let key_shares = rep3_ring::share_ring_elements(&keys_ring, &mut rng);
        let value1_shares = rep3_ring::share_ring_elements(&values1_ring, &mut rng);
        let value2_shares = rep3_ring::share_ring_elements(&values2_ring, &mut rng);

        let order = true; // 升序排序

        // 计算期望结果
        let mut combined_data: Vec<_> = izip!(&keys, &values1, &values2).collect();
        let mask = if CHUNK_SIZE == 64 { u64::MAX } else { (1u64 << CHUNK_SIZE) - 1 };
        combined_data.sort_by_key(|(k, _, _)| *k & mask);

        let should_keys: Vec<RingElement<u64>> = combined_data.iter().map(|(k, _, _)| RingElement(**k)).collect();
        let should_values1: Vec<RingElement<u64>> = combined_data.iter().map(|(_, v1, _)| RingElement(**v1)).collect();
        let should_values2: Vec<RingElement<u64>> = combined_data.iter().map(|(_, _, v2)| RingElement(**v2)).collect();

        let parties = vec![
            NetworkParty::new(0, "127.0.0.1:8200".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8201".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8202".parse().unwrap()),
        ];
        
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, (key_share, value1_share, value2_share)) in izip!(0..3, [tx1, tx2, tx3], izip!(key_shares, value1_shares, value2_shares)) {
            let parties = parties.clone();
            let order = order.clone();
            
            let handle = std::thread::spawn(move || {
                let config0 = NetworkConfig::new(
                    party_id,
                    parties[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties,
                    None,
                    None,
                );
                let [net] = TcpNetwork::networks::<1>(config0).unwrap();
        
                let mut state = Rep3State::new(&net).unwrap();

                // 创建 ShareTable
                let key_column = ShareColumn::new(key_share, DataType::Arithmetic, "keys".to_string());
                let value1_column = ShareColumn::new(value1_share, DataType::Arithmetic, "values1".to_string());
                let value2_column = ShareColumn::new(value2_share, DataType::Arithmetic, "values2".to_string());
                
                let mut table = ShareTable::new();
                table.insert_column("keys".to_string(), key_column.clone());
                table.insert_column("values1".to_string(), value1_column);
                table.insert_column("values2".to_string(), value2_column);
                let total_start = std::time::Instant::now();
                // 执行 order_by 操作
                table.order_by_single(
                    "keys",
                    order,
                    CHUNK_SIZE,
                    &net,
                    &mut state,
                ).unwrap();
                let total_dur = total_start.elapsed();
                eprintln!("Party {} total elapsed: {:?}", party_id, total_dur);


                // 返回排序后的结果
                let sorted_keys = table["keys"].get_data().to_vec();
                let sorted_values1 = table["values1"].get_data().to_vec();
                let sorted_values2 = table["values2"].get_data().to_vec();

                tx.send((sorted_keys, sorted_values1, sorted_values2)).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 收集三方的结果
        let (keys1, values1_1, values2_1) = rx1.recv().unwrap();
        let (keys2, values1_2, values2_2) = rx2.recv().unwrap();
        let (keys3, values1_3, values2_3) = rx3.recv().unwrap();

        // 组合结果
        let result_keys = rep3_ring::combine_ring_elements(&keys1, &keys2, &keys3);
        let result_values1 = rep3_ring::combine_ring_elements(&values1_1, &values1_2, &values1_3);
        let result_values2 = rep3_ring::combine_ring_elements(&values2_1, &values2_2, &values2_3);

        // 验证结果
        assert_eq!(result_keys, should_keys);
        assert_eq!(result_values1, should_values1);
        assert_eq!(result_values2, should_values2);

        println!("Order by test passed!");
    }

    

    #[test]
    fn tcp_from_l_to_r_first() {
        use operator::from_l_to_r;
        //let vec_size: usize = 1000000;

        // 测试用例：来自论文中 的经典例子
        // L: keys=[3, 5, 9], A=[42, 8, 23]
        // R: keys=[3, 7, 9, 9]
        // Expected result: [42, 0, 23, 23];
        let mut rng = thread_rng();
        
        // L 表数据
        let keys_l: Vec<u64> = vec![3, 5, 9];
        let values_a: Vec<u64> = vec![42, 8, 23];
        
        // R 表数据
        let keys_r: Vec<u64> = vec![7, 3, 9, 9];

        //let keys_l: Vec<u64> = (0..vec_size).map(|_| rng.gen_range(0..100)).collect();  
        //let values_a: Vec<u64> = (0..vec_size).map(|_| rng.gen_range(0..100)).collect();
        //let keys_r: Vec<u64> = (0..vec_size).map(|_| rng.gen_range(0..100)).collect();
        //let values: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen_range(0..100)).collect();
        
        // 转换为 RingElement
        let keys_l_ring: Vec<RingElement<u64>> = keys_l.iter().map(|v| RingElement(*v)).collect();
        let values_a_ring: Vec<RingElement<u64>> = values_a.iter().map(|v| RingElement(*v)).collect();
        let keys_r_ring: Vec<RingElement<u64>> = keys_r.iter().map(|v| RingElement(*v)).collect();

        // 分享数据给三方
        let keys_l_shares = rep3_ring::share_ring_elements(&keys_l_ring, &mut rng);
        let values_a_shares = rep3_ring::share_ring_elements(&values_a_ring, &mut rng);
        let keys_r_shares = rep3_ring::share_ring_elements(&keys_r_ring, &mut rng);

        // 期望结果：[42, 0, 23, 23]
        let should_result: Vec<RingElement<u64>> = vec![
            RingElement(42),
            RingElement(0),
            RingElement(23),
            RingElement(23),
        ];

        let bitsize = 64; // 使用完整的 64 位进行排序

        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8800".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8801".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8802".parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, "127.0.0.1:9800".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9801".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9802".parse().unwrap()),
        ];
        
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, (keys_l_share, values_a_share, keys_r_share)) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            izip!(keys_l_shares, values_a_shares, keys_r_shares)
        ) {
            let parties_main = parties_main.clone();
            let parties_fork = parties_fork.clone();
            
            let handle = std::thread::spawn(move || {
                
                
                // 创建主网络
                let config_main = NetworkConfig::new(
                    party_id,
                    parties_main[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_main,
                    None,
                    None,
                );
                let [net_main] = TcpNetwork::networks::<1>(config_main).unwrap();

                // 创建分叉网络
                let config_fork = NetworkConfig::new(
                    party_id,
                    parties_fork[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_fork,
                    None,
                    None,
                );
                let [net_fork] = TcpNetwork::networks::<1>(config_fork).unwrap();

                // 创建 MPC 状态
                let mut state_main = Rep3State::new(&net_main).unwrap();
                let mut state_fork = Rep3State::new(&net_fork).unwrap();

                // 执行 from_l_to_r_first
                let start_time = std::time::Instant::now();
                let (result, _perm) = from_l_to_r::from_l_to_r_first(
                    &keys_l_share,
                    &keys_r_share,
                    &values_a_share,
                    bitsize,
                    &net_main,
                    &net_fork,
                    &mut state_main,
                    &mut state_fork,
                ).unwrap();

                let elapsed = start_time.elapsed();
                println!("Party {} total elapsed: {:?}", party_id, elapsed);

                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 收集三方的结果
        let result1 = rx1.recv().unwrap();
        let result2 = rx2.recv().unwrap();
        let result3 = rx3.recv().unwrap();

        // 组合结果
        let is_result = rep3_ring::combine_ring_elements(&result1, &result2, &result3);

        // 验证结果
        assert_eq!(is_result, should_result);
         
        println!("FromLtoR test passed!");
        println!("L keys: {:?}", keys_l);
        println!("L values: {:?}", values_a);
        println!("R keys: {:?}", keys_r);
        println!("Result: {:?}", is_result.iter().map(|r| r.0).collect::<Vec<_>>());
        
    }

    //cargo test --release --package tests --test table -- table_operator_ring::rep3_ring_table_operator::tcp_radix_sort_ring_multithreads_8_net --exact --nocapture
    #[test]
    fn tcp_radix_sort_ring_multithreads_8_net() {
        const VEC_SIZE: usize = 3000;
        const CHUNK_SIZE: usize = 64;

        let mut rng = thread_rng();
        let values: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();
        let values_ring: Vec<RingElement<u64>> = values.iter().map(|v| RingElement(*v)).collect();
        let value_shares = rep3_ring::share_ring_elements(&values_ring, &mut rng);

        let order = false;

        let mut expected_values = values.clone();
        //降序排列
        expected_values.sort_by(|a, b| b.cmp(a));
        //expected_values.sort();
        let expected_ring: Vec<RingElement<u64>> = expected_values.iter().map(|v| RingElement(*v)).collect();

        // Generate parties for 6 networks (2 for decomp, 4 for parallel)
        let mut parties_collection = Vec::new();
        let mut port_counter = 8800;
        for _ in 0..10 {
            let mut parties = Vec::new();
            for party_id in 0..3 {
                parties.push(NetworkParty::new(
                    party_id,
                    format!("127.0.0.1:{}", port_counter).parse().unwrap(),
                ));
                port_counter += 1;
            }
            port_counter += 1000;
            parties_collection.push(parties);
        }

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, share) in izip!(0..3, [tx1, tx2, tx3], value_shares) {
            let parties_collection = parties_collection.clone();
            let handle = std::thread::spawn(move || {
                let mut nets = Vec::new();
                
                // Create all networks
                for parties in parties_collection {
                    let config = NetworkConfig::new(
                        party_id,
                        parties[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                        parties,
                        None,
                        None,
                    );
                    let [net] = TcpNetwork::networks::<1>(config).unwrap();
                    nets.push(net);
                }

                // Split networks: 0,1 for decomp; 2..10 for parallel
                let mut states: Vec<Rep3State> = nets.iter()
                    .map(|net| Rep3State::new(net).unwrap())
                    .collect();
                
                let (decomp_states, parallel_states) = states.split_at_mut(2);
                let (state_decomp0_slice, state_decomp1_slice) = decomp_states.split_at_mut(1);
                let state_decomp0 = &mut state_decomp0_slice[0];
                let state_decomp1 = &mut state_decomp1_slice[0];
                
                let mut parallel_state_refs: Vec<&mut Rep3State> = parallel_states.iter_mut().collect();
                let parallel_nets: Vec<&TcpNetwork> = nets[2..].iter().collect();

                let start_time = std::time::Instant::now();
                let result = sort::radix_sort_ring_multithreads(
                    share,
                    order,
                    CHUNK_SIZE,
                    &parallel_nets,
                    state_decomp0,
                    state_decomp1,
                    &mut parallel_state_refs,
                ).unwrap();
                let duration = start_time.elapsed();
                println!("Radix sort multithreads (8 nets) took: {:?}", duration);
                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let res1 = rx1.recv().unwrap();
        let res2 = rx2.recv().unwrap();
        let res3 = rx3.recv().unwrap();

        let result = rep3_ring::combine_ring_elements(&res1, &res2, &res3);
        assert_eq!(result, expected_ring);
        println!("Radix sort multithreads (8 nets) test passed!");
    }

    
    #[test]
    fn test_non_restoring_division() {
        
        let mut rng = thread_rng();
        let numerator: u64 = rng.gen_range(0..1000);
        let denominator: u64 = rng.gen_range(1..100); // Avoid division by zero
        let bits = 32; // u64, but we need 2*bits space in the ring, so 32 is safe for u64 ring if inputs are small enough

        let num_ring = RingElement(numerator);
        let den_ring = RingElement(denominator);

        let num_shares = rep3_ring::share_ring_elements_binary(&[num_ring], &mut rng);
        let den_shares = rep3_ring::share_ring_elements_binary(&[den_ring], &mut rng);

        let parties = vec![
            NetworkParty::new(0, "127.0.0.1:11000".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:11001".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:11002".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, (num_share, den_share)) in izip!(0..3, [tx1, tx2, tx3], izip!(num_shares, den_shares)) {
            let parties = parties.clone();
            let handle = std::thread::spawn(move || {
                let config = NetworkConfig::new(
                    party_id,
                    parties[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties,
                    None,
                    None,
                );
                let [net] = TcpNetwork::networks::<1>(config).unwrap();
                let mut state = Rep3State::new(&net).unwrap();

                let result = div::non_restoring_division(
                    &num_share[0],
                    &den_share[0],
                    bits,
                    &net,
                    &mut state,
                ).unwrap();

                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let res1 = rx1.recv().unwrap();
        let res2 = rx2.recv().unwrap();
        let res3 = rx3.recv().unwrap();

        let result = rep3_ring::combine_ring_elements_binary(&[res1], &[res2], &[res3]);
        
        println!("Numerator: {}, Denominator: {}, Result: {}, Expected: {}", numerator, denominator, result[0].0, numerator / denominator);
        assert_eq!(result[0].0, numerator / denominator);
    }

    #[test]
    fn test_non_restoring_division_public() {
        
        let mut rng = thread_rng();
        let numerator: u64 = rng.gen_range(0..1000);
        let denominator: u64 = rng.gen_range(1..100); // Avoid division by zero
        

        let num_ring = RingElement(numerator);
        let den_ring = RingElement(denominator);

        let num_shares = rep3_ring::share_ring_elements(&[num_ring], &mut rng);

        let parties = vec![
            NetworkParty::new(0, "127.0.0.1:11000".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:11001".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:11002".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, num_share) in izip!(0..3, [tx1, tx2, tx3], num_shares) {
            let parties = parties.clone();
            let handle = std::thread::spawn(move || {
                let config = NetworkConfig::new(
                    party_id,
                    parties[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties,
                    None,
                    None,
                );
                let [net] = TcpNetwork::networks::<1>(config).unwrap();
                let mut state = Rep3State::new(&net).unwrap();

                let result = div::div_rem_const_public_arithmetic_i64(
                    &num_share[0],
                    den_ring,
                    &net,
                    &mut state,
                ).unwrap();

                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let res1 = rx1.recv().unwrap();
        let res2 = rx2.recv().unwrap();
        let res3 = rx3.recv().unwrap();

        let result = rep3_ring::combine_ring_element::<u64>(res1, res2, res3);
        
        println!("Numerator: {}, Denominator: {}, Result: {}, Expected: {}", numerator, denominator, result, numerator / denominator);
        assert_eq!(result.0, numerator / denominator);
    }
    
    //cargo test --release --package tests --test table -- table_operator_ring::rep3_ring_table_operator::test_non_restoring_division_many --exact --nocapture
    #[test]
    fn test_non_restoring_division_many() {

        let mut rng = thread_rng();
        let count = 10;
        let numerators: Vec<u64> = (0..count).map(|_| rng.gen_range(0..1000)).collect();
        let denominators: Vec<u64> = (0..count).map(|_| rng.gen_range(1..100)).collect();
        let bits = 32;

        let num_ring: Vec<RingElement<u64>> = numerators.iter().map(|&x| RingElement(x)).collect();
        let den_ring: Vec<RingElement<u64>> = denominators.iter().map(|&x| RingElement(x)).collect();

        let num_shares = rep3_ring::share_ring_elements_binary(&num_ring, &mut rng);
        let den_shares = rep3_ring::share_ring_elements_binary(&den_ring, &mut rng);

        let parties = vec![
            NetworkParty::new(0, "127.0.0.1:11010".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:11011".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:11012".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, (num_share, den_share)) in izip!(0..3, [tx1, tx2, tx3], izip!(num_shares, den_shares)) {
            let parties = parties.clone();
            let handle = std::thread::spawn(move || {
                let config = NetworkConfig::new(
                    party_id,
                    parties[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties,
                    None,
                    None,
                );
                let [net] = TcpNetwork::networks::<1>(config).unwrap();
                let mut state = Rep3State::new(&net).unwrap();

                let tot_time = std::time::Instant::now();
                let result = div::non_restoring_division_many(
                    &num_share,
                    &den_share,
                    bits,
                    &net,
                    &mut state,
                ).unwrap();
                println!("Time taken: {:?}", tot_time.elapsed());

                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let res1 = rx1.recv().unwrap();
        let res2 = rx2.recv().unwrap();
        let res3 = rx3.recv().unwrap();

        let result = rep3_ring::combine_ring_elements_binary(&res1, &res2, &res3);
        
        for (i, res) in result.iter().enumerate() {
             //println!("Index {}: {} / {} = {}, Expected: {}", i, numerators[i], denominators[i], res.0, numerators[i] / denominators[i]);
             assert_eq!(res.0, numerators[i] / denominators[i], "Mismatch at index {}", i);
        }
    }

    //cargo test --release --package tests --test table -- table_operator_ring::rep3_ring_table_operator::test_non_restoring_division_public_many --exact --nocapture
    #[test]
    fn test_division_public_many() {

        let mut rng = thread_rng();
        let count = 10000;
        let numerators: Vec<u64> = (0..count).map(|_| rng.gen_range(0..1000)).collect();
        //let denominators: Vec<u64> = (0..count).map(|_| rng.gen_range(1..100)).collect();
        let denominator = rng.gen_range(1..100) as u64;

        let num_ring: Vec<RingElement<u64>> = numerators.iter().map(|&x| RingElement(x)).collect();
        let den_ring = RingElement(denominator);
        //let den_ring: Vec<RingElement<u64>> = denominators.iter().map(|&x| RingElement(x)).collect();

        let num_shares = rep3_ring::share_ring_elements(&num_ring, &mut rng);
    

        let parties = vec![
            NetworkParty::new(0, "127.0.0.1:11010".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:11011".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:11012".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, num_share) in izip!(0..3, [tx1, tx2, tx3], num_shares) {
            let parties = parties.clone();
            let den_ring = den_ring.clone();
            let handle = std::thread::spawn(move || {
                let config = NetworkConfig::new(
                    party_id,
                    parties[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties,
                    None,
                    None,
                );
                let [net] = TcpNetwork::networks::<1>(config).unwrap();
                let mut state = Rep3State::new(&net).unwrap();

                let tot_time = std::time::Instant::now();
                let result = div::div_rem_const_public_arithmetic_many_i64(
                    &num_share,
                    den_ring,
                    &net,
                    &mut state,
                ).unwrap();
                println!("Time taken: {:?}", tot_time.elapsed());

                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let res1 = rx1.recv().unwrap();
        let res2 = rx2.recv().unwrap();
        let res3 = rx3.recv().unwrap();

        let result = rep3_ring::combine_ring_elements(&res1, &res2, &res3);
         
        for (i, res) in result.iter().enumerate() {
             //println!("Index {}: {} / {} = {}, Expected: {}", i, numerators[i], denominators[i], res.0, numerators[i] / denominators[i]);
             assert_eq!(res.0, numerators[i] / denominator, "Mismatch at index {}", i);
        }
        
        
    }

    //cargo test --release --package tests --test table -- table_operator_ring::rep3_ring_table_operator::tcp_div_share_by_public_arithmetic_multithreads_8_net --exact --nocapture
    #[test]
    fn tcp_div_share_by_public_arithmetic_multithreads_8_net() {
        const VEC_SIZE: usize = 1000;

        let mut rng = thread_rng();
        let numerators: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen_range(0..1000)).collect();
        let denominator: u64 = rng.gen_range(1..100);

        let num_ring: Vec<RingElement<u64>> = numerators.iter().copied().map(RingElement).collect();
        let num_shares = rep3_ring::share_ring_elements(&num_ring, &mut rng);
        let den_ring = RingElement(denominator);

        let expected: Vec<RingElement<u64>> = numerators
            .iter()
            .map(|&x| RingElement(x / denominator))
            .collect();

        // Follow tcp_radix_sort_ring_multithreads_8_net_permute_test: create 10 networks,
        // and use nets[2..] (8 nets) as the parallel networks.
        let mut parties_collection = Vec::new();
        let mut port_counter = 12000;
        for _ in 0..10 {
            let mut parties = Vec::new();
            for party_id in 0..3 {
                parties.push(NetworkParty::new(
                    party_id,
                    format!("127.0.0.1:{}", port_counter).parse().unwrap(),
                ));
                port_counter += 1;
            }
            port_counter += 1000;
            parties_collection.push(parties);
        }

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, num_share) in izip!(0..3, [tx1, tx2, tx3], num_shares) {
            let parties_collection = parties_collection.clone();
            let handle = std::thread::spawn(move || {
                let mut nets = Vec::new();

                // Create all networks
                for parties in parties_collection {
                    let config = NetworkConfig::new(
                        party_id,
                        parties[party_id]
                            .dns_name
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                        parties,
                        None,
                        None,
                    );
                    let [net] = TcpNetwork::networks::<1>(config).unwrap();
                    nets.push(net);
                }

                let mut states: Vec<Rep3State> = nets
                    .iter()
                    .map(|net| Rep3State::new(net).unwrap())
                    .collect();

                let parallel_nets: Vec<&TcpNetwork> = nets[2..].iter().collect();
                let mut parallel_state_refs: Vec<&mut Rep3State> = states[2..].iter_mut().collect();

                let result = div::div_share_by_public_arithmetic_multithreads(
                    &num_share,
                    &den_ring,
                    &parallel_nets,
                    &mut parallel_state_refs,
                )
                .unwrap();

                tx.send(result).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let res1 = rx1.recv().unwrap();
        let res2 = rx2.recv().unwrap();
        let res3 = rx3.recv().unwrap();

        let result = rep3_ring::combine_ring_elements(&res1, &res2, &res3);
        assert_eq!(result, expected);
    }

}