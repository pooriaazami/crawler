use std::{collections::HashSet, fs::File, io::Write, sync::Arc};

use scraper::{Html, Selector};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
};

#[tokio::main]
async fn main() {
    crawl("http://yazd.ac.ir/").await;
}

async fn request(url: &str) -> String {
    println!("Downloading {url}");

    reqwest::get(url)
        .await
        .expect("There was an error while sending a request")
        .text()
        .await
        .expect("There was an error while reading the html of the request")
}

async fn crawl(url: &str) {
    let (to_thread_from_pool, mut from_pool_to_thread) = mpsc::channel(32);
    let (to_pool_from_thread, from_thread_to_pool) = mpsc::channel(32);
    let (to_file_from_thread, from_thread_to_file) = mpsc::channel(32);

    let initial_request = to_pool_from_thread.clone();

    let file_writer_task = tokio::spawn(async move {
        write_to_file(from_thread_to_file).await;
    });

    let uniqueness_check_task = tokio::spawn(async move {
        uniquness_cheker(to_thread_from_pool, from_thread_to_pool).await;
    });

    initial_request
        .send(url.to_owned())
        .await
        .expect("There  as error while sending the first url to the pool");

    let thread_pool = Arc::new(Mutex::new(Vec::<JoinHandle<()>>::new()));

    let cloned_thread_pool = thread_pool.clone();
    let thread_pool_pruner = tokio::spawn(async move {
        loop {
            let mut locked_thread_pool = cloned_thread_pool.lock().await;

            let mut done_threads = Vec::new();
            for (i, t) in locked_thread_pool.iter().enumerate() {
                if t.is_finished() {
                    done_threads.push(i);
                }
            }

            for index in done_threads.iter() {
                locked_thread_pool.remove(*index);
            }
        }
    });
    // println!("Starting....");
    while let Some(url) = from_pool_to_thread.recv().await {
        // println!("Start working on {url}");
        let cloned_to_pool_from_thread = to_pool_from_thread.clone();
        let cloned_to_file_from_thread = to_file_from_thread.clone();
        let thread = tokio::spawn(async move {
            url_pipeline(url, cloned_to_pool_from_thread, cloned_to_file_from_thread).await;
        });
        let mut locked_thread_pool = thread_pool.lock().await;

        locked_thread_pool.push(thread);
    }

    thread_pool_pruner
        .await
        .expect("There was an error while waiting for the thread_pool_pruner to stop");

    uniqueness_check_task
        .await
        .expect("There was an error while waiting for the uniqueness_check_task to stop");

    file_writer_task
        .await
        .expect("There was an error while waiting for the file_writer_task to stop");
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

async fn url_pipeline(
    url: String,
    to_pool_from_thread: Sender<String>,
    to_file_from_thread: Sender<String>,
) {
    let response = request(&url).await;
    let domain = extract_domain(&url);

    let urls = process(&response);
    for new_url in urls {
        let new_url = if new_url.starts_with("/") {
            let url = format!("{}{}", domain, new_url);
            url
        } else {
            new_url
        };

        to_pool_from_thread
            .send(new_url)
            .await
            .expect("There was an error while sending the url to the pool");
    }

    to_file_from_thread
        .send(response)
        .await
        .expect("There was an error while send html to the channel to e wtite to the file");
}

async fn write_to_file(mut rx: Receiver<String>) {
    let mut file =
        File::create("output.txt").expect("There was an error while creating the output file");

    while let Some(html) = rx.recv().await {
        file.write_all(html.as_bytes())
            .expect("There was an error while wariting data to the output file");

        file.flush()
            .expect("There was an error while flushing the output file");
    }
}

async fn uniquness_cheker(tx: Sender<String>, mut rx: Receiver<String>) {
    let url_memory = Arc::new(Mutex::new(HashSet::new()));
    let url_queue = Arc::new(Mutex::new(Vec::new()));

    let read_url_memory = url_memory.clone();
    let read_url_queue = url_queue.clone();

    let read_task = tokio::spawn(async move {
        let mut locked_url_memory = read_url_memory.lock().await;
        // let mut locked_url_queue = read_url_queue.lock().await;

        while let Some(url) = rx.recv().await {
            if !locked_url_memory.contains(&url) {
                locked_url_memory.insert(url.clone());
                read_url_queue.lock().await.push(url);
            }
        }
    });

    let read_url_queue = url_queue.clone();

    let write_task = tokio::spawn(async move {
        loop {
            let mut locked_url_queue = read_url_queue.lock().await;
            if locked_url_queue.len() != 0 {
                tx.send(locked_url_queue.remove(0)).await.expect("There was an error while waiting for the channel to send url outside the url_pool");
            }
        }
    });

    read_task
        .await
        .expect("There was an error while waiting for the read_task to stop");

    write_task
        .await
        .expect("There was an error while waiting for the write_task to stop");
}
