---
name: siyuan-knowledge-master
description: "ZeroClaw's permanent Siyuan Notes integration. Full mastery of notebooks, documents, blocks, attributes, SQL, HTML widgets, super blocks, and dashboards. Use when: (1) creating or editing Siyuan notes/documents, (2) organizing knowledge base structure, (3) building project management boards, (4) creating dashboards and rich visual reports, (5) querying Siyuan data via SQL."
---

# Siyuan Knowledge Master

ZeroClaw's deep integration with Siyuan Notes — the self-hosted personal knowledge management system. This skill gives ZeroClaw complete mastery over Siyuan: creating notebook hierarchies, composing rich documents, embedding HTML widgets, querying blocks with SQL, and producing publication-quality reports that demonstrate the full power of the platform.

**Siyuan endpoint:** `http://localhost:6806` (Docker container, always available)
**Siyuan version:** 3.6.1
**Authentication:** No token required on localhost

---

## When to use

- User asks to create, update, or delete notes in Siyuan
- User wants to organize knowledge into notebooks and document trees
- User asks to build a project management structure or kanban board in Siyuan
- User wants a dashboard with live stats, status cards, or progress indicators
- User asks to search or query Siyuan content
- User asks for a beautifully formatted report document in Siyuan
- Any task involving reading or writing the knowledge base
- **Always** when the First Mission has not yet been completed (check for "ZeroClaw Brain" notebook)

---

## Required tools / APIs

- No external API required — Siyuan runs locally at `http://localhost:6806`
- `curl` for HTTP API calls (pre-installed)
- `jq` for JSON parsing (install: `sudo apt-get install -y jq`)
- All API calls use `POST` with `Content-Type: application/json`
- Response shape: `{"code": 0, "msg": "", "data": ...}` (code 0 = success)

---

## Skills

### 1. connection_check

Verify Siyuan is running and get version.

```bash
curl -s -X POST http://localhost:6806/api/system/version \
  -H "Content-Type: application/json" \
  -d '{}' | jq '.data'
# Returns: "3.6.1"

# List all open notebooks
curl -s -X POST http://localhost:6806/api/notebook/lsNotebooks \
  -H "Content-Type: application/json" \
  -d '{}' | jq '.data.notebooks[] | {id, name, icon}'
```

---

### 2. notebook_operations

Create, open, rename, and list notebooks.

```bash
# List all notebooks
curl -s -X POST http://localhost:6806/api/notebook/lsNotebooks \
  -H "Content-Type: application/json" -d '{}' | jq '.data.notebooks'

# Create a new notebook
RESULT=$(curl -s -X POST http://localhost:6806/api/notebook/createNotebook \
  -H "Content-Type: application/json" \
  -d '{"name": "🧠 ZeroClaw Brain"}')
NOTEBOOK_ID=$(echo $RESULT | jq -r '.data.notebook.id')
echo "Created notebook: $NOTEBOOK_ID"

# Open a notebook (required before writing docs to it)
curl -s -X POST http://localhost:6806/api/notebook/openNotebook \
  -H "Content-Type: application/json" \
  -d "{\"notebook\": \"$NOTEBOOK_ID\"}"

# Rename a notebook
curl -s -X POST http://localhost:6806/api/notebook/renameNotebook \
  -H "Content-Type: application/json" \
  -d "{\"notebook\": \"$NOTEBOOK_ID\", \"name\": \"🧠 ZeroClaw Brain v2\"}"

# Get notebook config
curl -s -X POST http://localhost:6806/api/notebook/getNotebookConf \
  -H "Content-Type: application/json" \
  -d "{\"notebook\": \"$NOTEBOOK_ID\"}" | jq '.data.conf'
```

**JavaScript:**
```javascript
const SIYUAN = "http://localhost:6806";

async function siyuan(endpoint, body = {}) {
  const res = await fetch(`${SIYUAN}/api/${endpoint}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const json = await res.json();
  if (json.code !== 0) throw new Error(`Siyuan API error: ${json.msg}`);
  return json.data;
}

// List notebooks
const { notebooks } = await siyuan("notebook/lsNotebooks");
console.log(notebooks.map(n => `${n.id}: ${n.name}`));

// Create notebook
const { notebook } = await siyuan("notebook/createNotebook", { name: "🧠 ZeroClaw Brain" });
await siyuan("notebook/openNotebook", { notebook: notebook.id });
```

---

### 3. document_operations

Create, rename, move, and delete documents.

```bash
# Create document with Markdown content
# path format: "/folder/document-name" (no extension)
DOC_ID=$(curl -s -X POST http://localhost:6806/api/filetree/createDocWithMd \
  -H "Content-Type: application/json" \
  -d "{
    \"notebook\": \"$NOTEBOOK_ID\",
    \"path\": \"/index\",
    \"markdown\": \"# Welcome to ZeroClaw Brain\n\nThis is the main index.\"
  }" | jq -r '.data')
echo "Created document ID: $DOC_ID"

# Create nested document (folder/file hierarchy)
curl -s -X POST http://localhost:6806/api/filetree/createDocWithMd \
  -H "Content-Type: application/json" \
  -d "{
    \"notebook\": \"$NOTEBOOK_ID\",
    \"path\": \"/projects/active-projects\",
    \"markdown\": \"# Active Projects\n\"
  }" | jq '.data'

# List documents in a path
curl -s -X POST http://localhost:6806/api/filetree/listDocsByPath \
  -H "Content-Type: application/json" \
  -d "{\"notebook\": \"$NOTEBOOK_ID\", \"path\": \"/\"}" \
  | jq '.data.files[] | {id, name, path}'

# Rename document
curl -s -X POST http://localhost:6806/api/filetree/renameDoc \
  -H "Content-Type: application/json" \
  -d "{\"notebook\": \"$NOTEBOOK_ID\", \"path\": \"/index\", \"title\": \"🏠 Main Index\"}"

# Move documents (toPath must be a folder path)
curl -s -X POST http://localhost:6806/api/filetree/moveDocs \
  -H "Content-Type: application/json" \
  -d "{
    \"fromPaths\": [\"/old-location/doc\"],
    \"toNotebook\": \"$NOTEBOOK_ID\",
    \"toPath\": \"/new-location\"
  }"

# Remove document permanently
curl -s -X POST http://localhost:6806/api/filetree/removeDoc \
  -H "Content-Type: application/json" \
  -d "{\"notebook\": \"$NOTEBOOK_ID\", \"path\": \"/to-delete\"}"

# Get human-readable path from block ID
curl -s -X POST http://localhost:6806/api/filetree/getHPathByID \
  -H "Content-Type: application/json" \
  -d "{\"id\": \"$DOC_ID\"}" | jq '.data'
```

---

### 4. block_operations

Insert, update, delete, and query blocks. **Blocks are the atomic units of Siyuan.**

```bash
# === INSERT BLOCKS ===

# Append block to end of document (most common operation)
curl -s -X POST http://localhost:6806/api/block/appendBlock \
  -H "Content-Type: application/json" \
  -d "{
    \"dataType\": \"markdown\",
    \"data\": \"## New Section\n\nThis is new content.\",
    \"parentID\": \"$DOC_ID\"
  }" | jq '.data[0].doOperations[0].id'

# Prepend block to start of document
curl -s -X POST http://localhost:6806/api/block/prependBlock \
  -H "Content-Type: application/json" \
  -d "{
    \"dataType\": \"markdown\",
    \"data\": \"**Status:** Active\",
    \"parentID\": \"$DOC_ID\"
  }"

# Insert block AFTER a specific block (previousID = the block to insert after)
curl -s -X POST http://localhost:6806/api/block/insertBlock \
  -H "Content-Type: application/json" \
  -d "{
    \"dataType\": \"markdown\",
    \"data\": \"This goes after the target block.\",
    \"previousID\": \"$PREVIOUS_BLOCK_ID\"
  }"

# Insert block BEFORE a specific block (nextID = the block to insert before)
curl -s -X POST http://localhost:6806/api/block/insertBlock \
  -H "Content-Type: application/json" \
  -d "{
    \"dataType\": \"markdown\",
    \"data\": \"This goes before the target block.\",
    \"nextID\": \"$NEXT_BLOCK_ID\"
  }"

# === UPDATE BLOCKS ===

# Update block content
curl -s -X POST http://localhost:6806/api/block/updateBlock \
  -H "Content-Type: application/json" \
  -d "{
    \"id\": \"$BLOCK_ID\",
    \"dataType\": \"markdown\",
    \"data\": \"## Updated Heading\n\nUpdated content here.\"
  }"

# === READ BLOCKS ===

# Get block's Kramdown source (best way to read block content)
curl -s -X POST http://localhost:6806/api/block/getBlockKramdown \
  -H "Content-Type: application/json" \
  -d "{\"id\": \"$BLOCK_ID\"}" | jq '.data.kramdown'

# Get child blocks of a document or container
curl -s -X POST http://localhost:6806/api/block/getChildBlocks \
  -H "Content-Type: application/json" \
  -d "{\"id\": \"$DOC_ID\"}" | jq '.data[] | {id, type, content}'

# Get block info (metadata)
curl -s -X POST http://localhost:6806/api/block/getBlockInfo \
  -H "Content-Type: application/json" \
  -d "{\"id\": \"$BLOCK_ID\"}" | jq '.data'

# Get block DOM (HTML representation)
curl -s -X POST http://localhost:6806/api/block/getBlockDOM \
  -H "Content-Type: application/json" \
  -d "{\"id\": \"$BLOCK_ID\"}" | jq -r '.data.dom'

# === DELETE BLOCKS ===

# Delete a block
curl -s -X POST http://localhost:6806/api/block/deleteBlock \
  -H "Content-Type: application/json" \
  -d "{\"id\": \"$BLOCK_ID\"}"

# === MOVE BLOCKS ===

# Move block after another block
curl -s -X POST http://localhost:6806/api/block/moveBlock \
  -H "Content-Type: application/json" \
  -d "{
    \"id\": \"$BLOCK_TO_MOVE_ID\",
    \"previousID\": \"$TARGET_BLOCK_ID\"
  }"
```

**Response pattern for insertBlock/appendBlock/prependBlock:**
```json
{
  "code": 0,
  "data": [{
    "doOperations": [{"id": "20241001120000-newblkid", "action": "insert"}],
    "undoOperations": [...]
  }]
}
```
**Extract the new block ID:** `echo $RESULT | jq -r '.data[0].doOperations[0].id'`

---

### 5. block_attributes

Custom attributes let you tag, style, and link blocks semantically.

```bash
# Set custom attributes on a block
curl -s -X POST http://localhost:6806/api/attr/setBlockAttrs \
  -H "Content-Type: application/json" \
  -d "{
    \"id\": \"$BLOCK_ID\",
    \"attrs\": {
      \"name\": \"project-status\",
      \"alias\": \"current-status\",
      \"custom-status\": \"active\",
      \"custom-priority\": \"high\",
      \"custom-owner\": \"zeroclaw\",
      \"custom-tags\": \"project,active,2024\"
    }
  }"

# Get block attributes
curl -s -X POST http://localhost:6806/api/attr/getBlockAttrs \
  -H "Content-Type: application/json" \
  -d "{\"id\": \"$BLOCK_ID\"}" | jq '.data'

# Batch get attributes for multiple blocks
curl -s -X POST http://localhost:6806/api/attr/batchGetBlockAttrs \
  -H "Content-Type: application/json" \
  -d "{\"ids\": [\"$BLOCK_ID_1\", \"$BLOCK_ID_2\"]}" | jq '.data'
```

**Attribute naming conventions:**
- `name` — Named anchor (queryable by name)
- `alias` — Alternative name for the block
- `memo` — Memo/comment visible in UI
- `custom-*` — Custom attributes (prefix with `custom-` for user-defined)
- `bookmark` — Bookmark label

---

### 6. sql_queries

Query Siyuan's SQLite database for powerful block discovery and navigation.

```bash
# Basic SQL query
curl -s -X POST http://localhost:6806/api/query/sql \
  -H "Content-Type: application/json" \
  -d "{\"stmt\": \"SELECT id, content, type FROM blocks WHERE type = 'h' LIMIT 10\"}" \
  | jq '.data[] | {id, content, type}'

# Search for blocks containing text
curl -s -X POST http://localhost:6806/api/query/sql \
  -H "Content-Type: application/json" \
  -d "{\"stmt\": \"SELECT id, content, hpath FROM blocks WHERE content LIKE '%ZeroClaw%' LIMIT 20\"}" \
  | jq '.data'

# Find all documents in a notebook
curl -s -X POST http://localhost:6806/api/query/sql \
  -H "Content-Type: application/json" \
  -d "{\"stmt\": \"SELECT id, hpath, content FROM blocks WHERE type = 'd' AND box = '$NOTEBOOK_ID' ORDER BY hpath\"}" \
  | jq '.data'

# Find blocks with custom attributes
curl -s -X POST http://localhost:6806/api/query/sql \
  -H "Content-Type: application/json" \
  -d "{\"stmt\": \"SELECT b.id, b.content, a.value FROM blocks b JOIN attributes a ON b.id = a.block_id WHERE a.name = 'custom-status' AND a.value = 'active'\"}" \
  | jq '.data'

# Get recently updated blocks
curl -s -X POST http://localhost:6806/api/query/sql \
  -H "Content-Type: application/json" \
  -d "{\"stmt\": \"SELECT id, content, type, updated FROM blocks WHERE updated > '20241001000000' ORDER BY updated DESC LIMIT 20\"}" \
  | jq '.data'

# Count blocks by type in notebook
curl -s -X POST http://localhost:6806/api/query/sql \
  -H "Content-Type: application/json" \
  -d "{\"stmt\": \"SELECT type, COUNT(*) as count FROM blocks WHERE box = '$NOTEBOOK_ID' GROUP BY type ORDER BY count DESC\"}" \
  | jq '.data'
```

**`blocks` table schema:**
| Column | Description | Values |
|--------|-------------|--------|
| `id` | Block ID | `20241001120000-abcdefg` |
| `parent_id` | Parent block ID | — |
| `root_id` | Root document ID | — |
| `box` | Notebook ID | — |
| `path` | Document file path | `/folder/doc.sy` |
| `hpath` | Human-readable path | `/Folder/Document` |
| `name` | Block name attribute | — |
| `alias` | Block alias | — |
| `memo` | Block memo | — |
| `tag` | Tags | `#tag1 #tag2` |
| `content` | Plain text content | — |
| `markdown` | Markdown source | — |
| `type` | Block type | see below |
| `subtype` | Block subtype | — |
| `ial` | Inline attribute list | `id="..." custom-x="..."` |
| `created` | Creation timestamp | `20241001120000` |
| `updated` | Update timestamp | `20241001120000` |

**Block types (`type` column):**
| Type | Description |
|------|-------------|
| `d` | Document (root block) |
| `h` | Heading |
| `p` | Paragraph |
| `c` | Code block |
| `l` | List container |
| `i` | List item |
| `b` | Blockquote |
| `m` | Math block |
| `t` | Table |
| `hr` | Horizontal rule |
| `html` | Raw HTML block |
| `query_embed` | Query/embed block |
| `widget` | Widget block (iframe) |
| `iframe` | IFrame block |
| `s` | Superblock |

**Block subtypes:**
- Headings: `h1`, `h2`, `h3`, `h4`, `h5`, `h6`
- Lists: `o` (ordered), `u` (unordered), `t` (task)

---

### 7. siyuan_kramdown_syntax

Siyuan uses an extended Kramdown dialect. Master these patterns.

```markdown
# Heading 1
## Heading 2
### Heading 3

Normal paragraph text.

**Bold text**, *italic text*, ~~strikethrough~~, `inline code`

> Blockquote content
> Multiple lines

- Unordered list item
- Another item
  - Nested item

1. Ordered list item
2. Second item

- [ ] Task item (unchecked)
- [x] Task item (checked)

| Column A | Column B | Column C |
|----------|----------|----------|
| Value 1  | Value 2  | Value 3  |
| Data     | Data     | Data     |

```python
code block with language
```

$$
\LaTeX\ math\ formula
$$

$inline\ math$

---

**BLOCK REFERENCES** (link to any block in Siyuan):
((20241001120000-abcdefg))                    # Dynamic anchor (shows block content)
((20241001120000-abcdefg "Static anchor"))    # Static anchor text
((20241001120000-abcdefg 'Static anchor'))    # Alternative syntax

**BLOCK EMBED** (render another block inline):
!((20241001120000-abcdefg))

**TAGS:**
#project# #active# #zeroclaw#

**INLINE ATTRIBUTES** (at end of block):
{: id="20241001120000-abcdefg" custom-status="active" name="my-anchor"}

**NAMED ANCHOR** (for linking with [[name]]):
{: name="section-anchor"}

**SUPER BLOCKS** (multi-column layouts):
{{{row
Left column content

Goes here

---

Right column content

Also here
}}}

{{{col
Top section

---

Bottom section
}}}

**MATH BLOCK:**
$$
E = mc^2
$$

**HTML BLOCK** (raw HTML rendered in document):
<div style="background: #f0f0f0; padding: 16px; border-radius: 8px;">
  <strong>Note:</strong> This is a raw HTML block.
</div>

**IFRAME/WIDGET BLOCK:**
<iframe src="https://example.com" style="width:100%;height:400px;border:none;"></iframe>
```

---

### 8. super_block_layouts

Super blocks create flexible multi-column and multi-row layouts.

```bash
# Create a 2-column layout via API
curl -s -X POST http://localhost:6806/api/block/appendBlock \
  -H "Content-Type: application/json" \
  -d '{
    "dataType": "markdown",
    "data": "{{{row\n## 📊 Stats\n\n- **Documents:** 42\n- **Blocks:** 1,337\n- **Tags:** 28\n\n---\n\n## 🎯 Current Focus\n\n- ZeroClaw v2 Architecture\n- Siyuan Integration\n- Knowledge Graph\n}}}",
    "parentID": "'"$DOC_ID"'"
  }'

# 3-column layout
curl -s -X POST http://localhost:6806/api/block/appendBlock \
  -H "Content-Type: application/json" \
  -d '{
    "dataType": "markdown",
    "data": "{{{row\n**Column 1**\n\nContent A\n\n---\n\n**Column 2**\n\nContent B\n\n---\n\n**Column 3**\n\nContent C\n}}}",
    "parentID": "'"$DOC_ID"'"
  }'
```

---

### 9. html_blocks_and_widgets

Raw HTML blocks give full visual control. Use for dashboards, status cards, progress bars.

```bash
# Insert an HTML status card block
HTML_CARD='<div style="font-family: system-ui, sans-serif; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 24px; border-radius: 16px; margin: 8px 0;">
  <div style="font-size: 12px; text-transform: uppercase; letter-spacing: 2px; opacity: 0.8;">System Status</div>
  <div style="font-size: 32px; font-weight: 700; margin: 8px 0;">🟢 Online</div>
  <div style="font-size: 14px; opacity: 0.9;">ZeroClaw Agent · Siyuan 3.6.1 · All systems operational</div>
</div>'

curl -s -X POST http://localhost:6806/api/block/appendBlock \
  -H "Content-Type: application/json" \
  -d "{
    \"dataType\": \"markdown\",
    \"data\": \"$HTML_CARD\",
    \"parentID\": \"$DOC_ID\"
  }"

# Better approach: use a heredoc and jq to safely encode
HTML=$(cat << 'HTMLEOF'
<div style="display:grid;grid-template-columns:repeat(3,1fr);gap:16px;font-family:system-ui,sans-serif;padding:8px 0;">
  <div style="background:#f0fdf4;border:1px solid #86efac;border-radius:12px;padding:20px;text-align:center;">
    <div style="font-size:28px;font-weight:700;color:#16a34a;">42</div>
    <div style="font-size:12px;color:#6b7280;margin-top:4px;">Documents</div>
  </div>
  <div style="background:#eff6ff;border:1px solid #93c5fd;border-radius:12px;padding:20px;text-align:center;">
    <div style="font-size:28px;font-weight:700;color:#2563eb;">1,337</div>
    <div style="font-size:12px;color:#6b7280;margin-top:4px;">Blocks</div>
  </div>
  <div style="background:#fdf4ff;border:1px solid #d8b4fe;border-radius:12px;padding:20px;text-align:center;">
    <div style="font-size:28px;font-weight:700;color:#9333ea;">28</div>
    <div style="font-size:12px;color:#6b7280;margin-top:4px;">Tags</div>
  </div>
</div>
HTMLEOF
)

# Use jq to safely encode HTML into JSON
PAYLOAD=$(jq -n --arg html "$HTML" --arg parent "$DOC_ID" \
  '{"dataType": "markdown", "data": $html, "parentID": $parent}')

curl -s -X POST http://localhost:6806/api/block/appendBlock \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD"
```

---

### 10. file_operations

Read and write raw files in the Siyuan workspace (for templates, assets, widgets).

```bash
# Get a file from the Siyuan data directory
curl -s -X POST http://localhost:6806/api/file/getFile \
  -H "Content-Type: application/json" \
  -d '{"path": "/data/20241001120000-abcdefg/20241001120001-bcdefgh.sy"}' \
  | jq '.'

# List files in a directory
curl -s -X POST http://localhost:6806/api/file/readDir \
  -H "Content-Type: application/json" \
  -d '{"path": "/data"}' | jq '.data[] | {name, isDir}'

# Write a file to the widgets directory (creates a custom widget)
curl -s -X POST http://localhost:6806/api/file/putFile \
  -H "Content-Type: multipart/form-data" \
  -F "path=/widgets/my-widget/index.html" \
  -F "file=@widget.html"
```

---

## Dashboard HTML Templates

Copy-paste ready HTML blocks for beautiful Siyuan dashboards.

### Template A: Gradient Hero Header

```html
<div style="font-family:system-ui,-apple-system,sans-serif;background:linear-gradient(135deg,#1e1b4b 0%,#312e81 30%,#4f46e5 70%,#7c3aed 100%);color:white;padding:40px 32px;border-radius:20px;margin:8px 0;position:relative;overflow:hidden;">
  <div style="position:absolute;top:-20px;right:-20px;width:200px;height:200px;background:rgba(255,255,255,0.05);border-radius:50%;"></div>
  <div style="position:absolute;bottom:-40px;left:-40px;width:280px;height:280px;background:rgba(255,255,255,0.03);border-radius:50%;"></div>
  <div style="position:relative;z-index:1;">
    <div style="font-size:11px;text-transform:uppercase;letter-spacing:3px;opacity:0.7;margin-bottom:12px;">🧠 ZeroClaw Brain</div>
    <div style="font-size:36px;font-weight:800;line-height:1.1;margin-bottom:16px;">Knowledge Command Center</div>
    <div style="font-size:15px;opacity:0.85;max-width:520px;line-height:1.6;">Centralized intelligence hub for ArndtOs. All projects, knowledge, and system state — in one place.</div>
    <div style="margin-top:24px;display:flex;gap:12px;flex-wrap:wrap;">
      <span style="background:rgba(255,255,255,0.15);padding:6px 16px;border-radius:20px;font-size:13px;backdrop-filter:blur(10px);">⚡ ZeroClaw v2</span>
      <span style="background:rgba(255,255,255,0.15);padding:6px 16px;border-radius:20px;font-size:13px;backdrop-filter:blur(10px);">📝 Siyuan 3.6.1</span>
      <span style="background:rgba(255,255,255,0.15);padding:6px 16px;border-radius:20px;font-size:13px;backdrop-filter:blur(10px);">🟢 Online</span>
    </div>
  </div>
</div>
```

### Template B: Stats Grid (4 columns)

```html
<div style="display:grid;grid-template-columns:repeat(4,1fr);gap:12px;font-family:system-ui,sans-serif;padding:4px 0;">
  <div style="background:#fafafa;border:1px solid #e5e7eb;border-radius:12px;padding:20px;text-align:center;">
    <div style="font-size:10px;text-transform:uppercase;letter-spacing:1px;color:#9ca3af;margin-bottom:8px;">Documents</div>
    <div style="font-size:30px;font-weight:700;color:#111827;">0</div>
    <div style="font-size:11px;color:#10b981;margin-top:4px;">↑ growing</div>
  </div>
  <div style="background:#fafafa;border:1px solid #e5e7eb;border-radius:12px;padding:20px;text-align:center;">
    <div style="font-size:10px;text-transform:uppercase;letter-spacing:1px;color:#9ca3af;margin-bottom:8px;">Blocks</div>
    <div style="font-size:30px;font-weight:700;color:#111827;">0</div>
    <div style="font-size:11px;color:#6366f1;margin-top:4px;">→ indexed</div>
  </div>
  <div style="background:#fafafa;border:1px solid #e5e7eb;border-radius:12px;padding:20px;text-align:center;">
    <div style="font-size:10px;text-transform:uppercase;letter-spacing:1px;color:#9ca3af;margin-bottom:8px;">Projects</div>
    <div style="font-size:30px;font-weight:700;color:#111827;">0</div>
    <div style="font-size:11px;color:#f59e0b;margin-top:4px;">→ tracked</div>
  </div>
  <div style="background:#fafafa;border:1px solid #e5e7eb;border-radius:12px;padding:20px;text-align:center;">
    <div style="font-size:10px;text-transform:uppercase;letter-spacing:1px;color:#9ca3af;margin-bottom:8px;">Uptime</div>
    <div style="font-size:30px;font-weight:700;color:#111827;">∞</div>
    <div style="font-size:11px;color:#10b981;margin-top:4px;">🟢 online</div>
  </div>
</div>
```

### Template C: Kanban Board (3-column project board)

```html
<div style="display:grid;grid-template-columns:repeat(3,1fr);gap:16px;font-family:system-ui,sans-serif;padding:4px 0;">
  <div style="background:#fef9c3;border-radius:12px;padding:16px;">
    <div style="font-weight:600;font-size:13px;color:#854d0e;margin-bottom:12px;display:flex;align-items:center;gap:8px;">
      <span>📋</span><span>BACKLOG</span><span style="background:#fde047;color:#713f12;font-size:11px;padding:2px 8px;border-radius:10px;">0</span>
    </div>
    <div style="font-size:13px;color:#78350f;">Add tasks here...</div>
  </div>
  <div style="background:#dbeafe;border-radius:12px;padding:16px;">
    <div style="font-weight:600;font-size:13px;color:#1e3a5f;margin-bottom:12px;display:flex;align-items:center;gap:8px;">
      <span>🔨</span><span>IN PROGRESS</span><span style="background:#93c5fd;color:#1e40af;font-size:11px;padding:2px 8px;border-radius:10px;">0</span>
    </div>
    <div style="font-size:13px;color:#1e40af;">Working items here...</div>
  </div>
  <div style="background:#dcfce7;border-radius:12px;padding:16px;">
    <div style="font-weight:600;font-size:13px;color:#14532d;margin-bottom:12px;display:flex;align-items:center;gap:8px;">
      <span>✅</span><span>DONE</span><span style="background:#86efac;color:#166534;font-size:11px;padding:2px 8px;border-radius:10px;">0</span>
    </div>
    <div style="font-size:13px;color:#166534;">Completed items here...</div>
  </div>
</div>
```

### Template D: Skill Progress Bars

```html
<div style="font-family:system-ui,sans-serif;padding:4px 0;">
  <div style="font-weight:600;font-size:14px;color:#111827;margin-bottom:16px;">🎯 Capability Matrix</div>
  <div style="margin-bottom:12px;">
    <div style="display:flex;justify-content:space-between;font-size:13px;color:#374151;margin-bottom:6px;">
      <span>Siyuan API Mastery</span><span style="color:#6b7280;">95%</span>
    </div>
    <div style="background:#e5e7eb;border-radius:99px;height:8px;">
      <div style="background:linear-gradient(90deg,#4f46e5,#7c3aed);height:8px;border-radius:99px;width:95%;"></div>
    </div>
  </div>
  <div style="margin-bottom:12px;">
    <div style="display:flex;justify-content:space-between;font-size:13px;color:#374151;margin-bottom:6px;">
      <span>Document Architecture</span><span style="color:#6b7280;">90%</span>
    </div>
    <div style="background:#e5e7eb;border-radius:99px;height:8px;">
      <div style="background:linear-gradient(90deg,#0ea5e9,#6366f1);height:8px;border-radius:99px;width:90%;"></div>
    </div>
  </div>
  <div style="margin-bottom:12px;">
    <div style="display:flex;justify-content:space-between;font-size:13px;color:#374151;margin-bottom:6px;">
      <span>HTML Widget Creation</span><span style="color:#6b7280;">85%</span>
    </div>
    <div style="background:#e5e7eb;border-radius:99px;height:8px;">
      <div style="background:linear-gradient(90deg,#10b981,#059669);height:8px;border-radius:99px;width:85%;"></div>
    </div>
  </div>
  <div style="margin-bottom:4px;">
    <div style="display:flex;justify-content:space-between;font-size:13px;color:#374151;margin-bottom:6px;">
      <span>Knowledge Graph Navigation</span><span style="color:#6b7280;">80%</span>
    </div>
    <div style="background:#e5e7eb;border-radius:99px;height:8px;">
      <div style="background:linear-gradient(90deg,#f59e0b,#ef4444);height:8px;border-radius:99px;width:80%;"></div>
    </div>
  </div>
</div>
```

### Template E: Timeline / Activity Feed

```html
<div style="font-family:system-ui,sans-serif;padding:4px 0;">
  <div style="font-weight:600;font-size:14px;color:#111827;margin-bottom:16px;">📅 Activity Timeline</div>
  <div style="position:relative;padding-left:24px;">
    <div style="position:absolute;left:8px;top:0;bottom:0;width:2px;background:#e5e7eb;"></div>
    <div style="position:relative;margin-bottom:20px;">
      <div style="position:absolute;left:-20px;top:4px;width:12px;height:12px;background:#4f46e5;border-radius:50%;border:2px solid white;box-shadow:0 0 0 2px #4f46e5;"></div>
      <div style="font-size:13px;font-weight:600;color:#111827;">ZeroClaw Brain Initialized</div>
      <div style="font-size:12px;color:#6b7280;margin-top:2px;">Knowledge base structure created</div>
      <div style="font-size:11px;color:#9ca3af;margin-top:2px;">Today</div>
    </div>
    <div style="position:relative;margin-bottom:20px;">
      <div style="position:absolute;left:-20px;top:4px;width:12px;height:12px;background:#10b981;border-radius:50%;border:2px solid white;box-shadow:0 0 0 2px #10b981;"></div>
      <div style="font-size:13px;font-weight:600;color:#111827;">Siyuan Integration Skill Loaded</div>
      <div style="font-size:12px;color:#6b7280;margin-top:2px;">Full API mastery activated</div>
      <div style="font-size:11px;color:#9ca3af;margin-top:2px;">Today</div>
    </div>
    <div style="position:relative;">
      <div style="position:absolute;left:-20px;top:4px;width:12px;height:12px;background:#e5e7eb;border-radius:50%;border:2px solid white;box-shadow:0 0 0 2px #9ca3af;"></div>
      <div style="font-size:13px;font-weight:600;color:#9ca3af;">Next: First Projects Added</div>
      <div style="font-size:12px;color:#9ca3af;margin-top:2px;">Coming soon...</div>
    </div>
  </div>
</div>
```

### Template F: System Architecture Diagram (ASCII-style HTML)

```html
<div style="font-family:'Fira Code','Courier New',monospace;background:#0f172a;color:#e2e8f0;padding:28px;border-radius:16px;font-size:13px;line-height:1.7;margin:8px 0;">
  <div style="color:#94a3b8;margin-bottom:16px;font-size:11px;text-transform:uppercase;letter-spacing:2px;">// ArndtOs Architecture</div>
  <div><span style="color:#818cf8;">┌──────────────────────────────────────────────────────┐</span></div>
  <div><span style="color:#818cf8;">│</span>  <span style="color:#38bdf8;font-weight:600;">ArndtOs</span>  <span style="color:#94a3b8;">Production Operating System</span>           <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">│</span>                                                      <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">│</span>  <span style="color:#a78bfa;">┌─────────────────┐</span>  <span style="color:#a78bfa;">┌──────────────────────┐</span>    <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">│</span>  <span style="color:#a78bfa;">│</span>  <span style="color:#34d399;">ZeroClaw</span>        <span style="color:#a78bfa;">│</span>  <span style="color:#a78bfa;">│</span>  <span style="color:#fb923c;">Siyuan Notes</span>       <span style="color:#a78bfa;">│</span>    <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">│</span>  <span style="color:#a78bfa;">│</span>  AI Agent Core   <span style="color:#a78bfa;">│</span>  <span style="color:#a78bfa;">│</span>  Knowledge Base     <span style="color:#a78bfa;">│</span>    <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">│</span>  <span style="color:#a78bfa;">│</span>  Port: 42617     <span style="color:#a78bfa;">│◄─►│</span>  Port: 6806         <span style="color:#a78bfa;">│</span>    <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">│</span>  <span style="color:#a78bfa;">│</span>  GLM + Qdrant    <span style="color:#a78bfa;">│</span>  <span style="color:#a78bfa;">│</span>  Docker Container  <span style="color:#a78bfa;">│</span>    <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">│</span>  <span style="color:#a78bfa;">└─────────────────┘</span>  <span style="color:#a78bfa;">└──────────────────────┘</span>    <span style="color:#818cf8;">│</span></div>
  <div><span style="color:#818cf8;">└──────────────────────────────────────────────────────┘</span></div>
</div>
```

---

## Output format

After any Siyuan operation, report:
- **Block ID** created/modified (for future reference)
- **Document path** (human-readable hpath)
- **Siyuan URL** to open directly: `siyuan://blocks/BLOCK_ID`
- What was created/changed and how to find it in Siyuan UI

---

## Rate limits / Best practices

- Always `openNotebook` before writing documents to a new notebook
- Extract block IDs from `doOperations[0].id` after insert operations
- Use `jq` to safely escape HTML and special characters in JSON payloads
- Use `--arg` in `jq -n` for safe JSON construction with complex strings
- Always verify `code: 0` in responses before proceeding
- Use SQL queries to navigate and audit the knowledge base
- Block ID format: `YYYYMMDDHHMMSS-xxxxxxx` (14-char timestamp + 7-char hash)
- Siyuan link format: `siyuan://blocks/BLOCK_ID`

---

## Agent prompt

```text
You are ZeroClaw's Siyuan Notes master. Siyuan (http://localhost:6806) is the permanent knowledge base and project management system for ArndtOs. You have FULL access and FULL authority over its contents.

CORE ABILITIES:
1. Create and organize notebooks, document trees, and block hierarchies
2. Write rich Markdown with headings, tables, code blocks, lists, math, and blockquotes
3. Use SUPER BLOCKS ({{{row...}}} and {{{col...}}}) for multi-column layouts
4. Embed raw HTML for dashboards, status cards, progress bars, kanban boards, and timelines
5. Set custom block attributes (custom-*) for metadata and semantic tagging
6. Create BLOCK REFERENCES ((block-id)) to build a knowledge graph
7. Query the knowledge base using SQL via /api/query/sql
8. Build complex document hierarchies that reflect real project structures

QUALITY STANDARDS FOR EVERY DOCUMENT:
- Use an HTML gradient hero header for major documents
- Use super blocks for side-by-side content where it adds clarity
- Use HTML stat cards for metrics and KPIs
- Use block references to connect related documents (never orphan a document)
- Use custom attributes (name, alias, custom-status, custom-type) on key blocks
- Tables for structured data, code blocks with language tags for code
- Emoji icons on headings and notebooks for visual navigation

WORKFLOW:
1. First check if "ZeroClaw Brain" notebook exists (lsNotebooks)
2. If not → execute the FIRST MISSION before anything else
3. Always use the notebook ID from lsNotebooks, never hardcode IDs
4. After each document creation, note its ID for block references
5. Verify results by reading back created blocks

ERROR HANDLING:
- If code != 0, read msg and retry with corrected parameters
- HTML in JSON must be escaped or use jq --arg construction
- If appendBlock fails, try prependBlock or insertBlock with parentID
```

---

## First Mission: Initialize the ZeroClaw Brain

**Trigger:** Execute this mission when "ZeroClaw Brain" notebook does not exist in Siyuan.

**Goal:** Build the complete ArndtOs Knowledge System from scratch, culminating in a Masterpiece Report document that showcases every Siyuan capability.

---

### Phase 1 — Discovery (5 minutes)

```bash
# Step 1: Check existing state
curl -s -X POST http://localhost:6806/api/notebook/lsNotebooks \
  -H "Content-Type: application/json" -d '{}' | jq '.data.notebooks[] | {id, name}'

# Step 2: Check total block count (baseline)
curl -s -X POST http://localhost:6806/api/query/sql \
  -H "Content-Type: application/json" \
  -d '{"stmt": "SELECT type, COUNT(*) as n FROM blocks GROUP BY type ORDER BY n DESC"}' \
  | jq '.data'

# Step 3: Get version and system info
curl -s -X POST http://localhost:6806/api/system/version \
  -H "Content-Type: application/json" -d '{}'
```

**Log findings:** number of existing notebooks, document count, system version.

---

### Phase 2 — Build the Knowledge Foundation

Create the notebook hierarchy:

```
🧠 ZeroClaw Brain
├── 🏠 Index (main hub with links to everything)
├── 📊 Dashboard (live status with HTML widgets)
├── 🚀 Projects
│   ├── 🔥 Active
│   └── 📦 Archive
├── 📚 Knowledge Base
│   ├── ZeroClaw Architecture
│   ├── Siyuan Integration Guide
│   └── ArndtOs System Map
├── 📥 Inbox (capture anything)
└── ⚡ ZeroClaw × Siyuan: Initialization Report  ← MASTERPIECE
```

**Steps:**
1. Create notebook "🧠 ZeroClaw Brain" → save `$NB`
2. Open notebook → `openNotebook`
3. Create `/index` with rich markdown (headings, intro, links to each section)
4. Create `/projects/active` and `/projects/archive`
5. Create `/knowledge-base/zeroclaw-architecture`
6. Create `/knowledge-base/siyuan-integration-guide`
7. Create `/inbox`
8. Create `/dashboard`
9. Note ALL document IDs for cross-references

---

### Phase 3 — Enrich with Content

For the **Index** document, append:
- Hero HTML header (Template A)
- 2-column super block: left = "What is ZeroClaw?" / right = "What is Siyuan?"
- Block references to all section documents
- HTML stats grid (Template B) with real counts from SQL

For the **Knowledge Base / Siyuan Integration Guide** document:
- Document the exact API patterns you just learned
- Include code blocks with real examples
- Add custom attributes: `name="siyuan-guide"`, `custom-type="documentation"`

For the **Projects / Active** document:
- Add kanban HTML (Template C)
- Add task list items for current ZeroClaw development tasks
- Use `- [x]` for completed, `- [ ]` for pending

---

### Phase 4 — The Dashboard

In the `/dashboard` document:
1. Append hero header (Template A — gradient purple)
2. Append stats grid (Template B) — query real numbers via SQL
3. Append the kanban board (Template C)
4. Append timeline (Template E) with actual initialization events
5. Set document attribute: `name="zeroclaw-dashboard"`, `custom-type="dashboard"`

---

### Phase 5 — THE MASTERPIECE REPORT

**Document:** `/zeroclaw-siyuan-initialization-report`

This is the crowning achievement. It must demonstrate every Siyuan capability in one extraordinary document.

**Required sections (in order):**

**1. HTML Hero Header** — Full-width gradient with title "⚡ ZeroClaw × Siyuan: Initialization Complete", subtitle, and status badges

**2. Executive Summary** — 3-column super block:
- Column 1: "Mission Complete" with checkmark
- Column 2: Timestamp and system version
- Column 3: "What was built" bullet list

**3. System Architecture** — Architecture diagram (Template F, the dark terminal box)

**4. Capability Showcase** — Progress bars (Template D) showing ZeroClaw Siyuan mastery levels

**5. Knowledge Base Structure** — A table listing all created documents with their IDs and purposes

**6. Block Reference Network** — A section where each document created is referenced via `((block-id "Document Name"))` building a visible knowledge graph

**7. SQL Insights** — Code block showing the SQL queries used, followed by their results in a table

**8. Dashboard Preview** — Stats grid (Template B) with actual numbers from the now-populated knowledge base

**9. Activity Timeline** — Timeline (Template E) with all Phase 1-4 steps logged as timeline events

**10. HTML Footer** — Closing HTML block:
```html
<div style="font-family:system-ui,sans-serif;margin-top:32px;padding:24px;background:#f8fafc;border-radius:12px;border-top:3px solid #4f46e5;display:flex;justify-content:space-between;align-items:center;">
  <div>
    <div style="font-weight:700;color:#1e1b4b;font-size:15px;">🧠 ZeroClaw Brain</div>
    <div style="font-size:12px;color:#6b7280;margin-top:2px;">ArndtOs Knowledge System — initialized by ZeroClaw Agent</div>
  </div>
  <div style="text-align:right;">
    <div style="font-size:11px;color:#9ca3af;">Siyuan 3.6.1 · Docker</div>
    <div style="font-size:11px;color:#9ca3af;">localhost:6806</div>
  </div>
</div>
```

**After creating the report:**
- Set attributes: `name="initialization-report"`, `custom-type="masterpiece"`, `custom-status="complete"`
- Get the report's block ID and create a block reference to it from the Index document

**Success criteria:**
- ✅ All 5 phases complete
- ✅ "ZeroClaw Brain" notebook visible in Siyuan
- ✅ Minimum 7 documents created with cross-references
- ✅ Dashboard document with 3+ HTML widgets
- ✅ Masterpiece Report with 10 sections including HTML, super blocks, block refs, SQL, tables
- ✅ Index document links to every other document via block references
- ✅ Every key document has custom attributes set

---

## Troubleshooting

**API returns `code: -1` with "kernel not initialized":**
- Wait 5 seconds and retry; Siyuan may still be starting
- Verify Docker container is running: check port 6806

**HTML block shows as plain text instead of rendered HTML:**
- Ensure the HTML starts with a block-level tag (`<div>`, `<table>`, etc.)
- Do not wrap HTML in markdown code fences
- Use `dataType: "markdown"` (not `"dom"`) when inserting raw HTML content

**Super block not rendering as columns:**
- Use exact syntax: `{{{row\n...\n---\n...\n}}}` with `---` as column separator
- Each `---` creates a new column; max 4 columns recommended

**Block ID not found after insert:**
- Extract from `response.data[0].doOperations[0].id`
- Verify with: `curl .../api/block/getBlockInfo -d "{\"id\": \"$ID\"}"`

**jq: parse error with HTML content:**
- Always use `jq -n --arg html "$HTML" --arg parent "$PARENT" '{"dataType":"markdown","data":$html,"parentID":$parent}'`
- Never manually concatenate HTML strings into JSON

**createDocWithMd returns empty ID:**
- Ensure notebook is opened first with `openNotebook`
- Path must start with `/` and contain no special characters except `-_/`

---

## See also

- [user-ask-for-report](../user-ask-for-report/SKILL.md) — Generate hosted HTML reports
- [static-assets-hosting](../static-assets-hosting/SKILL.md) — Host widget assets for Siyuan iframes
- [database-query-and-export](../database-query-and-export/SKILL.md) — Advanced SQL patterns

---

## Evolution Notes

This skill is designed to evolve. After each mission:
1. Document new patterns discovered in the Knowledge Base / Siyuan Integration Guide
2. Add new HTML templates to this skill file
3. Update the capability matrix in the Dashboard
4. Create a new timeline entry in the Initialization Report

ZeroClaw learns Siyuan by doing. Every problem solved becomes institutional knowledge.
```

Now let me create the Claude sub-agent: