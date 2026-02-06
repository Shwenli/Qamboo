mod rep3_ring_table_join {
    use itertools::izip;
    use protocols::protocols::rep3_ring::Rep3State;
    use protocols::protocols::rep3_ring;
    use protocols::protocols::rep3_ring::ring::ring_impl::RingElement;
    use rand::thread_rng;
    use rand::Rng;
    use table::NetStateArgs;
    use std::sync::mpsc;
    use net::tcp::{TcpNetwork, NetworkConfig, NetworkParty};
    use std::net::{ToSocketAddrs};
    use table::share_table::ShareTable;
    use table::share_column::{ShareColumn, DataType};
    use table::table_operator::Join;
    use operator::join_onethread;


    #[test]
    fn tcp_inner_join_test() {
        const CHUNK_SIZE: usize = 64;
        const VEC_SIZE_L: usize = 1000000;
        const VEC_SIZE_R: usize = 6000000;

        let mut rng = thread_rng();

        let l_ids: Vec<u64> = (0..VEC_SIZE_L).map(|_| rng.gen::<u64>()).collect();
        let l_vals: Vec<u64> = (0..VEC_SIZE_L).map(|_| rng.gen::<u64>()).collect();
        let l_validity: Vec<u64> = (0..VEC_SIZE_L).map(|_| 1).collect();

        let r_ids: Vec<u64> = (0..VEC_SIZE_R).map(|_| rng.gen::<u64>()).collect();
        let r_vals: Vec<u64> = (0..VEC_SIZE_R).map(|_| rng.gen::<u64>()).collect();
        let r_validity: Vec<u64> = (0..VEC_SIZE_R).map(|_| 1).collect();

        
        // Define data
        // L: (id, value)
        /* 
        let l_ids: Vec<u64> = vec![1, 2, 3, 4];
        let l_vals: Vec<u64> = vec![10, 20, 30, 40];
        let l_validity: Vec<u64> = vec![1,1,1,1];
        */
        
        // R: (id, value)
        /*
        let r_ids: Vec<u64> = vec![1, 1, 2, 5];
        let r_vals: Vec<u64> = vec![100, 200, 300, 400];
        let r_validity: Vec<u64> = vec![1,1,1,1];
        */


        // Convert to RingElement
        let l_ids_ring: Vec<RingElement<u64>> = l_ids.iter().map(|v| RingElement(*v)).collect();
        let l_vals_ring: Vec<RingElement<u64>> = l_vals.iter().map(|v| RingElement(*v)).collect();
        let l_validity_ring: Vec<RingElement<u64>> = l_validity.iter().map(|v| RingElement(*v)).collect();
        let r_ids_ring: Vec<RingElement<u64>> = r_ids.iter().map(|v| RingElement(*v)).collect();
        let r_vals_ring: Vec<RingElement<u64>> = r_vals.iter().map(|v| RingElement(*v)).collect();
        let r_validity_ring: Vec<RingElement<u64>> = r_validity.iter().map(|v| RingElement(*v)).collect();

        // Share data
        let l_ids_shares = rep3_ring::share_ring_elements(&l_ids_ring, &mut rng);
        let l_vals_shares = rep3_ring::share_ring_elements(&l_vals_ring, &mut rng);
        let l_validity_shares = rep3_ring::share_ring_elements(&l_validity_ring, &mut rng);
        let r_ids_shares = rep3_ring::share_ring_elements(&r_ids_ring, &mut rng);
        let r_vals_shares = rep3_ring::share_ring_elements(&r_vals_ring, &mut rng);
        let r_validity_shares = rep3_ring::share_ring_elements(&r_validity_ring, &mut rng);

        // Network setup
        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8300".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8301".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8302".parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, "127.0.0.1:9300".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9301".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9302".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, l_id_share, l_val_share, l_validity_share, r_id_share, r_val_share, r_validity_share) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            l_ids_shares, l_vals_shares, l_validity_shares, r_ids_shares, r_vals_shares, r_validity_shares
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

                // Prepare inputs: L = [ids, vals], R = [ids, vals]
                let l = vec![l_id_share, l_val_share];
                let r = vec![r_id_share, r_val_share];

                let total_start = std::time::Instant::now();
                let result = join_onethread::inner_join(
                    0, // k_l_idx
                    0, // k_r_idx
                    l,
                    r,
                    &l_validity_share,
                    &r_validity_share,
                    CHUNK_SIZE,
                    &net0,
                    &net1,
                    &mut state0,
                    &mut state1,
                ).unwrap();
                let total_time = total_start.elapsed();
                eprintln!("Total time: {:?}", total_time);

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
        // Result structure:
        // j[0]: k_r (original R keys)
        // j[1]: jl_k (L keys aligned to R)
        // j[2]: jl_v (L values aligned to R)
        // j[3]: jr_k (R keys scaled)
        // j[4]: jr_v (R values scaled)
        // j[5]: vj (validity)

        let mut reconstructed = Vec::new();
        for i in 0..res1.len() {
            let col = rep3_ring::combine_ring_elements(&res1[i], &res2[i], &res3[i]);
            reconstructed.push(col);
        }

        //let res_kr = &reconstructed[0];
        //let res_jl_k = &reconstructed[1];
        //let res_jl_v = &reconstructed[2];
        //let res_jr_k = &reconstructed[3];
        //let res_jr_v = &reconstructed[4];
        //let res_vj = &reconstructed[5];

        //println!("Reconstructed Results:");
        //println!("k_r: {:?}", res_kr);
        //println!("jl_k: {:?}", res_jl_k);
        //println!("jl_v: {:?}", res_jl_v);
        //println!("jr_k: {:?}", res_jr_k);
        //println!("jr_v: {:?}", res_jr_v);
        //println!("vj: {:?}", res_vj);

        // Verify with SQLite
       
    }
    
    /* 
    #[test]
    fn tcp_table_inner_join_test() {
        ///如果发现v_j有重复值，这是因为，右表的一行有多个左表行匹配，变成了non-unique non-unique
        const CHUNK_SIZE: usize = 64;
        const VEC_SIZE_L: usize = 1000000;
        const VEC_SIZE_R: usize = 1000000;

        let mut rng = thread_rng();
        
        //let l_ids: Vec<u64> = vec![1,2,3,4,5,6,7,8,9,10];

        
        let l_ids: Vec<u64> = (0..VEC_SIZE_L).map(|_| rng.gen::<u64>()%100000).collect();
        let l_vals: Vec<u64> = (0..VEC_SIZE_L).map(|_| rng.gen::<u64>()).collect();
        let l_validity: Vec<u64> = (0..VEC_SIZE_L).map(|_| 1).collect();

        let r_ids: Vec<u64> = (0..VEC_SIZE_R).map(|_| rng.gen::<u64>()).collect();
        let r_vals: Vec<u64> = (0..VEC_SIZE_R).map(|_| rng.gen::<u64>()).collect();
        let r_validity: Vec<u64> = (0..VEC_SIZE_R).map(|_| 1).collect();
        
    
        // Define data
        // L: (id, value)
        /* 
        let l_ids: Vec<u64> = vec![1, 2, 3, 4];
        let l_vals: Vec<u64> = vec![10, 20, 30, 40];
        let l_validity: Vec<u64> = vec![1,1,1,1];
        
        
        // R: (id, value)
        
        let r_ids: Vec<u64> = vec![1, 1, 2, 5];
        let r_vals: Vec<u64> = vec![100, 200, 300, 400];
        let r_validity: Vec<u64> = vec![1,1,1,1];
        */


        // Convert to RingElement
        let l_ids_ring: Vec<RingElement<u64>> = l_ids.iter().map(|v| RingElement(*v)).collect();
        let l_vals_ring: Vec<RingElement<u64>> = l_vals.iter().map(|v| RingElement(*v)).collect();
        let l_validity_ring: Vec<RingElement<u64>> = l_validity.iter().map(|v| RingElement(*v)).collect();
        let r_ids_ring: Vec<RingElement<u64>> = r_ids.iter().map(|v| RingElement(*v)).collect();
        let r_vals_ring: Vec<RingElement<u64>> = r_vals.iter().map(|v| RingElement(*v)).collect();
        let r_validity_ring: Vec<RingElement<u64>> = r_validity.iter().map(|v| RingElement(*v)).collect();

        // Share data
        let l_ids_shares = rep3_ring::share_ring_elements(&l_ids_ring, &mut rng);
        let l_vals_shares = rep3_ring::share_ring_elements(&l_vals_ring, &mut rng);
        let l_validity_shares = rep3_ring::share_ring_elements(&l_validity_ring, &mut rng);
        let r_ids_shares = rep3_ring::share_ring_elements(&r_ids_ring, &mut rng);
        let r_vals_shares = rep3_ring::share_ring_elements(&r_vals_ring, &mut rng);
        let r_validity_shares = rep3_ring::share_ring_elements(&r_validity_ring, &mut rng);

        // Network setup
        let parties_main = vec![
            NetworkParty::new(0, "127.0.0.1:8300".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:8301".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:8302".parse().unwrap()),
        ];
        
        let parties_fork = vec![
            NetworkParty::new(0, "127.0.0.1:9300".parse().unwrap()),
            NetworkParty::new(1, "127.0.0.1:9301".parse().unwrap()),
            NetworkParty::new(2, "127.0.0.1:9302".parse().unwrap()),
        ];

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, l_id_share, l_val_share, l_validity_share, r_id_share, r_val_share, r_validity_share) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            l_ids_shares, l_vals_shares, l_validity_shares, r_ids_shares, r_vals_shares, r_validity_shares
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

                let mut state0 = Rep3State::new(&net0, A2BType::default()).unwrap();
                let mut state1 = Rep3State::new(&net1, A2BType::default()).unwrap();

                // Prepare inputs: L = [ids, vals], R = [ids, vals]
                let l_pk = ShareColumn::new(l_id_share, DataType::Arithmetic, "pk_l".to_string());
                let l_val = ShareColumn::new(l_val_share, DataType::Arithmetic, "val_l".to_string());
                let l_valid = ShareColumn::new(l_validity_share, DataType::Arithmetic, "valid".to_string());
                let mut l = ShareTable::new();
                l.insert_column("pk_l".to_string(), l_pk);
                l.insert_column("val_l".to_string(), l_val);
                l.insert_column("valid".to_string(), l_valid);

                let r_pk = ShareColumn::new(r_id_share, DataType::Arithmetic, "pk_r".to_string());
                let r_val = ShareColumn::new(r_val_share, DataType::Arithmetic, "val_r".to_string());
                let r_valid = ShareColumn::new(r_validity_share, DataType::Arithmetic, "valid".to_string());
                let mut r = ShareTable::new();
                r.insert_column("pk_r".to_string(), r_pk);
                r.insert_column("val_r".to_string(), r_val);
                r.insert_column("valid".to_string(), r_valid);

                let total_start = std::time::Instant::now();
                let result = l.inner_join_multithreads(
                    "pk_l", // k_l_name
                    "pk_r", // k_r_name
                    &mut r,
                    CHUNK_SIZE,
                    &[&net0, &net1],
                    &mut state0,
                    &mut state1,
                    &mut [&mut state0, &mut state1],
                ).unwrap();
                let total_time = total_start.elapsed();
                eprintln!("Total time: {:?}", total_time);

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
        // Result structure:
        // j[0]: k_r (original R keys)
        // j[1]: jl_k (L keys aligned to R)
        // j[2]: jl_v (L values aligned to R)
        // j[3]: jr_k (R keys scaled)
        // j[4]: jr_v (R values scaled)
        // j[5]: vj (validity)

        let mut reconstructed = Vec::new();
        for i in 0..res1.num_columns() {
            let col = rep3_ring::combine_ring_elements(res1.get_column_by_index(i).get_data(), res2.get_column_by_index(i).get_data(), res3.get_column_by_index(i).get_data());
            reconstructed.push(col);
        }
        /* 
        let res_kr = &reconstructed[0];
        let res_jl_k = &reconstructed[1];
        let res_jr_k = &reconstructed[2];
        //let res_jr_k = &reconstructed[3];
        //let res_jr_v = &reconstructed[4];
        let res_vj = &reconstructed[3];
        
        println!("Reconstructed Results:");
        println!("k_r: {:?}", res_kr);
        println!("jl_k: {:?}", res_jl_k);
        println!("jr_k: {:?}", res_jr_k);
        println!("vj: {:?}", res_vj);
        */
        // Verify with SQLite
       
    }
    */


    //* cargo test --release --package tests --test table -- table_join_ring::rep3_ring_table_join::test_join_multithreads --exact --nocapture 
    #[test]
    fn test_join_multithreads() {

        
        //const VEC_SIZE_L: usize = 1000000;
        //const VEC_SIZE_R: usize = 1000000;

        let mut rng = thread_rng();
        
        /* 
        let l_ids: Vec<u64> = (0..VEC_SIZE_L).map(|_| rng.gen::<u64>()%100000).collect();
        let l_vals: Vec<u64> = (0..VEC_SIZE_L).map(|_| rng.gen::<u64>()).collect();
        let l_validity: Vec<u64> = (0..VEC_SIZE_L).map(|_| 1).collect();

        let r_ids: Vec<u64> = (0..VEC_SIZE_R).map(|_| rng.gen::<u64>()).collect();
        let r_vals: Vec<u64> = (0..VEC_SIZE_R).map(|_| rng.gen::<u64>()).collect();
        let r_validity: Vec<u64> = (0..VEC_SIZE_R).map(|_| 1).collect();
        */

        // L: (id, value)
        let l_ids: Vec<u64> = vec![1, 2, 3, 4];
        let l_vals: Vec<u64> = vec![10, 20, 30, 40];
        let l_validity: Vec<u64> = vec![1,1,0,1];
        
        // R: (id, value)
        let r_ids: Vec<u64> = vec![1, 1, 2, 5, 6, 3];
        let r_vals: Vec<u64> = vec![100, 200, 300, 400, 500, 600];
        let r_validity: Vec<u64> = vec![1,1,1,1,1,1];
        

        // Convert to RingElement
        let l_ids_ring: Vec<RingElement<u64>> = l_ids.iter().map(|v| RingElement(*v)).collect();
        let l_vals_ring: Vec<RingElement<u64>> = l_vals.iter().map(|v| RingElement(*v)).collect();
        let l_validity_ring: Vec<RingElement<u64>> = l_validity.iter().map(|v| RingElement(*v)).collect();
        let r_ids_ring: Vec<RingElement<u64>> = r_ids.iter().map(|v| RingElement(*v)).collect();
        let r_vals_ring: Vec<RingElement<u64>> = r_vals.iter().map(|v| RingElement(*v)).collect();
        let r_validity_ring: Vec<RingElement<u64>> = r_validity.iter().map(|v| RingElement(*v)).collect();

        // Share data
        let l_ids_shares = rep3_ring::share_ring_elements(&l_ids_ring, &mut rng);
        let l_vals_shares = rep3_ring::share_ring_elements(&l_vals_ring, &mut rng);
        let l_validity_shares = rep3_ring::share_ring_elements(&l_validity_ring, &mut rng);
        let r_ids_shares = rep3_ring::share_ring_elements(&r_ids_ring, &mut rng);
        let r_vals_shares = rep3_ring::share_ring_elements(&r_vals_ring, &mut rng);
        let r_validity_shares = rep3_ring::share_ring_elements(&r_validity_ring, &mut rng);

        // Network setup
        let mut parties_collection = Vec::new();
        let mut port_counter = 8800;
        for _ in 0..4 {
            let mut parties = Vec::new();
            for party_id in 0..3 {
                parties.push(NetworkParty::new(
                    party_id,
                    format!("127.0.0.1:{}", port_counter).parse().unwrap(),
                ));
                port_counter += 1;
            }
            parties_collection.push(parties);
        }

        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let (tx3, rx3) = mpsc::channel();

        let mut handles = vec![];

        for (party_id, tx, l_id_share, l_val_share, l_validity_share, r_id_share, r_val_share, r_validity_share) in izip!(
            0..3, 
            [tx1, tx2, tx3], 
            l_ids_shares, l_vals_shares, l_validity_shares, r_ids_shares, r_vals_shares, r_validity_shares
        ) {
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

                let mut mpc_exec_args = NetStateArgs::new(&parallel_nets, state_decomp0, state_decomp1, &mut parallel_state_refs);

                // Prepare inputs: L = [ids, vals], R = [ids, vals]
                let l_pk = ShareColumn::new(l_id_share, DataType::Arithmetic, "pk_l".to_string());
                let l_val = ShareColumn::new(l_val_share, DataType::Arithmetic, "val_l".to_string());
                let l_valid = ShareColumn::new(l_validity_share, DataType::Arithmetic, "valid".to_string());
                let mut l = ShareTable::new();
                l.insert_column("pk_l".to_string(), l_pk);
                l.insert_column("val_l".to_string(), l_val);
                l.insert_column("valid".to_string(), l_valid);

                let r_pk = ShareColumn::new(r_id_share, DataType::Arithmetic, "pk_r".to_string());
                let r_val = ShareColumn::new(r_val_share, DataType::Arithmetic, "val_r".to_string());
                let r_valid = ShareColumn::new(r_validity_share, DataType::Arithmetic, "valid".to_string());
                let mut r = ShareTable::new();
                r.insert_column("pk_r".to_string(), r_pk);
                r.insert_column("val_r".to_string(), r_val);
                r.insert_column("valid".to_string(), r_valid);

                let total_start = std::time::Instant::now();
                let result = l.inner_join(
                    "pk_l", // k_l_name
                    "pk_r", // k_r_name
                    &mut r,
                    &mut mpc_exec_args,
                    
                ).unwrap();
                let total_time = total_start.elapsed();
                eprintln!("Total time: {:?}", total_time);

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


        let mut reconstructed = Vec::new();
        for i in 0..res1.num_columns() {
            let col = rep3_ring::combine_ring_elements(res1.get_column_by_index(i).get_data(), res2.get_column_by_index(i).get_data(), res3.get_column_by_index(i).get_data());
            reconstructed.push(col);
        }

         
        let res_kr = &reconstructed[0];
        let res_jl_k = &reconstructed[1];
        let res_jr_k = &reconstructed[2];
        //let res_jr_k = &reconstructed[3];
        //let res_jr_v = &reconstructed[4];
        let res_vj = &reconstructed[3];
        
        println!("Reconstructed Results:");
        println!("k_r: {:?}", res_kr);
        println!("jl_k: {:?}", res_jl_k);
        println!("jr_k: {:?}", res_jr_k);
        println!("vj: {:?}", res_vj);
        
        
        // Verify with SQLite
       

    }
    
}
