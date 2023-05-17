use std::sync::Arc;

use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

#[tokio::main]
async fn main() {
    let (tx_to_pool, rx_from_thread) = mpsc::channel(1);
    let (tx_to_thread, mut rx_from_pool) = mpsc::channel(1);

    let data_pool_task = tokio::spawn(async move { data_pool(tx_to_thread, rx_from_thread).await });

    let mut threads = Vec::new();
    for i in 1..10 {
        let new_port = tx_to_pool.clone();
        let thread: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            for j in 1..20 {
                new_port.send(i * 10 + j).await.expect(&format!(
                    "There was an error while waiting for message to send to the channel thread: {i}"
                ));
            }
        });

        threads.push(thread);
    }

    while let Some(data) = rx_from_pool.recv().await {
        println!("{data}");
    }

    for i in 0..10 {
        threads
            .pop()
            .unwrap()
            .await
            .expect(&format!("There was an error while for thread {i} to stop",));
    }

    data_pool_task
        .await
        .expect("There was an error while waiting for the pool task to stop");
}

async fn data_pool(tx: Sender<usize>, mut rx: Receiver<usize>) {
    let pool = Arc::new(Mutex::new(Vec::new()));

    let read_pool = pool.clone();
    let reader_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            read_pool.lock().await.push(data);

            let length = read_pool.lock().await.len();
            if length > 0 {
                println!("Length of the queue is: {}", length);
            }
        }
    });

    let write_pool = pool.clone();
    let writer_task = tokio::spawn(async move {
        loop {
            if !write_pool.lock().await.is_empty() {
                tx.send(write_pool.lock().await.remove(0))
                    .await
                    .expect("There was an error while sending data to the channel");
            }
        }
    });

    reader_task
        .await
        .expect("There was an error while waiting for the reader task to stop");
    writer_task
        .await
        .expect("There was an error while waiting for the writer task to stop");
}
