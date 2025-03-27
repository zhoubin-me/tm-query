import json
import os

with open('dset/raw_data.json', 'r') as f:
    data = json.load(f)

invalids = []
entries = []
for date in data:
    for item in date['items']:
        index = item['markIndex']
        imgs = item['documents']
        app_num = item['applicationNum']
        if index is not None and app_num is not None and len(imgs) == 1 and len(index) == 1:
            index = index[0]
            url = imgs[0]['url']
            img_name = f"{app_num}_{os.path.basename(url)}"
            index['imageName'] = img_name
            entries.append(index)
        else:
            invalids.append((app_num, index, imgs))


print(len(invalids), len(entries))
for i in range(0, 4):
    print(i, len([x for x in invalids if len(x[2]) == i]))

desc = [x for x in entries if x['descrOfDevice'] is not None]
print(len(desc))
words = [len(x['descrOfDevice'].strip().split(' ')) for x in desc]

for i in range(1, 20):
    print(i, len([x for x in words if x == i]))
print(max(words))

entries_cleaned = []
for x in entries:
    keys = ['descrOfDevice', 'chineseCharacter', 'wordsInMark']
    is_all_none = all([x[k] is None for k in keys])
    is_desc_too_long = False
    if x['descrOfDevice'] is not None:
        num_words = len(x['descrOfDevice'].strip().split(' '))
        is_desc_too_long = num_words > 10
    img_name = x['imageName']
    if not os.path.exists(f"dset/imgs/{img_name}"):
        continue
    if not is_all_none and not is_desc_too_long:
        entries_cleaned.append(x)


with open('dset/cleaned_data.json', 'w') as f:
    json.dump(entries_cleaned, f, indent=4)

print(len(entries_cleaned))