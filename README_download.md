# Trademark Data Fetcher

A Rust application to fetch trademark data from the Singapore Intellectual Property Office (IPOS) Data API. The application queries the API for trademarks filed within a specified date range and saves the data as a JSON file.

## Features

- Fetch trademarks by lodgement date range
- Process data in configurable chunks to avoid API rate limits
- Configurable concurrency for better performance
- Option to download and save trademark images
- Save data in a structured JSON format for easy analysis

## Installation

### Prerequisites

- Rust and Cargo (install from [rustup.rs](https://rustup.rs))

### Building

Clone the repository and build the application:

```bash
git clone <repository-url>
cd tm-query
cargo build --release
```

The executable will be available at `target/release/tm-query`.

## Usage

```bash
# Basic usage - retrieve data for a single day
cargo run -- --start-date 2020-01-01 --end-date 2020-01-01

# Retrieve data for a date range
cargo run -- --start-date 2020-01-01 --end-date 2020-01-31

# Retrieve data for a date range and download images
cargo run -- --start-date 2020-01-01 --end-date 2020-01-31 --download-images

# Specify output file and images directory
cargo run -- --start-date 2020-01-01 --end-date 2020-01-31 --output tm_data_jan_2020.json --download-images --images-dir tm_images

# Adjust chunk size (days per request) and concurrency (parallel requests)
cargo run -- --start-date 2020-01-01 --end-date 2020-01-31 --chunk-size 1 --concurrency 3
```

### Command Line Options

- `--start-date`, `-s`: Start date in YYYY-MM-DD format (required)
- `--end-date`, `-e`: End date in YYYY-MM-DD format (required)
- `--output`, `-o`: Output JSON file path (default: `trademark_data.json`)
- `--chunk-size`, `-c`: Number of days per chunk (default: 1)
- `--concurrency`: Maximum concurrent requests (default: 5)
- `--download-images`, `-d`: Download trademark images
- `--images-dir`: Directory to save images (default: `images`)

## Data Structure

The output JSON file contains an array of trademark data objects, each containing:

- `date`: The lodgement date
- `count`: Number of trademarks for that date
- `items`: Array of trademark details

Each trademark in the `items` array contains comprehensive information including:

- Application number
- Applicant details
- Goods and services specifications
- Trademark images/logos
- Registration status
- And more

## Image Downloads

When the `--download-images` flag is used, the application will:

1. Create a directory structure based on application numbers
2. Download all trademark images/logos
3. Save them with their original filenames
4. Skip files that have already been downloaded

## License

[MIT License](LICENSE)