# Instructions to finetune InternVL-2.5-4B

## Download Raw Data

```
cargo run --release -- --start-date 2014-01-01 --end-date 2021-01-31 --output tm_data_jan_2020.json --download-images --images-dir tm_images
```


## Cleanup and preprocess Raw data

```
cd python
python extract_clean_data.py
```

## Preapare Training Data

check ```python/prepare_training_data.ipynb``` for details


## Organize the training data


```
dset
--| prepared_data.jsonl
--| imgs
----| 40201400001U_ead6e817-8028-4ea7-a620-52d827068638.jpg
----| 40201622404S_103e48d4-b5dc-48f6-8852-a67bea12e44e.jpg
...
```

## Finetune InternVL-2.5-4B


```
git clone https://github.com/zhoubin-me/InternVL.git
cd internVL
pip install -r requirements.txt
```

You may face quite a lot of issues to be manually fixed during installation.

Now change the path of data in ```internvl_chat/shell/data/trademark.json``` to where you organize the data


Make sure you have 2x 4090 like GPUs to finetune:
```
cd internvl_chat
GPUS=2 PER_DEVICE_BATCH_SIZE=1 sh shell/internvl2.5/2nd_finetune/internvl2_5_4b_dynamic_res_2nd_finetune_lora_custom.sh
```
It takes around 12 hours to finish finetuning on 2x 4090 GPU.

Then merge the weights:
```
python tools/merge_lora.py work_dirs/internvl_chat_v2_5/internvl2_5_4b_dynamic_res_2nd_finetune_lora work_dirs/internvl_chat_v2_5/internvl2_5_4b_dynamic_res_2nd_finetune_lora_merge
```


You may now push the weights under ```internvl2_5_4b_dynamic_res_2nd_finetune_lora_merge``` to your own huggingface repo to use.





