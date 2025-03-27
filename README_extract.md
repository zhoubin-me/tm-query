# Extract with LLM

This Rust implementation extracts text from images using an LLM model API.

## Prerequisites

Before running this program, make sure you have:

1. Rust and Cargo installed
2. The `dset/cleaned_data.json` dataset file
3. Image files in `dset/imgs/` directory
4. Access to an LLM API service that supports image understanding

## Configuration

The program is set to use a local LLM API service at `http://0.0.0.0:23333/v1`. You should modify the following variables in the code if needed:

- `api_key`: Your API key
- `base_url`: The base URL of your LLM API service

## Usage

```bash
# Set the RUST_LOG environment variable to control log level
export RUST_LOG=info

# Run the program
cargo run --bin extract_with_llm
```

## Output

The program will:

1. Process up to 2000 images from the dataset
2. Extract text using the LLM
3. Compare the extracted text with the expected text
4. Generate a log file in the `logs/` directory with the results

## OpenCC Configuration

The program uses OpenCC with the `t2s.json` configuration for Traditional to Simplified Chinese conversion. Make sure you have the appropriate OpenCC configuration files installed on your system.