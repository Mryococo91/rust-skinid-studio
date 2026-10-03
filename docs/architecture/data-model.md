# Data model

## Workshop content folder

```text
asset/
├── preview.png          # SetItemPreview
├── meta.json            # local only
└── content/
    ├── manifest.txt     # Version3 icon-only JSON
    └── icon.png         # optional copy inside content
```

## manifest.txt

```json
{
  "Version": 3,
  "ItemType": "Grenade",
  "Groups": [
    {
      "Textures": {},
      "Floats": {},
      "Colors": {}
    }
  ],
  "PublishedFileId": "9000000001"
}
```

`PublishedFileId` is local bookkeeping written after publish.

## SQLite (`studio.db`)

### projects

| column | type |
|--------|------|
| id | TEXT PK |
| title | TEXT |
| description | TEXT |
| item_type | TEXT |
| asset_dir | TEXT |
| published_file_id | TEXT NULL |
| created_at | TEXT RFC3339 |
| updated_at | TEXT RFC3339 |

### publish_history

| column | type |
|--------|------|
| id | TEXT PK |
| project_id | TEXT |
| published_file_id | TEXT |
| url | TEXT |
| dry_run | INTEGER |
| success | INTEGER |
| message | TEXT |
| created_at | TEXT RFC3339 |
