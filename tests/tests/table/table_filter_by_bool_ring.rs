mod rep3_ring_table_groupby{

    use itertools::izip;
    use protocols::protocols::rep3_ring::Rep3State;
    use protocols::protocols::rep3_ring;
    use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
    use protocols::protocols::rep3_ring::conversion;
    use rand::thread_rng;
    use rand::Rng;
    use table::column_operator::ColumnBooleanOperator;
    use std::sync::mpsc;
    use net::tcp::{TcpNetwork, NetworkConfig, NetworkParty};
    use std::net::{ToSocketAddrs};
    use table::share_column::{ShareColumn,ShareType};
    use table::share_table::ShareTable;
    use table::table_operator::Filter;
    


    //*cargo test --release --package tests --test table -- table_operator_ring::rep3_ring_table_operator::tcp_filter_shared_4_threads --exact --nocapture
    #[test]
    fn tcp_filter_shared_4_threads() {
        use table::table_operator::Filter;
        use table::predicate::Predicate;

        const VEC_SIZE: usize = 1000000;

        let mut rng = thread_rng();
        
        // 生成测试数据
        let lhs_values: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen_range(0..100)).collect();
        let rhs_values: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen_range(0..100)).collect();
        let other_values: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()).collect();
        
        let lhs_ring: Vec<RingElement<u64>> = lhs_values.iter().map(|v| RingElement(*v)).collect();
        let rhs_ring: Vec<RingElement<u64>> = rhs_values.iter().map(|v| RingElement(*v)).collect();
        let other_ring: Vec<RingElement<u64>> = other_values.iter().map(|v| RingElement(*v)).collect();

        // 分享数据给三方
        let lhs_shares = rep3_ring::share_ring_elements(&lhs_ring, &mut rng);
        let rhs_shares = rep3_ring::share_ring_elements(&rhs_ring, &mut rng);
        let other_shares = rep3_ring::share_ring_elements(&other_ring, &mut rng);
        
        // 创建初始的 valid 列（全为 1，表示所有行都有效）
        let initial_valid: Vec<RingElement<u64>> = vec![RingElement(1u64); VEC_SIZE];
        let valid_shares = rep3_ring::share_ring_elements(&initial_valid, &mut rng);

        // 过滤条件：lhs <= rhs
        let predicate = Predicate::Equal;

        // 计算期望结果：valid 列应该只在 lhs <= rhs 的位置为 1
        let should_valid: Vec<RingElement<u64>> = izip!(&lhs_values, &rhs_values)
            .map(|(l, r)| if *l == *r { RingElement(1u64) } else { RingElement(0u64) })
            .collect();

        // 创建4组网络端口
        let parties_net0 = vec![
            NetworkParty::new(0, "127.0.0.1:8500".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8501".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8502".parse().unwrap()),
        ];
        
        let parties_net1 = vec![
            NetworkParty::new(0, "127.0.0.1:9500".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9501".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9502".parse().unwrap()),
        ];
        
        let parties_net2 = vec![
            NetworkParty::new(0, "127.0.0.1:8600".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8601".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8602".parse().unwrap()),
        ];
        
        let parties_net3 = vec![
            NetworkParty::new(0, "127.0.0.1:9600".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9601".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9602".parse().unwrap()),
        ];
        
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, (lhs_share, rhs_share, other_share, valid_share)) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            izip!(lhs_shares, rhs_shares, other_shares, valid_shares)
        ) {
            let parties_net0 = parties_net0.clone();
            let parties_net1 = parties_net1.clone();
            let parties_net2 = parties_net2.clone();
            let parties_net3 = parties_net3.clone();
            
            let handle = std::thread::spawn(move || {
                // 创建4个网络连接
                let config0 = NetworkConfig::new(
                    party_id,
                    parties_net0[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_net0,
                    None,
                    None,
                );
                let [net0] = TcpNetwork::networks::<1>(config0).unwrap();
                
                let config1 = NetworkConfig::new(
                    party_id,
                    parties_net1[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_net1,
                    None,
                    None,
                );
                let [net1] = TcpNetwork::networks::<1>(config1).unwrap();
                
                let config2 = NetworkConfig::new(
                    party_id,
                    parties_net2[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_net2,
                    None,
                    None,
                );
                let [net2] = TcpNetwork::networks::<1>(config2).unwrap();
                
                let config3 = NetworkConfig::new(
                    party_id,
                    parties_net3[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_net3,
                    None,
                    None,
                );
                let [net3] = TcpNetwork::networks::<1>(config3).unwrap();

                // 创建4个状态
                let mut state0 = Rep3State::new(&net0).unwrap();
                let mut state1 = Rep3State::new(&net1).unwrap();
                let mut state2 = Rep3State::new(&net2).unwrap();
                let mut state3 = Rep3State::new(&net3).unwrap();

                let 

                // 创建 ShareTable
                let lhs_column = ShareColumn::new(lhs_share, DataType::Arithmetic, "lhs".to_string());
                let rhs_column = ShareColumn::new(rhs_share, DataType::Arithmetic, "rhs".to_string());
                let other_column = ShareColumn::new(other_share, DataType::Arithmetic, "other".to_string());
                let valid_column_bool = conversion::a2b_many(&valid_share, &net0, &mut state0).unwrap();
                let valid_column = ShareColumn::new(valid_column_bool, DataType::Binary, "valid".to_string());
                
                let mut table = ShareTable::new();
                table.insert_column("lhs".to_string(), lhs_column.clone());
                table.insert_column("rhs".to_string(), rhs_column.clone());
                table.insert_column("valid".to_string(), valid_column.clone());
                table.insert_column("other".to_string(), other_column);

                // 执行 filter_shared 操作 (4线程)
                let total_start = std::time::Instant::now();
                
                table.filter_shared(
                &lhs_column.get_name(),
                &rhs_column.get_name(),
                predicate,
                &[&net0, &net1, &net2, &net3],
                &mut [&mut state0, &mut state1, &mut state2, &mut state3],
                ).unwrap();

                let total_dur = total_start.elapsed();
                eprintln!("Party {} total elapsed (4 threads): {:?}", party_id, total_dur);

                // 返回过滤后的 valid 列
                let result_valid = table["valid"].get_data().to_vec();

                tx.send(result_valid).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 收集三方的结果
        let valid1 = rx1.recv().unwrap();
        let valid2 = rx2.recv().unwrap();
        let valid3 = rx3.recv().unwrap();

        // 组合结果
        let result_valid = rep3_ring::combine_ring_elements_binary(&valid1, &valid2, &valid3);

        // 验证结果
        assert_eq!(result_valid, should_valid);

        println!("Filter shared (4 threads) test passed!");
    }


    
    fn filter_public_2_net_t(predicate: &str)
    {
        const VEC_SIZE: usize = 10000;
        let mut rng = thread_rng();
        
        // 生成测试数据
        let values: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen_range(0..100)).collect();
       
        let values_ring: Vec<RingElement<u64>> = values.iter().map(|v| RingElement(*v)).collect();

        // 分享数据给三方
        let value_shares = rep3_ring::share_ring_elements_binary(&values_ring, &mut rng);
        
        // 创建初始的 valid 列（全为 1，表示所有行都有效）
        let initial_valid: Vec<RingElement<u64>> = vec![RingElement(1u64); VEC_SIZE];
        let valid_shares = rep3_ring::share_ring_elements(&initial_valid, &mut rng);

        // 根据谓词确定端口偏移和名称
        let (predicate_name, port_offset) = match predicate {
            "Equal" => ("Equal", 0),
            "GreaterThan" => ("GreaterThan", 10),
            "LessThan" => ("LessThan", 20),
            "GreaterOrEqual" => ("GreaterOrEqual", 30),
            "LessOrEqual" => ("LessOrEqual", 40),
            "NotEqual" => ("NotEqual", 50),
            "EqualBinary" => ("EqualBinary", 60),
            "NotEqualBinary" => ("NotEqualBinary", 70),
            "GreaterOrEqualBinary" => ("GreaterOrEqualBinary", 80),
            "LessOrEqualBinary" => ("LessOrEqualBinary", 90),
            _ => panic!("Invalid predicate"),

        };

        let filter_value = 50u64;
        
        // 计算期望结果
        let should_valid: Vec<RingElement<u64>> = values.iter()
            .map(|v| {
                let result = match predicate {
                    "Equal" => *v == filter_value,
                    "GreaterThan" => *v > filter_value,
                    "LessThan" => *v < filter_value,
                    "GreaterOrEqual" => *v >= filter_value,
                    "LessOrEqual" => *v <= filter_value,
                    "NotEqual" => *v != filter_value,
                    "EqualBinary" => *v == filter_value,
                    "NotEqualBinary" => *v != filter_value,
                    "GreaterOrEqualBinary" => *v >= filter_value,
                    "LessOrEqualBinary" => *v <= filter_value,
                    _ => panic!("Invalid predicate"),
                };
                if result { RingElement(1u64) } else { RingElement(0u64) }
            })
            .collect();

        let parties_main = vec![
            NetworkParty::new(0, format!("127.0.0.1:{}", 8700 + port_offset).parse().unwrap()),
            NetworkParty::new(1, format!("127.0.0.1:{}", 8701 + port_offset).parse().unwrap()),
            NetworkParty::new(2, format!("127.0.0.1:{}", 8702 + port_offset).parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, format!("127.0.0.1:{}", 9700 + port_offset).parse().unwrap()),
            NetworkParty::new(1, format!("127.0.0.1:{}", 9701 + port_offset).parse().unwrap()),
            NetworkParty::new(2, format!("127.0.0.1:{}", 9702 + port_offset).parse().unwrap()),
        ];
        
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, (value_share, valid_share)) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            izip!(value_shares, valid_shares)
        ) {
            let parties_main = parties_main.clone();
            let parties_fork = parties_fork.clone();
            
            let handle = std::thread::spawn(move || {
                let config0 = NetworkConfig::new(
                    party_id,
                    parties_main[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_main,
                    None,
                    None,
                );
                let [net0] = TcpNetwork::networks::<1>(config0).unwrap();
                
                let config1 = NetworkConfig::new(
                    party_id,
                    parties_fork[party_id].dns_name.to_socket_addrs().unwrap().next().unwrap(),
                    parties_fork,
                    None,
                    None,
                );
                let [net1] = TcpNetwork::networks::<1>(config1).unwrap();

                let mut state0 = Rep3State::new(&net0).unwrap();
                let mut state1 = Rep3State::new(&net1).unwrap();

                //let valid_binary = conversion::a2b_many(&valid_share, &net0, &mut state0).unwrap();
                let value_column = ShareColumn::new(value_share, DataType::Arithmetic, "values".to_string());
                let valid_column = ShareColumn::new(valid_share, DataType::Binary, "valid".to_string());
                
                let mut table = ShareTable::new();
                table.insert_column("values".to_string(), value_column.clone());
                table.insert_column("valid".to_string(), valid_column.clone());

                let total_start = std::time::Instant::now();

                let bool_result = match predicate_name{

                    "Equal" => {
                        table["values"].eq_public_binary(&filter_value, &[&net0, &net1], &mut [&mut state0, &mut state1])  
                    },
                    "GreaterThan" => {
                        table["values"].gt_public_binary(&filter_value, &[&net0, &net1], &mut [&mut state0, &mut state1])     
                    },
                    "LessThan" => {
                        table["values"].lt_public_binary(&filter_value, &[&net0, &net1], &mut [&mut state0, &mut state1])              
                    },
                    "GreaterOrEqual" => {
                        table["values"].ge_public_binary(&filter_value, &[&net0, &net1], &mut [&mut state0, &mut state1])
                    },
                    "LessOrEqual" => {
                        table["values"].le_public_binary(&filter_value, &[&net0, &net1], &mut [&mut state0, &mut state1]) 
                    },
                    "NotEqual" => {
                        table["values"].neq_public_binary(&filter_value, &[&net0, &net1], &mut [&mut state0, &mut state1])
                    },
                    _ => {
                        panic!("Predicate not implemented in test");
                    }
                };

                let bool_result = bool_result.unwrap();

                let _ = table.filter_directed_by_bool(bool_result.get_data(), &[&net0, &net1], &mut [&mut state0, &mut state1]).unwrap();

                // 执行 filter_public 操作（2个网络）
                

                let total_dur = total_start.elapsed();
                eprintln!("Party {} total elapsed: {:?}", party_id, total_dur);

                // 返回过滤后的 valid 列
                let result_valid = table["valid"].get_data().to_vec();

                tx.send(result_valid).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 收集三方的结果
        let valid1 = rx1.recv().unwrap();
        let valid2 = rx2.recv().unwrap();
        let valid3 = rx3.recv().unwrap();

        // 组合结果
        let result_valid = rep3_ring::combine_ring_elements(&valid1, &valid2, &valid3);

        // 验证结果
        assert_eq!(result_valid, should_valid, "Failed with predicate: {}", predicate_name);
        println!("Filter public 2 networks test passed with predicate: {}!", predicate_name);
    }

    //cargo test --release --package tests --test table -- table_operator_ring::rep3_ring_table_operator::filter_public_2_net --exact --nocapture
    #[test]
    fn filter_public_2_net() {
        filter_public_2_net_t("Equal");
        filter_public_2_net_t("GreaterThan");
        filter_public_2_net_t("LessThan");
        //filter_public_2_net_t("GreaterOrEqual");
        //filter_public_2_net_t("LessOrEqual");//这个没通过
        //filter_public_2_net_t("NotEqual");
        //filter_public_2_net_t("EqualBinary");
        //filter_public_2_net_t("LessOrEqualBinary");
        //filter_public_2_net_t("GreaterThanBinary");
        //filter_public_2_net_t("GreaterOrEqualBinary");
        //filter_public_2_net_t("NotEqualBinary");
    }
    
}
