use scraper::{Html, Selector};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::mpsc::{self, Receiver, Sender},
};

#[tokio::main]
async fn main() {
    let spider_task = tokio::spawn(async move {
        spider("https://fa.wikipedia.org/").await;
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
    let reader_task = tokio::spawn(async move {
        while let Some(url) = rx.recv().await {
            println!("new url: {}", url);
        }
    });

    let writer_task = tokio::spawn(async move {
        loop {
            tx.send("https://fa.wikipedia.org/".to_owned())
                .await
                .expect("There was an error while writing the urlto the channel");
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
        .map(|x| x.value().attr("href").unwrap().to_owned())
        .collect()
}

async fn spider(base_url: &str) {
    let (file_tx, file_rx) = mpsc::channel(1);
    let (read_url_tx, mut read_url_rx) = mpsc::channel(1);
    let (write_url_tx, write_url_rx) = mpsc::channel(1);

    let file_manager_task = tokio::spawn(async move { file_manager(file_rx).await });

    let url_manager_task =
        tokio::spawn(async move { url_manager(write_url_rx, read_url_tx).await });

    while let Some(url) = read_url_rx.recv().await {
        let response = request(url).await;
        let urls = process(&response);

        for url in urls {
            write_url_tx
                .send(url)
                .await
                .expect("There was an error while sending the url to the channel");
        }

        file_tx
            .send(response)
            .await
            .expect("There was ana error while sending the html to file writer task");
    }

    file_manager_task
        .await
        .expect("There was an error while spawning the file manager task");
    url_manager_task
        .await
        .expect("There was an error while spawning the url manager task");
}
