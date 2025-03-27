from openai import OpenAI
import base64
import re
import json
import random
from tqdm import tqdm
import opencc
import logging
import os
from datetime import datetime

# Configure logging
def setup_logging():
    # Create logs directory if it doesn't exist
    if not os.path.exists('logs'):
        os.makedirs('logs')

    # Set up timestamp for log filename
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    log_filename = f'logs/extraction_{timestamp}.log'

    # Configure root logger
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(levelname)s - %(message)s',
        handlers=[
            logging.FileHandler(log_filename),
            logging.StreamHandler()  # Also output to console
        ]
    )

    logging.info(f"Logging configured. Log file: {log_filename}")
    return log_filename

def encode_image(image_path):
    with open(image_path, "rb") as image_file:
        return base64.b64encode(image_file.read()).decode("utf-8")

def strip_to_non_repeating(s: str) -> str:
    """Return the smallest non-repeating substring of a repeating string."""
    n = len(s)
    for i in range(1, n // 2 + 1):  # Check possible substring lengths
        if n % i == 0:  # The substring length must evenly divide the full length
            if s[:i] * (n // i) == s:
                return s[:i]
    return s  # Return original string if it's not repeating

def main():
    converter = opencc.OpenCC('t2s.json')

    # Setup logging
    log_filename = setup_logging()

    logging.info("Starting extraction process")

    # Initialize OpenAI client
    api_key = 'YOUR_API_KEY'
    logging.info(f"Initializing OpenAI client with base URL: http://0.0.0.0:23333/v1")
    client = OpenAI(api_key=api_key, base_url='http://0.0.0.0:23333/v1')

    # Get model information
    model_name = client.models.list().data[0].id
    logging.info(f"Using model: {model_name}")

    # Load dataset
    logging.info("Loading dataset from 'dset/cleaned_data.json'")
    with open('dset/cleaned_data.json', 'r') as f:
        data = json.load(f)

    # Set processing parameters
    total = 2000
    logging.info(f"Processing {total} images from dataset")

    count = 0
    correct = 0

    for idx, entry in tqdm(enumerate(data[:total])):
        image_path = f"./dset/imgs/{entry['imageName']}"

        # Skip if file doesn't exist
        if not os.path.exists(image_path):
            logging.warning(f"Image not found: {image_path}")

        # Get Chinese character and skip if None
        chinese_chars = entry['chineseCharacter']
        if chinese_chars is not None:
            chinese_chars = converter.convert(entry['chineseCharacter']).strip()

        # Process image
        try:
            base64_image = encode_image(image_path)
            logging.debug(f"Processing image: {entry['imageName']}")

            prompt = "Extract all readable text from the provided image while preserving formatting, punctuation, and line breaks as accurately as possible."
            # Make API call
            response = client.chat.completions.create(
                model=model_name,
                messages=[{
                    'role': 'user',
                    'content': [{
                        'type': 'text',
                        'text': prompt,
                    }, {
                        'type': 'image_url',
                        'image_url': {
                            'url': f"data:image/jpeg;base64,{base64_image}",
                        },
                    }],
                }],
                temperature=0.8,
                max_tokens=128,
                top_p=0.8)

            # Process response
            answer = response.choices[0].message.content
            chinese_text = re.findall(r'[\u4e00-\u9fff]+', answer)


            if len(chinese_text) == 0:
                chinese_text = None
                logging.warning(f"No Chinese text extracted from image {entry['imageName']}")
            else:
                chinese_text = converter.convert("".join(chinese_text))
                chinese_text = strip_to_non_repeating(chinese_text)

            count += 1

            # Compare results
            if chinese_text == chinese_chars:
                logging.info(f"[{idx:6d}/{total}] ✓ Match: {chinese_chars}")
                correct += 1
            else:
                logging.warning(f"[{idx:6d}/{total}] ✗ Mismatch - Extracted: '{chinese_text}', Original: '{chinese_chars}', File: {entry['imageName']}")

        except Exception as e:
            logging.error(f"Error processing {image_path}: {str(e)}")

    # Log summary statistics
    accuracy = (correct / count) * 100 if count > 0 else 0
    logging.info(f"Processing complete. Summary:")
    logging.info(f"Total processed: {count}")
    logging.info(f"Correct matches: {correct}")
    logging.info(f"Accuracy: {accuracy:.2f}%")
    logging.info(f"Log file saved to: {log_filename}")


if __name__ == '__main__':
    main()
