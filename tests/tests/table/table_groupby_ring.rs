mod rep3_ring_table_groupby{

    use itertools::izip;
    use protocols::rep3_ring::Rep3State;
    use protocols::rep3_ring;
    use protocols::rep3_ring::ring::ring_impl::RingElement;
    use rand::thread_rng;
    use rand::Rng;
    use std::sync::mpsc;
    use net::tcp::{TcpNetwork, NetworkConfig, NetworkParty};
    use std::net::{ToSocketAddrs};

    macro_rules! apply_to_all {
        ($expr:ident,[$($t:ty),*]) => {
            $(
                $expr::<$t>();
            )*
        };
    }


    #[test]
    fn tcp_muti_key_group_by_common() {
        //还有一个问题，gen_bit_perm还没有
        const VEC_SIZE: usize = 10;
        const CHUNK_SIZE: usize = 64;

        let mut rng = thread_rng();
        // Key 1
        let keys1: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>() % 5).collect();
        // Key 2
        let keys2: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>() % 5).collect();
        
        let values: Vec<u64> = (0..VEC_SIZE).map(|_| rng.gen::<u64>()% 5).collect();
        let valid: Vec<u64> = vec![1; VEC_SIZE]; // All valid

        // Convert to RingElement
        let keys1_ring: Vec<RingElement<u64>> = keys1.iter().map(|v| RingElement(*v)).collect();
        let keys2_ring: Vec<RingElement<u64>> = keys2.iter().map(|v| RingElement(*v)).collect();
        let values_ring: Vec<RingElement<u64>> = values.iter().map(|v| RingElement(*v)).collect();
        let valid_ring: Vec<RingElement<u64>> = valid.iter().map(|v| RingElement(*v)).collect();

        // Share
        let keys1_shares = rep3_ring::share_ring_elements(&keys1_ring, &mut rng);
        let keys2_shares = rep3_ring::share_ring_elements(&keys2_ring, &mut rng);
        let value_shares = rep3_ring::share_ring_elements(&values_ring, &mut rng);
        let valid_shares = rep3_ring::share_ring_elements(&valid_ring, &mut rng);

        let order = true;

        // Network setup (Ports 8500-8502, 9500-9502)
        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8500".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8501".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8502".parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, "127.0.0.1:9500".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9501".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9502".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, keys1_share, keys2_share, value_share, valid_share) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            keys1_shares, keys2_shares, value_shares, valid_shares
        ) {
            let parties_main = parties_main.clone();
            let parties_fork = parties_fork.clone();
            let order = order.clone();
            
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

                let keys_refs = vec![keys1_share.as_slice(), keys2_share.as_slice()];
                let total_start = std::time::Instant::now();
                let result = operator::group_by_onethread::muti_key_group_by_common(
                    keys_refs,
                    &value_share,
                    &valid_share,
                    order,
                    CHUNK_SIZE,
                    &net0,
                    &net1,
                    &mut state0,
                    &mut state1,
                ).unwrap();
                let total_dur = total_start.elapsed();
                eprintln!("Party {} group_by elapsed: {:?}", party_id, total_dur);
                
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
         

        // Reconstruct result
        // (k_g, v_g, e_t_res, k_g_n_vec, perm_e, k_out_vec)
        // k_g is Vec<Vec<Share>>
        
        // Reconstruct k_g (Grouped Keys)
        let mut k_g_reconstructed = Vec::new();
        for i in 0..res1.0.len() { //For each key column
            let col = rep3_ring::combine_ring_elements(&res1.0[i], &res2.0[i], &res3.0[i]);
            k_g_reconstructed.push(col);
        }

        // Reconstruct v_g (Grouped Values)
        let v_g_reconstructed = rep3_ring::combine_ring_elements(&res1.1, &res2.1, &res3.1);

        // Reconstruct e_t_res
        let e_t_res_reconstructed = rep3_ring::combine_ring_elements(&res1.2, &res2.2, &res3.2);

        // Reconstruct k_g_n_vec
        let mut k_g_n_reconstructed = Vec::new();
        for i in 0..res1.3.len() {
            let col = rep3_ring::combine_ring_elements(&res1.3[i], &res2.3[i], &res3.3[i]);
            k_g_n_reconstructed.push(col);
        }

        // Reconstruct k_out_vec
        let mut k_out_reconstructed = Vec::new();
        for i in 0..res1.5.len() {
            let col = rep3_ring::combine_ring_elements(&res1.5[i], &res2.5[i], &res3.5[i]);
            k_out_reconstructed.push(col);
        }

        // Reconstruct new valid
        let new_valid = rep3_ring::combine_ring_elements(&res1.7, &res2.7, &res3.7);
        println!("New Valid: {:?}", new_valid);

        println!("Original Keys 1: {:?}", keys1);
        println!("Original Keys 2: {:?}", keys2);
        println!("Original Values: {:?}", values);
        
        println!("Grouped Keys (k_g):");
        for (i, col) in k_g_reconstructed.iter().enumerate() {
            println!("  Key {}: {:?}", i, col);
        }
        
        println!("Grouped Values (v_g): {:?}", v_g_reconstructed);
        println!("Edge Indicators (e_t_res): {:?}", e_t_res_reconstructed);
        
        println!("New Keys (k_g_n):");
        for (i, col) in k_g_n_reconstructed.iter().enumerate() {
            println!("  Key {}: {:?}", i, col);
        }

        println!("Output Keys (k_out):");
        for (i, col) in k_out_reconstructed.iter().enumerate() {
            println!("  Key {}: {:?}", i, col);
        }

        // Verify sorting (primary key is key1, secondary is key2)
        // Since we sort by multiple keys, we check if the rows are sorted lexicographically
        let rows = k_g_reconstructed[0].len();
        for i in 0..rows - 1 {
            let k1_curr = k_g_reconstructed[0][i].0;
            let k1_next = k_g_reconstructed[0][i+1].0;
            let k2_curr = k_g_reconstructed[1][i].0;
            let k2_next = k_g_reconstructed[1][i+1].0;

            if k1_curr < k1_next {
                continue;
            } else if k1_curr == k1_next {
                assert!(k2_curr <= k2_next, "Secondary key not sorted at index {}: ({}, {}) > ({}, {})", i, k1_curr, k2_curr, k1_next, k2_next);
            } else {
                panic!("Primary key not sorted at index {}: {} > {}", i, k1_curr, k1_next);
            }
        }

        println!("Multi-key Group by test passed!");
        
    }

    
    
    #[test]
    fn tcp_agg_sum_test() {
        const VEC_SIZE: usize = 10;

        let mut rng = thread_rng();
        // Create test data
        // Key 1: [1, 1, 1, 2, 2, 2, 3, 3, 3, 3]
        // Key 2: [1, 1, 2, 1, 1, 2, 1, 1, 2, 2]
        // Values: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        
        let keys1: Vec<u64> = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 3];
        let keys2: Vec<u64> = vec![1, 1, 2, 1, 1, 2, 1, 1, 2, 2];
        let values: Vec<u64> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let valid: Vec<u64> = vec![1; VEC_SIZE];

        let keys1_ring: Vec<RingElement<u64>> = keys1.iter().map(|v| RingElement(*v)).collect();
        let keys2_ring: Vec<RingElement<u64>> = keys2.iter().map(|v| RingElement(*v)).collect();
        let values_ring: Vec<RingElement<u64>> = values.iter().map(|v| RingElement(*v)).collect();
        let valid_ring: Vec<RingElement<u64>> = valid.iter().map(|v| RingElement(*v)).collect();

        let keys1_shares = rep3_ring::share_ring_elements(&keys1_ring, &mut rng);
        let keys2_shares = rep3_ring::share_ring_elements(&keys2_ring, &mut rng);
        let value_shares = rep3_ring::share_ring_elements(&values_ring, &mut rng);
        let valid_shares = rep3_ring::share_ring_elements(&valid_ring, &mut rng);

        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8600".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8601".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8602".parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, "127.0.0.1:9600".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9601".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9602".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, keys1_share, keys2_share, value_share, valid_share) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            keys1_shares, keys2_shares, value_shares, valid_shares
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

                let keys_refs = vec![keys1_share.as_slice(), keys2_share.as_slice()];
                let result = operator::group_by_onethread::agg_count(
                    keys_refs,
                    &value_share,
                    &valid_share,
                    &net0,
                    &net1,
                    &mut state0,
                    &mut state1,
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

        let s_reconstructed = rep3_ring::combine_ring_elements(&res1.0, &res2.0, &res3.0);
        
        let mut k_out_reconstructed = Vec::new();
        for i in 0..res1.1.len() {
            let col = rep3_ring::combine_ring_elements(&res1.1[i], &res2.1[i], &res3.1[i]);
            k_out_reconstructed.push(col);
        }

        println!("Aggregated Sums: {:?}", s_reconstructed);
        println!("Output Keys:");
        for (i, col) in k_out_reconstructed.iter().enumerate() {
            println!("  Key {}: {:?}", i, col);
        }

        let expected_sums: std::collections::HashMap<(u64, u64), u64> = [
            ((1, 1), 3),
            ((1, 2), 3),
            ((2, 1), 9),
            ((2, 2), 6),
            ((3, 1), 15),
            ((3, 2), 19),
        ].iter().cloned().collect();

        for ((k1, k2), expected_sum) in expected_sums {
             let mut found = false;
             for i in 0..s_reconstructed.len() {
                 if k_out_reconstructed[0][i].0 == k1 && k_out_reconstructed[1][i].0 == k2 {
                     if s_reconstructed[i].0 == expected_sum {
                         found = true;
                         break;
                     }
                 }
             }
             assert!(found, "Could not find sum {} for key ({}, {})", expected_sum, k1, k2);
        }
        
        println!("Agg Sum test passed!");
    }

    #[test]
    fn tcp_agg_count_test() {
        const VEC_SIZE: usize = 10;

        let mut rng = thread_rng();
        // Create test data
        // Key 1: [1, 1, 1, 2, 2, 2, 3, 3, 3, 3]
        // Key 2: [1, 1, 2, 1, 1, 2, 1, 1, 2, 2]
        // Values: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        
        let keys1: Vec<u64> = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 3];
        let keys2: Vec<u64> = vec![1, 1, 2, 1, 1, 2, 1, 1, 2, 2];
        let values: Vec<u64> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let valid: Vec<u64> = vec![1; VEC_SIZE];

        let keys1_ring: Vec<RingElement<u64>> = keys1.iter().map(|v| RingElement(*v)).collect();
        let keys2_ring: Vec<RingElement<u64>> = keys2.iter().map(|v| RingElement(*v)).collect();
        let values_ring: Vec<RingElement<u64>> = values.iter().map(|v| RingElement(*v)).collect();
        let valid_ring: Vec<RingElement<u64>> = valid.iter().map(|v| RingElement(*v)).collect();

        let keys1_shares = rep3_ring::share_ring_elements(&keys1_ring, &mut rng);
        let keys2_shares = rep3_ring::share_ring_elements(&keys2_ring, &mut rng);
        let value_shares = rep3_ring::share_ring_elements(&values_ring, &mut rng);
        let valid_shares = rep3_ring::share_ring_elements(&valid_ring, &mut rng);

        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8700".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8701".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8702".parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, "127.0.0.1:9700".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9701".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9702".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, keys1_share, keys2_share, value_share, valid_share) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            keys1_shares, keys2_shares, value_shares, valid_shares
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

                let keys_refs = vec![keys1_share.as_slice(), keys2_share.as_slice()];
                let result = operator::group_by_onethread::agg_count(
                    keys_refs,
                    &value_share,
                    &valid_share,
                    &net0,
                    &net1,
                    &mut state0,
                    &mut state1,
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

        let s_reconstructed = rep3_ring::combine_ring_elements(&res1.0, &res2.0, &res3.0);
        
        let mut k_out_reconstructed = Vec::new();
        for i in 0..res1.1.len() {
            let col = rep3_ring::combine_ring_elements(&res1.1[i], &res2.1[i], &res3.1[i]);
            k_out_reconstructed.push(col);
        }

        println!("Aggregated Counts: {:?}", s_reconstructed);
        println!("Output Keys:");
        for (i, col) in k_out_reconstructed.iter().enumerate() {
            println!("  Key {}: {:?}", i, col);
        }

        let expected_counts: std::collections::HashMap<(u64, u64), u64> = [
            ((1, 1), 2),
            ((1, 2), 1),
            ((2, 1), 2),
            ((2, 2), 1),
            ((3, 1), 2),
            ((3, 2), 2),
        ].iter().cloned().collect();

        for ((k1, k2), expected_count) in expected_counts {
             let mut found = false;
             for i in 0..s_reconstructed.len() {
                 if k_out_reconstructed[0][i].0 == k1 && k_out_reconstructed[1][i].0 == k2 {
                     if s_reconstructed[i].0 == expected_count {
                         found = true;
                         break;
                     }
                 }
             }
             assert!(found, "Could not find count {} for key ({}, {})", expected_count, k1, k2);
        }
        
        println!("Agg Count test passed!");
    }

    #[test]
    fn agg_count_by_valid_test() {
        const VEC_SIZE: usize = 10;

        let mut rng = thread_rng();
        // Create test data
        // Key 1: [1, 1, 1, 2, 2, 2, 3, 3, 3, 3]
        // Key 2: [1, 1, 2, 1, 1, 2, 1, 1, 2, 2]
        // Values: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        
        let keys1: Vec<u64> = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 3];
        let keys2: Vec<u64> = vec![1, 1, 2, 1, 1, 2, 1, 1, 2, 2];
        let values: Vec<u64> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let valid: Vec<u64> = vec![1; VEC_SIZE];

        let keys1_ring: Vec<RingElement<u64>> = keys1.iter().map(|v| RingElement(*v)).collect();
        let keys2_ring: Vec<RingElement<u64>> = keys2.iter().map(|v| RingElement(*v)).collect();
        let values_ring: Vec<RingElement<u64>> = values.iter().map(|v| RingElement(*v)).collect();
        let valid_ring: Vec<RingElement<u64>> = valid.iter().map(|v| RingElement(*v)).collect();

        let keys1_shares = rep3_ring::share_ring_elements(&keys1_ring, &mut rng);
        let keys2_shares = rep3_ring::share_ring_elements(&keys2_ring, &mut rng);
        let value_shares = rep3_ring::share_ring_elements(&values_ring, &mut rng);
        let valid_shares = rep3_ring::share_ring_elements(&valid_ring, &mut rng);

        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8700".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8701".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8702".parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, "127.0.0.1:9700".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9701".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9702".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, keys1_share, keys2_share, value_share, valid_share) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            keys1_shares, keys2_shares, value_shares, valid_shares
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

                let keys_refs = vec![keys1_share.as_slice(), keys2_share.as_slice()];
                let result = operator::group_by_onethread::agg_count_by_valid(
                    keys_refs,
                    &value_share,
                    &valid_share,
                    &net0,
                    &net1,
                    &mut state0,
                    &mut state1,
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

        let s_reconstructed = rep3_ring::combine_ring_elements(&res1.0, &res2.0, &res3.0);
        
        let mut k_out_reconstructed = Vec::new();
        for i in 0..res1.1.len() {
            let col = rep3_ring::combine_ring_elements(&res1.1[i], &res2.1[i], &res3.1[i]);
            k_out_reconstructed.push(col);
        }

        println!("Aggregated Counts: {:?}", s_reconstructed);
        println!("Output Keys:");
        for (i, col) in k_out_reconstructed.iter().enumerate() {
            println!("  Key {}: {:?}", i, col);
        }

        let expected_counts: std::collections::HashMap<(u64, u64), u64> = [
            ((1, 1), 2),
            ((1, 2), 1),
            ((2, 1), 2),
            ((2, 2), 1),
            ((3, 1), 2),
            ((3, 2), 2),
        ].iter().cloned().collect();

        for ((k1, k2), expected_count) in expected_counts {
             let mut found = false;
             for i in 0..s_reconstructed.len() {
                 if k_out_reconstructed[0][i].0 == k1 && k_out_reconstructed[1][i].0 == k2 {
                     if s_reconstructed[i].0 == expected_count {
                         found = true;
                         break;
                     }
                 }
             }
             assert!(found, "Could not find count {} for key ({}, {})", expected_count, k1, k2);
        }
        
        println!("Agg Count test passed!");
    }
}
