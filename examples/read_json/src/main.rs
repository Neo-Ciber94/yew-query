use std::{convert::Infallible, io::ErrorKind, path::Path, time::Duration};
use serde::{de::DeserializeOwned, Deserialize};
use tokio::task::LocalSet;
use yew_query_core::{QueryClient, QueryKey};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Color {
    id: usize,
    color: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ColorData {
    data: Vec<Color>,
}

#[tokio::main]
async fn main() {
    let mut client = QueryClient::builder()
        .cache_time(Duration::from_secs(5))
        .refetch_time(Duration::from_secs(2))
        .build();

    let local_set = LocalSet::new();

    loop {
        let key = QueryKey::of::<ColorData>("colors");
        let ret = {
            local_set
                .run_until(async { client.fetch_query(key, read_colors).await })
                .await
        };

        tokio::time::sleep(Duration::from_secs(1)).await;

        match ret {
            Ok(x) => {
                println!("{:#?}", x);
            }
            Err(err) => {
                eprintln!("{:?}", err);
            }
        }
    }
}

async fn read_colors() -> Result<ColorData, Infallible> {
    tokio::time::sleep(Duration::from_secs(3)).await;
    let data = read_file_as_json::<ColorData>("./data.json").await.unwrap();
    Ok(data)
}

async fn read_file_as_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> std::io::Result<T> {
    let json = tokio::fs::read_to_string(path).await?;
    let value = serde_json::from_str::<T>(json.as_str())
        .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?;

    Ok(value)
}
