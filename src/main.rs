use std::{collections::HashSet, sync::Arc};

use scraper::{Html, Selector};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
};

#[tokio::main]
async fn main() {
    let spider_task = tokio::spawn(async move {
        spider("https://fa.wikipedia.org/".to_owned()).await;
    });
    println!("Spawning the spider task");
    spider_task
        .await
        .expect("There was an error while spawing the spider");
}

async fn file_manager(mut rx: Receiver<String>) {
    let mut output_file = File::create("output.txt")
        .await
        .expect("There was an error while opening the output file");

    while let Some(html) = rx.recv().await {
        output_file
            .write_all(html.as_bytes())
            .await
            .expect("There was an error while writing to the output file");
    }
}

async fn url_manager(mut rx: Receiver<String>, tx: Sender<String>) {
    let all_urls = Arc::new(Mutex::new(HashSet::new()));
    let queue = Arc::new(Mutex::new(Vec::new()));

    let cloned_all_urls = all_urls.clone();
    let cloned_queue = queue.clone();
    let reader_task = tokio::spawn(async move {
        println!("Restating the reader task");
        while let Some(url) = rx.recv().await {
            if url.starts_with("https://") {
                let mut locked_all_urls = cloned_all_urls.lock().await;

                if !locked_all_urls.contains(&url) {
                    locked_all_urls.insert(url.clone());
                    let mut locked_queue = cloned_queue.lock().await;
                    locked_queue.push(url);
                } else {
                    println!("I have already seen this url");
                }
            }
        }
    });

    let writer_task = tokio::spawn(async move {
        loop {
            let mut locked_queue = queue.lock().await;

            if locked_queue.len() != 0 {
                let url = locked_queue.remove(0);
                tx //.blocking_send(url)
                    .send(url)
                    .await
                    .expect("There was an error while writing the url to the channel");
            }
        }
    });

    reader_task
        .await
        .expect("There was an error while spawning the reader task");
    writer_task
        .await
        .expect("There was an error while spawning the writer task");
}

async fn request(url: String) -> String {
    reqwest::get(url)
        .await
        .expect("There was an error while waiting for the response")
        .text()
        .await
        .expect("There was an error while extracting the text")
}

fn process(html: &str) -> Vec<String> {
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a").expect("There was an error while parsing the selector");
    document
        .select(&selector)
        .map(|x| x.value().attr("href"))
        .filter_map(|x| x)
        .map(|x| x.to_owned())
        .collect()
}

fn extract_domain(url: &str) -> String {
    let mut slash_pos = Vec::new();
    for (i, v) in url.chars().enumerate() {
        if v == '/' {
            slash_pos.push(i);
        }
    }

    if slash_pos.len() >= 3 {
        String::from(&url[0..slash_pos[2]])
    } else {
        String::from("/")
    }
}

async fn spider(base_url: String) {
    let count_inputs_to_the_channel = 32;
    let (file_tx, file_rx) = mpsc::channel(1);
    let (read_url_tx, mut read_url_rx) = mpsc::channel(1);
    let (write_url_tx, write_url_rx) = mpsc::channel(count_inputs_to_the_channel);

    let file_manager_task = tokio::spawn(async move { file_manager(file_rx).await });

    let write_url_tx_starter = write_url_tx.clone();
    let url_manager_task = tokio::spawn(async move {
        write_url_tx_starter
            .clone()
            .send(base_url)
            .await
            .expect("Theer was an error while sending base_url to the channel");

        url_manager(write_url_rx, read_url_tx).await;
    });

    while let Some(url) = read_url_rx.recv().await {
        println!("Crawling {}", &url);

        let domain = extract_domain(&url);

        let response = request(url).await;
        let urls = process(&response);

        for new_url in urls {
            let new_url = if new_url.starts_with("/") {
                let url = format!("{}{}", domain, new_url);
                url
            } else {
                new_url
            };

            let cloned_write_url_tx = write_url_tx.clone();
            tokio::spawn(async move {
                cloned_write_url_tx
                    .clone()
                    .send(new_url)
                    .await
                    .expect("There was an error while sending the url to the channel");
            });
        }

        file_tx
            .send(response)
            .await
            .expect("There was ana error while sending the html to file writer task");
    }

    url_manager_task
        .await
        .expect("There was an error while spawning the url manager task");

    file_manager_task
        .await
        .expect("There was an error while spawning the file manager task");
}
