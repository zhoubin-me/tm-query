#!/usr/bin/env python3

import os
import base64
import json
import requests
import logging
from pathlib import Path
import glob
from datetime import datetime
from PIL import Image

def setup_logger():
    # Create logs directory if it doesn't exist
    os.makedirs('logs', exist_ok=True)

    # Set up logging with timestamp in filename
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    log_file = f"logs/query_samples_{timestamp}.log"

    # Configure logger
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(levelname)s - %(message)s',
        handlers=[
            logging.FileHandler(log_file),
            logging.StreamHandler()  # Also output to console
        ]
    )
    logging.info(f"Logging initialized. Log file: {log_file}")
    return logging.getLogger()

def main():
    # Set up logger
    logger = setup_logger()

    # Directory containing the images
    try:
        with open('dset/cleaned_data.json', 'r') as f:
            data = json.load(f)
        logger.info(f"Loaded {len(data)} entries from cleaned_data.json")
    except Exception as e:
        logger.error(f"Failed to load data: {str(e)}")
        return

    url = "http://0.0.0.0:1234/invoke"
    logger.info(f"Using API endpoint: {url}")

    processed_count = 0
    failed_count = 0

    for entry in data[:1000]:
        image_name = entry['imageName']
        img_path = os.path.join('dset/imgs', image_name)
        if not os.path.exists(img_path):
            logger.warning(f"Image not found: {img_path}")
            failed_count += 1
            continue

        try:
            img = Image.open(img_path)
            img.verify()
        except Exception as e:
            logger.warning(f"Image is not valid: {img_path}")
            failed_count += 1
            continue

        if entry['chineseCharacter'] is None:
            logger.warning(f"Chinese character is None: {image_name}")
            failed_count += 1
            continue

        try:
            logger.info(f"Processing image: {image_name}")

            # Read the image file
            with open(img_path, "rb") as img_file:
                img_data = img_file.read()

            # Encode image in base64
            base64_encoded = base64.b64encode(img_data).decode('utf-8')

            # Prepare the request payload
            payload = {"image": base64_encoded}

            # Send the POST request
            headers = {"Content-Type": "application/json"}
            logger.debug(f"Sending request to {url}")

            response = requests.post(url, headers=headers, data=json.dumps(payload))

            # Log the response
            logger.info(f"Response status code: {response.status_code}")

            if response.ok:
                response_data = response.json()
                logger.info(f"Successful response for {image_name}")
                original_chinese_character = entry['chineseCharacter']
                if original_chinese_character is None:
                    original_chinese_character = ""
                info = f"Response content: Chinese characters: {response_data['chineseCharacter']}, English words: {response_data['wordsInMark']}, Description: {response_data['descrOfDevice']}, Original: {original_chinese_character}, image_name: {image_name}"
                logger.info(info)
                processed_count += 1
            else:
                logger.error(f"Failed request for {image_name}: {response.text}")
                failed_count += 1

        except Exception as e:
            logger.error(f"Error processing {image_name}: {str(e)}", exc_info=True)
            failed_count += 1

    logger.info(f"Processing complete. Successfully processed: {processed_count}, Failed: {failed_count}")

if __name__ == "__main__":
    main()
