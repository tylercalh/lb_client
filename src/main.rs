use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpStream};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;
use std::env;

fn main() -> std::io::Result<()> {
    const CLIENTS: u128 = 5;
    const HOST: &str = "127.0.0.1:8085";
    const REQ: u8 = 40;

    // Collect command line args.
    let mut args = env::args().collect::<VecDeque<String>>();
    args.pop_front();
    
    // No error Checking...
    let host_ip = match args.pop_front() {
        Some(ip) => ip,
        None => String::from(HOST),
    };
    let num_clients = match args.pop_front() {
        Some(n) => n.parse::<u128>().unwrap(),
        None => CLIENTS,
    };

    let mut handles = Vec::with_capacity(num_clients as usize);

    // Allocate space for storing message passing channels.
    let mut channels: Vec<Receiver<BenchmarkInfo>> = Vec::with_capacity(num_clients as usize);
    
    // Begin stopwatch for throughput.
    let throughput = Instant::now();
    
    // Create threads for each client.
    for _i in 0..num_clients {
        
        // Create a channel for the thread to send its turnaround time.
        let (tx, rx) = mpsc::channel();
        channels.push(rx);

        // Each thread needs its own copy of the host ip.
        let thread_host_ip = host_ip.clone();

        // Spawn the client threads.
        handles.push( thread::spawn(move || {
            // Connect to the host.
            let mut stream = TcpStream::connect(thread_host_ip)
                .unwrap();
            
            // Send a request to the host.
            stream.write(&[REQ]).unwrap();

            // Begin stopwatch for turnaround time.
            let turnaround_time = Instant::now();

            // Response received, read the response.
            let mut buf_in = [0; 4];
            stream.read(&mut buf_in).unwrap();

            // Store the threads benchmark info.
            let bi_client = stream.local_addr().unwrap().to_string();
            let bi_server = buf_in;
            let bi_tat = turnaround_time.elapsed().as_millis();
            let bi = BenchmarkInfo::new(bi_client, bi_server, bi_tat);
            println!("{:?}", bi);
            tx.send(bi).unwrap();
        }));
    }
    
    // Wait for clients to finish up.
    for handle in handles {
        handle.join().unwrap();
    }

    // Calculate throughput.
    let total_time = throughput.elapsed().as_millis();
    let throughput = total_time / num_clients;

    // Collect thread's benchmark info.
    let bis = channels
        .iter()
        .map(|x| x.recv().unwrap())
        .collect::<Vec<BenchmarkInfo>>();

    // Read the turnaround times from the benchmark infos. Calculate the average.
    let total_tat = bis
        .iter()
        .fold(0, |acc, x| acc + x.turnaround_time);
    let avg_tat = total_tat / num_clients;

    println!("Total time: {} Throughput: {} Average Turnaround Time {}", total_time, throughput, avg_tat);
    
    Ok(())
}

#[derive(Debug)]
struct BenchmarkInfo {
    client: String,
    server: String,
    turnaround_time: u128,
}

impl BenchmarkInfo {
    fn new(client: String, server_bytes: [u8; 4], turnaround_time: u128) -> Self {
        // Build server ip.
        let ip_v4 = Ipv4Addr::from(server_bytes);
        let ip = IpAddr::V4(ip_v4);

        Self {
            client: client,
            server: ip.to_string(),
            turnaround_time: turnaround_time,
        }
    }
}