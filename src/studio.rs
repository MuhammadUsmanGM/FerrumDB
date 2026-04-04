//! # Ferrum Studio
//! 
//! An embedded web dashboard for FerrumDB. Launch with:
//! ```rust,no_run
//! use ferrumdb::StorageEngine;
//! use std::sync::Arc;
//! 
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = Arc::new(StorageEngine::new("ferrum.db").await?);
//! ferrumdb::studio::serve(engine.clone(), 3030).await;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post, delete},
    response::{Html, Json, IntoResponse},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::storage::StorageEngine;

type SharedEngine = Arc<StorageEngine>;

/// Launch Ferrum Studio on the given port. Non-blocking — spawns a background task.
pub async fn serve(engine: Arc<StorageEngine>, port: u16) {
    let app = Router::new()
        .route("/", get(dashboard_page))
        .route("/api/keys", get(api_keys))
        .route("/api/count", get(api_count))
        .route("/api/get/{key}", get(api_get))
        .route("/api/set", post(api_set))
        .route("/api/delete/{key}", delete(api_delete))
        .route("/api/metrics", get(api_metrics))
        .with_state(engine);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("🔥 Ferrum Studio running at http://localhost:{}", port);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
}

// ─── REST API Handlers ───────────────────────────────────────────────────────

async fn api_keys(State(engine): State<SharedEngine>) -> Json<Value> {
    let keys = engine.keys().await;
    Json(json!({ "keys": keys, "count": keys.len() }))
}

async fn api_count(State(engine): State<SharedEngine>) -> Json<Value> {
    Json(json!({ "count": engine.len().await }))
}

async fn api_get(
    State(engine): State<SharedEngine>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    match engine.get(&key).await {
        Some(val) => (StatusCode::OK, Json(json!({ "key": key, "value": val }))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "Key not found" }))).into_response(),
    }
}

#[derive(Deserialize)]
struct SetRequest {
    key: String,
    value: Value,
}

async fn api_set(
    State(engine): State<SharedEngine>,
    Json(body): Json<SetRequest>,
) -> impl IntoResponse {
    match engine.set(body.key.clone(), body.value).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "ok": true, "key": body.key }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn api_delete(
    State(engine): State<SharedEngine>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    match engine.delete(&key).await {
        Ok(Some(_)) => (StatusCode::OK, Json(json!({ "ok": true, "key": key }))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Key not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

async fn api_metrics(State(engine): State<SharedEngine>) -> Json<Value> {
    let snapshot = engine.metrics().snapshot();
    Json(json!(snapshot))
}

// ─── Dashboard HTML ───────────────────────────────────────────────────────────

async fn dashboard_page() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ferrum Studio</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg: #0a0a0f;
            --surface: #12121a;
            --surface2: #1a1a26;
            --border: #2a2a40;
            --accent: #e05a3a;
            --accent2: #f07a55;
            --text: #e8e8f0;
            --text-muted: #6a6a8a;
            --success: #3ada8c;
            --danger: #e05a5a;
            --mono: 'JetBrains Mono', monospace;
        }
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: 'Inter', sans-serif; background: var(--bg); color: var(--text); min-height: 100vh; }

        /* ── Header ── */
        header {
            background: linear-gradient(135deg, #1a0a06 0%, #12121a 100%);
            border-bottom: 1px solid var(--border);
            padding: 16px 32px;
            display: flex;
            align-items: center;
            gap: 16px;
        }
        .logo { font-size: 22px; font-weight: 700; }
        .logo span { color: var(--accent); }
        .badge {
            background: linear-gradient(135deg, var(--accent), #c0392b);
            color: #fff;
            font-size: 10px;
            font-weight: 700;
            padding: 2px 8px;
            border-radius: 99px;
            letter-spacing: 1px;
        }
        #status-dot {
            margin-left: auto;
            width: 8px; height: 8px;
            border-radius: 50%;
            background: var(--success);
            box-shadow: 0 0 8px var(--success);
            animation: pulse 2s infinite;
        }
        @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }

        /* ── Layout ── */
        .main { display: grid; grid-template-columns: 300px 1fr; min-height: calc(100vh - 57px); }

        /* ── Sidebar ── */
        sidebar {
            background: var(--surface);
            border-right: 1px solid var(--border);
            padding: 24px;
            display: flex;
            flex-direction: column;
            gap: 24px;
        }
        .stat-card {
            background: var(--surface2);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 16px 20px;
        }
        .stat-card .label { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px; margin-bottom: 6px; }
        .stat-card .value { font-size: 32px; font-weight: 700; color: var(--accent2); font-family: var(--mono); }

        .section-title { font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px; margin-bottom: 8px; }
        .key-list { list-style: none; display: flex; flex-direction: column; gap: 4px; max-height: 400px; overflow-y: auto; }
        .key-list li {
            font-family: var(--mono);
            font-size: 13px;
            padding: 8px 12px;
            border-radius: 6px;
            background: var(--surface2);
            border: 1px solid transparent;
            cursor: pointer;
            transition: all 0.15s;
            color: var(--text-muted);
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        .key-list li:hover { border-color: var(--accent); color: var(--text); background: rgba(224,90,58,0.1); }
        .key-list li.active { border-color: var(--accent); color: var(--accent2); background: rgba(224,90,58,0.15); }

        /* ── Main panel ── */
        .panel { padding: 32px; display: flex; flex-direction: column; gap: 24px; overflow-y: auto; }

        /* ── Editor card ── */
        .card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 16px;
            overflow: hidden;
        }
        .card-header {
            background: var(--surface2);
            padding: 14px 20px;
            font-size: 12px;
            font-weight: 600;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 1px;
            border-bottom: 1px solid var(--border);
            display: flex;
            align-items: center;
            gap: 8px;
        }
        .card-body { padding: 20px; }

        /* ── Form elements ── */
        .form-row { display: flex; gap: 12px; align-items: flex-end; flex-wrap: wrap; }
        .form-group { display: flex; flex-direction: column; gap: 6px; flex: 1; min-width: 180px; }
        label { font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.8px; }
        input, textarea {
            background: var(--surface2);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 10px 14px;
            font-family: var(--mono);
            font-size: 13px;
            color: var(--text);
            outline: none;
            transition: border-color 0.15s;
            resize: vertical;
        }
        input:focus, textarea:focus { border-color: var(--accent); }
        textarea { min-height: 100px; }
        
        /* ── Buttons ── */
        .btn {
            border: none;
            border-radius: 8px;
            padding: 10px 20px;
            font-family: 'Inter', sans-serif;
            font-weight: 600;
            font-size: 13px;
            cursor: pointer;
            transition: all 0.15s;
            display: inline-flex;
            align-items: center;
            gap: 6px;
            white-space: nowrap;
        }
        .btn-primary { background: linear-gradient(135deg, var(--accent), #c0392b); color: white; }
        .btn-primary:hover { filter: brightness(1.15); transform: translateY(-1px); }
        .btn-danger { background: rgba(224,90,90,0.15); color: var(--danger); border: 1px solid rgba(224,90,90,0.3); }
        .btn-danger:hover { background: rgba(224,90,90,0.25); }
        .btn-ghost { background: var(--surface2); color: var(--text-muted); border: 1px solid var(--border); }
        .btn-ghost:hover { color: var(--text); border-color: var(--accent); }

        /* ── Value viewer ── */
        #value-display {
            background: var(--bg);
            border-radius: 8px;
            padding: 16px;
            font-family: var(--mono);
            font-size: 13px;
            white-space: pre-wrap;
            word-break: break-all;
            min-height: 80px;
            line-height: 1.6;
            border: 1px solid var(--border);
            color: var(--success);
        }
        #value-display.error { color: var(--danger); }

        /* ── Toast ── */
        #toast {
            position: fixed;
            bottom: 24px; right: 24px;
            background: var(--surface2);
            border: 1px solid var(--border);
            border-radius: 10px;
            padding: 12px 20px;
            font-size: 13px;
            font-weight: 500;
            opacity: 0;
            transform: translateY(10px);
            transition: all 0.25s;
            pointer-events: none;
            z-index: 100;
        }
        #toast.show { opacity: 1; transform: translateY(0); }
        #toast.ok { border-color: var(--success); color: var(--success); }
        #toast.err { border-color: var(--danger); color: var(--danger); }

        /* ── Scrollbar ── */
        ::-webkit-scrollbar { width: 5px; }
        ::-webkit-scrollbar-track { background: transparent; }
        ::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
    </style>
</head>
<body>

<header>
    <div class="logo">⚡ Ferrum<span>DB</span></div>
    <span class="badge">STUDIO</span>
    <div id="status-dot" title="Connected"></div>
</header>

<div class="main">
    <sidebar>
        <div class="stat-card">
            <div class="label">Total Keys</div>
            <div class="value" id="count-display">—</div>
        </div>

        <div class="stat-card">
            <div class="label">Ops/Second</div>
            <div class="value" id="ops-display" style="color:var(--success)">—</div>
        </div>

        <div class="stat-card">
            <div class="label">Uptime</div>
            <div class="value" id="uptime-display" style="color:var(--text);font-size:20px">—</div>
        </div>

        <div>
            <div class="section-title">Keys</div>
            <ul class="key-list" id="key-list"></ul>
        </div>

        <button class="btn btn-ghost" onclick="loadKeys()" style="width:100%">↻ Refresh</button>
    </sidebar>

    <div class="panel">

        <!-- GET -->
        <div class="card">
            <div class="card-header">🔍 Inspect Key</div>
            <div class="card-body" style="display:flex;flex-direction:column;gap:12px;">
                <div class="form-row">
                    <div class="form-group">
                        <label>Key</label>
                        <input type="text" id="get-key" placeholder="e.g. user:42" />
                    </div>
                    <button class="btn btn-primary" onclick="doGet()">Get Value</button>
                    <button class="btn btn-danger" onclick="doDelete()">Delete</button>
                </div>
                <div>
                    <label style="margin-bottom:6px;display:block;">Value</label>
                    <div id="value-display">(select a key from the sidebar or type above)</div>
                </div>
            </div>
        </div>

        <!-- SET -->
        <div class="card">
            <div class="card-header">✏️ Set Key</div>
            <div class="card-body">
                <div class="form-row">
                    <div class="form-group">
                        <label>Key</label>
                        <input type="text" id="set-key" placeholder="e.g. user:42" />
                    </div>
                    <div class="form-group" style="flex:2;">
                        <label>Value (JSON)</label>
                        <input type="text" id="set-value" placeholder='e.g. {"name":"alice","role":"admin"}' />
                    </div>
                    <button class="btn btn-primary" onclick="doSet()">Set</button>
                </div>
            </div>
        </div>

    </div>
</div>

<div id="toast"></div>

<script>
const API = '';

async function loadKeys() {
    const r = await fetch(API + '/api/keys');
    const d = await r.json();
    document.getElementById('count-display').textContent = d.count;
    const list = document.getElementById('key-list');
    list.innerHTML = '';
    (d.keys || []).sort().forEach(k => {
        const li = document.createElement('li');
        li.textContent = k;
        li.onclick = () => selectKey(k, li);
        list.appendChild(li);
    });
}

async function loadMetrics() {
    const r = await fetch(API + '/api/metrics');
    if (!r.ok) return;
    const d = await r.json();
    document.getElementById('ops-display').textContent = d.operations_per_second.toFixed(1);
    document.getElementById('uptime-display').textContent = formatUptime(d.uptime_seconds);
}

function formatUptime(seconds) {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) return `${h}h ${m}m ${s}s`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
}

function selectKey(k, el) {
    document.querySelectorAll('.key-list li').forEach(x => x.classList.remove('active'));
    el.classList.add('active');
    document.getElementById('get-key').value = k;
    doGet();
}

async function doGet() {
    const key = document.getElementById('get-key').value.trim();
    if (!key) return;
    const r = await fetch(API + '/api/get/' + encodeURIComponent(key));
    const d = await r.json();
    const el = document.getElementById('value-display');
    if (r.ok) {
        el.className = '';
        el.textContent = JSON.stringify(d.value, null, 2);
    } else {
        el.className = 'error';
        el.textContent = d.error;
    }
}

async function doSet() {
    const key = document.getElementById('set-key').value.trim();
    let valueStr = document.getElementById('set-value').value.trim();
    if (!key || !valueStr) return toast('Key and value are required', 'err');

    let value;
    try { value = JSON.parse(valueStr); } catch { value = valueStr; }

    const r = await fetch(API + '/api/set', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key, value })
    });
    if (r.ok) { toast('✓ Key set successfully', 'ok'); loadKeys(); }
    else { const d = await r.json(); toast('✗ ' + d.error, 'err'); }
}

async function doDelete() {
    const key = document.getElementById('get-key').value.trim();
    if (!key) return;
    if (!confirm(`Delete "${key}"?`)) return;
    const r = await fetch(API + '/api/delete/' + encodeURIComponent(key), { method: 'DELETE' });
    if (r.ok) {
        toast('✓ Deleted', 'ok');
        document.getElementById('value-display').textContent = '(deleted)';
        loadKeys();
    } else { const d = await r.json(); toast('✗ ' + d.error, 'err'); }
}

let toastTimer;
function toast(msg, type) {
    const el = document.getElementById('toast');
    el.textContent = msg;
    el.className = 'show ' + (type || '');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => el.className = '', 3000);
}

loadKeys();
loadMetrics();
setInterval(loadKeys, 10000);
setInterval(loadMetrics, 2000);
</script>
</body>
</html>"#;
