use std::{fs::File, io::Write};

use tokio::sync::mpsc::{self, Receiver, Sender};

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel(1);

    let file_task = tokio::spawn(async move {
        write_response_to_file(rx).await;
    });

    let request_task = tokio::spawn(async move {
        request("https://fa.wikipedia.org/", tx).await;
    });

    file_task
        .await
        .expect("There was an error while spawning the file manager task");

    request_task
        .await
        .expect("There was an error while starting the request task");
}

async fn request(url: &str, file_channel: Sender<String>) {
    let response = reqwest::get(url)
        .await
        .expect("There was an error while requesting for the url")
        .text()
        .await
        .expect("There was an error while readint the text from the result returned form the url");

    file_channel
        .send(response)
        .await
        .expect("There was an error while writing the response text to the channel");
}

async fn write_response_to_file(mut input_channel: Receiver<String>) {
    let mut file =
        File::create("data.txt").expect("There was an error while creating the output file");

    while let Some(data) = input_channel.recv().await {
        println!("There is some new data {}", &data);
        file.write_all(data.as_bytes())
            .expect("There was an error while writing new data to the output file");
    }
}
