use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use futures::future::join_all;
use log::{warn, error, debug};
use opencc_rust::OpenCC;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task;
use rand::{rngs::StdRng, SeedableRng};
use rand::seq::SliceRandom;

// Structure for the dataset entries
#[derive(Debug, Deserialize)]
struct DatasetEntry {
    #[serde(rename = "imageName")]
    image_name: String,
    #[serde(rename = "chineseCharacter")]
    chinese_character: Option<String>,
}

// Structure for the new API response
#[derive(Debug, Deserialize)]
struct ApiResponse {
    #[serde(rename = "wordsInMark")]
    words_in_mark: Option<String>,
    #[serde(rename = "chineseCharacter")]
    chinese_character: Option<String>,
    #[serde(rename = "descrOfDevice")]
    description_of_device: Option<String>,
}

// Setup logging
fn setup_logging() -> (PathBuf, Arc<Mutex<File>>) {
    // Create logs directory if it doesn't exist
    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(logs_dir).expect("Failed to create logs directory");
    }

    // Set up timestamp for log filename
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let log_filename = logs_dir.join(format!("extraction_{}.log", timestamp));

    // Create log file for print statements
    let print_log_filename = logs_dir.join(format!("print_output_{}.txt", timestamp));

    // Create the log file for stdout redirection
    let file = File::create(&print_log_filename).expect("Failed to create print log file");
    let file_mutex = Arc::new(Mutex::new(file));

    // Initialize the logger
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .format_target(false)
        .init();

    println!("Logging configured. Log file: {:?}", log_filename);
    println!("Print output will be saved to: {:?}", print_log_filename);

    // Also write to the log file
    if let Ok(mut file) = file_mutex.lock() {
        let _ = writeln!(file, "Logging configured. Log file: {:?}", log_filename);
        let _ = writeln!(file, "Print output will be saved to: {:?}", print_log_filename);
    }

    (log_filename, file_mutex)
}

// Helper function to log both to console and file
fn log_to_both(log_file: &Arc<Mutex<File>>, message: &str) {
    println!("{}", message);
    if let Ok(mut file) = log_file.lock() {
        let _ = writeln!(file, "{}", message);
    }
}

// Encode image to base64
fn encode_image(image_path: &Path) -> Result<String> {
    let mut file = File::open(image_path)
        .with_context(|| format!("Failed to open image file: {:?}", image_path))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read image file: {:?}", image_path))?;

    Ok(general_purpose::STANDARD.encode(&buffer))
}

#[tokio::main]
async fn main() -> Result<()> {

    // Setup logging
    let (_, log_file) = setup_logging();

    log_to_both(&log_file, "Starting extraction process");

    // Initialize HTTP client
    let api_key = ""; // No longer needed but keeping for compatibility
    let base_url = "http://localhost:1234"; // Updated to the new API URL
    log_to_both(&log_file, &format!("Initializing API client with base URL: {}", base_url));

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    // We no longer need to get model information since we're using a simple REST API
    let model_name = "local-api"; // Just a placeholder value

    log_to_both(&log_file, &format!("Using API endpoint: {}/invoke", base_url));

    // Load dataset
    log_to_both(&log_file, "Loading dataset from 'python/dset/cleaned_data.json'");
    let data_file = fs::read_to_string("python/dset/cleaned_data.json")
        .context("Failed to read dataset file")?;

    let data: Vec<DatasetEntry> = serde_json::from_str(&data_file)
        .context("Failed to parse dataset JSON")?;

    // Uncomment these lines once the rand crate is built
    let seed: u64 = 42; // You can change the seed value as needed
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut shuffled_data: Vec<_> = data.iter().collect();
    shuffled_data.shuffle(&mut rng);

    // Set processing parameters
    let total = 10000.min(data.len());
    log_to_both(&log_file, &format!("Processing {} images from dataset", total));


    // Create shared resources
    let client = Arc::new(client);
    let log_file = Arc::clone(&log_file);

    // Process images in chunks
    let chunk_size = 10;
    let data_to_process: Vec<_> = data.iter().take(total).collect();

    for (chunk_idx, chunk) in data_to_process.chunks(chunk_size).enumerate() {
        let mut tasks = Vec::new();

        for (idx_in_chunk, entry) in chunk.iter().enumerate() {
            let image_path = PathBuf::from("./python/dset/imgs").join(&entry.image_name);

            // Skip if file doesn't exist
            if !image_path.exists() {
                warn!("Image not found: {:?}", image_path);
                continue;
            }

            // Get Chinese character and skip if None
            let chinese_chars = &entry.chinese_character;
            if chinese_chars.is_none() {
                continue;
            }

            // Create clones of Arc resources for the task
            let client_clone = Arc::clone(&client);
            let model_name_clone = model_name.clone();
            let base_url_clone = base_url.to_string();
            let api_key_clone = api_key.to_string();
            let image_name_clone = entry.image_name.clone();
            let chinese_chars_clone = chinese_chars.clone();
            let image_path_clone = image_path.clone();
            let log_file_clone = Arc::clone(&log_file);

            // Calculate global index
            let global_idx = chunk_idx * chunk_size + idx_in_chunk;

            // Spawn a task for each image
            let task = task::spawn(async move {
                match process_image(
                    &client_clone,
                    &image_path_clone,
                    &model_name_clone,
                    &base_url_clone,
                    &api_key_clone,
                    global_idx,
                    total,
                    &image_name_clone,
                ).await {
                    Ok(api_response) => {
                        let message = format!(
                            "[{:6}/{:6}] API Response - Chinese character: '{}', Words in mark: '{}', Device: '{}', Original: '{}', File: {}",
                            global_idx,
                            total,
                            api_response.chinese_character.unwrap_or_else(|| "None".to_string()),
                            api_response.words_in_mark.unwrap_or_else(|| "None".to_string()),
                            api_response.description_of_device.unwrap_or_else(|| "None".to_string()),
                            chinese_chars_clone.unwrap_or_else(|| "None".to_string()),
                            image_name_clone
                        );
                        log_to_both(&log_file_clone, &message);
                    },
                    Err(e) => {
                        error!("Error processing {:?}: {}", image_path_clone, e);
                        if let Ok(mut file) = log_file_clone.lock() {
                            let _ = writeln!(file, "Error processing {:?}: {}", image_path_clone, e);
                        }
                    }
                }
            });

            tasks.push(task);
        }

        // Wait for all tasks in the current chunk to complete
        join_all(tasks).await;
    }



    Ok(())
}

async fn process_image(
    client: &Client,
    image_path: &Path,
    model_name: &str,
    base_url: &str,
    api_key: &str,
    _idx: usize,
    _total: usize,
    image_name: &str,
) -> Result<ApiResponse> {
    // Encode image to base64
    let base64_image = encode_image(image_path)?;
    debug!("Processing image: {}", image_name);

    // Prepare the new request for the RESTful API
    let request_url = format!("{}/invoke", base_url);

    // Create the request body with just the base64 encoded image
    let request_body = serde_json::json!({
        "image": base64_image
    });

    // Make API call to the new endpoint
    let response: ApiResponse = client
        .post(&request_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to send request to API")?
        .json()
        .await
        .context("Failed to parse API response")?;

    // Return the full API response
    Ok(response)
}
