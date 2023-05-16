use tokio::{fs::File, io::AsyncWriteExt, sync::mpsc::Receiver};

#[tokio::main]
async fn main() {
    println!("Hello, World")
}

async fn write_responce_to_file(mut rx: Receiver<String>) {
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
