# Domain model

## SkinPack (local)

- source image
- processed PNG (square, power-of-two)
- `manifest.txt` (Version 3, ItemType, empty Textures)
- preview.png
- meta.json (title, description, tags)

## WorkshopItem (Steam)

- `PublishedFileId` / SkinID
- tags: Version3, Skin
- preview image (inventory icon source)
- content folder (manifest + icon)

## Project (SQLite)

- id, title, description, item_type
- asset_dir
- published_file_id?
- timestamps

## PublishHistory

- project_id, published_file_id, url
- dry_run, success, message, created_at

## Association to Supply Signal

Not a first-class Facepunch binding. Consumers apply:

```text
shortname = supply.signal
skin = PublishedFileId
```

via Oxide plugins / kits / custom items.
