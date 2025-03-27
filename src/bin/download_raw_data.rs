use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate};
use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start date in YYYY-MM-DD format
    #[arg(short, long)]
    start_date: String,

    /// End date in YYYY-MM-DD format
    #[arg(short, long)]
    end_date: String,

    /// Output file path
    #[arg(short, long, default_value = "trademark_data.json")]
    output: PathBuf,

    /// Number of days per chunk
    #[arg(short, long, default_value_t = 1)]
    chunk_size: u64,

    /// Maximum concurrent requests
    #[arg(short = 'p', long, default_value_t = 30)]
    concurrency: usize,

    /// Download trademark images
    #[arg(short, long)]
    download_images: bool,

    /// Directory to save images (defaults to ./images)
    #[arg(long, default_value = "images")]
    images_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ApiResponse {
    lodgement_date: String,
    count: u32,
    items: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Document {
    #[serde(rename = "fileName")]
    file_name: String,

    #[serde(rename = "lodgementDate")]
    lodgement_date: String,

    #[serde(rename = "docType")]
    doc_type: DocumentType,

    #[serde(rename = "fileId")]
    file_id: String,

    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DocumentType {
    description: String,
    code: String,
}

async fn download_image(
    client: &Client,
    url: &str,
    app_num: &str,
    file_name: &str,
    dir: &Path
) -> Result<PathBuf> {
    // Path for the image file
    let img_path = dir.join(format!("{}_{}", app_num, file_name));

    // Check if file already exists
    if img_path.exists() {
        return Ok(img_path);
    }

    // Download the image
    let response = client.get(url).send().await.context("Failed to download image")?;
    let bytes = response.bytes().await.context("Failed to read image bytes")?;

    // Save the image to file
    fs::write(&img_path, bytes).context("Failed to save image file")?;

    Ok(img_path)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let start_date = NaiveDate::parse_from_str(&args.start_date, "%Y-%m-%d")
        .context("Failed to parse start date")?;
    let end_date = NaiveDate::parse_from_str(&args.end_date, "%Y-%m-%d")
        .context("Failed to parse end date")?;

    if start_date > end_date {
        anyhow::bail!("Start date must be before or equal to end date");
    }

    println!("Fetching trademark data from {} to {}", start_date, end_date);
    println!("Using chunk size of {} day(s)", args.chunk_size);
    println!("Maximum concurrent requests: {}", args.concurrency);

    if args.download_images {
        println!("Will download trademark images to {}", args.images_dir.display());
        fs::create_dir_all(&args.images_dir).context("Failed to create images directory")?;
    }

    let client = Client::new();
    let mut all_data: HashMap<String, ApiResponse> = HashMap::new();

    // Generate all dates to fetch
    let mut dates = Vec::new();
    let mut current_date = start_date;

    while current_date <= end_date {
        dates.push(current_date);
        current_date += Duration::days(args.chunk_size as i64);
    }

    // Process in batches to control concurrency
    let total_dates = dates.len();
    for (i, chunk) in dates.chunks(args.concurrency).enumerate() {
        let mut tasks = Vec::new();

        for &date in chunk {
            let date_str = date.format("%Y-%m-%d").to_string();
            let client = client.clone();

            tasks.push(tokio::spawn(async move {
                let url = format!(
                    "https://api.data.gov.sg/v1/technology/ipos/trademarks?lodgement_date={}",
                    date_str
                );

                println!("Fetching data for date: {}", date_str);

                match client.get(&url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<ApiResponse>().await {
                                Ok(api_response) => {
                                    println!(
                                        "Successfully fetched {} trademarks for {}",
                                        api_response.count, date_str
                                    );
                                    Ok((date_str, api_response))
                                }
                                Err(e) => {
                                    eprintln!("Error parsing JSON for {}: {}", date_str, e);
                                    Err(format!("Error parsing JSON: {}", e))
                                }
                            }
                        } else {
                            eprintln!(
                                "Error fetching data for {}: HTTP status {}",
                                date_str,
                                response.status()
                            );
                            Err(format!("HTTP error: {}", response.status()))
                        }
                    }
                    Err(e) => {
                        eprintln!("Request error for {}: {}", date_str, e);
                        Err(format!("Request error: {}", e))
                    }
                }
            }));
        }

        // Process results from this batch
        for task in tasks {
            if let Ok(result) = task.await {
                if let Ok((date, response)) = result {
                    all_data.insert(date, response);
                }
            }
        }

        println!("Completed batch {}/{} ({:.1}%)",
            i + 1,
            (total_dates + args.concurrency - 1) / args.concurrency,
            (i + 1) as f64 * 100.0 / ((total_dates + args.concurrency - 1) / args.concurrency) as f64
        );

        // Add delay between batches to avoid rate limiting
        sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Save all data to output file
    println!("Saving data to {}", args.output.display());
    let file = File::create(&args.output).context("Failed to create output file")?;
    let writer = BufWriter::new(file);

    // For easier analysis, transform data structure from map to array of objects with date field
    let transformed_data: Vec<_> = all_data.iter()
        .map(|(date, response)| {
            json!({
                "date": date,
                "count": response.count,
                "items": response.items
            })
        })
        .collect();


    serde_json::to_writer_pretty(writer, &transformed_data).context("Failed to write output file")?;

    println!("Successfully saved trademark data to {}", args.output.display());

    // Download images if requested
    if args.download_images && !all_data.is_empty() {
        println!("Downloading trademark images...");
        let mut download_tasks = Vec::new();
        let mut total_tasks = 0;

        // Collect all download tasks
        for (_, api_response) in all_data.iter() {
            for item in &api_response.items {
                if let Some(documents) = item.get("documents").and_then(|d| d.as_array()) {
                    if let Some(app_num) = item.get("applicationNum").and_then(|a| a.as_str()) {
                        for doc in documents {
                            if let (Some(url), Some(file_name)) = (doc.get("url").and_then(|u| u.as_str()),
                                                                 doc.get("fileName").and_then(|f| f.as_str())) {
                                // Store the task information
                                download_tasks.push((url.to_string(), app_num.to_string(), file_name.to_string()));
                                total_tasks += 1;
                            }
                        }
                    }
                }
            }
        }

        println!("Found {} images to download", total_tasks);
        let mut downloaded_count = 0;

        // Process in batches to control concurrency
        for (batch_idx, chunk) in download_tasks.chunks(args.concurrency).enumerate() {
            let mut tasks: Vec<tokio::task::JoinHandle<bool>> = Vec::new();

            for (url, app_num, file_name) in chunk {
                let client = client.clone();
                let url = url.clone();
                let app_num = app_num.clone();
                let file_name = file_name.clone();
                let images_dir = args.images_dir.clone();

                tasks.push(tokio::spawn(async move {
                    let result = match download_image(&client, &url, &app_num, &file_name, &images_dir).await {
                        Ok(_) => true,
                        Err(e) => {
                            eprintln!("Failed to download image {}: {}", url, e);
                            false
                        }
                    };
                    result
                }));
            }

            // Process results from this batch
            for task in tasks {
                if let Ok(result) = task.await {
                    if result {
                        downloaded_count += 1;
                    }
                }
            }

            println!("Completed batch {}/{} - Downloaded {}/{} images ({:.1}%)",
                batch_idx + 1,
                (total_tasks + args.concurrency - 1) / args.concurrency,
                downloaded_count,
                total_tasks,
                downloaded_count as f64 * 100.0 / total_tasks as f64
            );

            // Add delay between batches to avoid rate limiting
            sleep(tokio::time::Duration::from_millis(500)).await;
        }

        println!("Downloaded {}/{} images", downloaded_count, total_tasks);
    }

    Ok(())
}
