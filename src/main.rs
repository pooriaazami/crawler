use std::{fs::File, io::Write};

use tokio::sync::mpsc::{self, Receiver};

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel(2);

    let manager = tokio::spawn(async move {
        write_response_to_file(rx).await;
    });

    let tx2 = tx.clone();
    let t1 = tokio::spawn(async move {
        println!("Starting a new therad");

        tx.send("Line 1_1\n")
            .await
            .expect("There was an error while writing to the channel");

        tx.send("Line 1_2\n")
            .await
            .expect("There was an error while writing to the channel");

        println!("Data has been sent(therad 1)");
    });

    let t2 = tokio::spawn(async move {
        println!("Starting a new therad");

        tx2.send("Line 2_1\n")
            .await
            .expect("There was an error while writing to the channel");

        tx2.send("Line 2_2\n")
            .await
            .expect("There was an error while writing to the channel");

        println!("Data has been sent(therad 2)");
    });

    t1.await
        .expect("There was an error while spawning one of the therads");
    t2.await
        .expect("There was an error while spawning one of the therads");
    manager
        .await
        .expect("There was an error while spawning the manager");
}

async fn write_response_to_file(mut input_channel: Receiver<&str>) {
    let mut file =
        File::create("data.dat").expect("There was an error while creating the output file");

    while let Some(data) = input_channel.recv().await {
        println!("There is some new data {}", &data);
        file.write_all(data.as_bytes())
            .expect("There was an error while writing new data to the output file");
    }
}
