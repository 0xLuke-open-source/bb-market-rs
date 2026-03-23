use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn read_to_string(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

fn extract_dashboard_assets_from_server_rs(server_rs_src: &str) -> anyhow::Result<DashboardExtract> {
    // Matches: `const HTML: &str = r#" ... "#;`
    let start_token = "const HTML: &str = r#\"";
    let start = server_rs_src
        .find(start_token)
        .ok_or_else(|| anyhow::anyhow!("cannot find HTML start token in src/web/server.rs"))?;
    let start_content = start + start_token.len();

    let end_rel = server_rs_src[start_content..]
        .find("\"#;")
        .ok_or_else(|| anyhow::anyhow!("cannot find HTML end token in src/web/server.rs"))?;
    let html = &server_rs_src[start_content..start_content + end_rel];

    let script_open = "<script>";
    let script_close = "</script>";
    let open_pos = html
        .find(script_open)
        .ok_or_else(|| anyhow::anyhow!("cannot find <script> in HTML constant"))?;
    let close_pos = html[open_pos..]
        .find(script_close)
        .ok_or_else(|| anyhow::anyhow!("cannot find </script> in HTML constant"))?
        + open_pos;
    let close_end = close_pos + script_close.len();

    let prefix = &html[..open_pos];
    let suffix = &html[close_end..];

    let script_contents = &html[open_pos + script_open.len()..close_pos];

    // Split JS by markers we know are present and stable in the current HTML:
    // - state.js: everything before `function ensureTradingView`
    // - tv.js: from `function ensureTradingView` to `function filterP`
    // - app.js: from `function filterP` to end
    let tv_start = script_contents
        .find("function ensureTradingView")
        .ok_or_else(|| anyhow::anyhow!("cannot find TradingView section in embedded JS"))?;
    let filter_p_start = script_contents
        .find("function filterP")
        .ok_or_else(|| anyhow::anyhow!("cannot find left-search section (function filterP) in embedded JS"))?;

    let state_js = &script_contents[..tv_start];
    let tv_js = &script_contents[tv_start..filter_p_start];
    let app_js = &script_contents[filter_p_start..];

    Ok(DashboardExtract {
        prefix,
        suffix,
        state_js,
        tv_js,
        app_js,
    })
}

struct DashboardExtract<'a> {
    prefix: &'a str,
    suffix: &'a str,
    state_js: &'a str,
    tv_js: &'a str,
    app_js: &'a str,
}

fn write_file(path: &Path, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn extract_chunk(src: &str, start_idx: usize, end_idx: usize) -> &str {
    &src[start_idx..end_idx]
}

fn split_app_js_into_modules(app_js: &str, module_root: &Path, force: bool) -> anyhow::Result<()> {
    // Modules:
    // - prefs.js: search/prefs/favorites/symbol detail loader helpers
    // - orders.js: trader order records tab
    // - trade.js: trade form + submit/cancel actions
    // - metrics.js: enterprise metrics helpers + CVD/OHLCV helpers
    // - render.js: main rendering + list/detail/signal sections
    // - alerts.js: alert generation
    // - tools.js: DOM helpers + formatting + replay/websocket/bootstrap

    let prefs_path = module_root.join("prefs.js");
    let orders_path = module_root.join("orders.js");
    let trade_path = module_root.join("trade.js");
    let metrics_path = module_root.join("metrics.js");
    let render_path = module_root.join("render.js");
    let alerts_path = module_root.join("alerts.js");
    let tools_path = module_root.join("tools.js");

    let modules_exist = prefs_path.exists()
        && orders_path.exists()
        && trade_path.exists()
        && metrics_path.exists()
        && render_path.exists()
        && alerts_path.exists()
        && tools_path.exists();

    if modules_exist && !force {
        return Ok(());
    }

    let marker_orders = "// ── 委托记录 Tab";
    let marker_trade = "// ── 交易类型切换";
    let marker_metrics = "// ── EMA";
    let marker_render = "// ── 主渲染";
    let marker_alerts = "// ── 预警";
    let marker_tools = "// ── 工具";

    let idx_orders = app_js
        .find(marker_orders)
        .ok_or_else(|| anyhow::anyhow!("cannot find marker: {marker_orders} in app.js"))?;
    let idx_trade = app_js
        .find(marker_trade)
        .ok_or_else(|| anyhow::anyhow!("cannot find marker: {marker_trade} in app.js"))?;
    let idx_metrics = app_js
        .find(marker_metrics)
        .ok_or_else(|| anyhow::anyhow!("cannot find marker: {marker_metrics} in app.js"))?;
    let idx_render = app_js
        .find(marker_render)
        .ok_or_else(|| anyhow::anyhow!("cannot find marker: {marker_render} in app.js"))?;
    let idx_alerts = app_js
        .find(marker_alerts)
        .ok_or_else(|| anyhow::anyhow!("cannot find marker: {marker_alerts} in app.js"))?;
    let idx_tools = app_js
        .find(marker_tools)
        .ok_or_else(|| anyhow::anyhow!("cannot find marker: {marker_tools} in app.js"))?;

    // Start of file is prefs chunk.
    let prefs_chunk = extract_chunk(app_js, 0, idx_orders);
    let orders_chunk = extract_chunk(app_js, idx_orders, idx_trade);
    let trade_chunk = extract_chunk(app_js, idx_trade, idx_metrics);
    let metrics_chunk = extract_chunk(app_js, idx_metrics, idx_render);
    let render_chunk = extract_chunk(app_js, idx_render, idx_alerts);
    let alerts_chunk = extract_chunk(app_js, idx_alerts, idx_tools);
    let tools_chunk = extract_chunk(app_js, idx_tools, app_js.len());

    write_file(&prefs_path, prefs_chunk)?;
    write_file(&orders_path, orders_chunk)?;
    write_file(&trade_path, trade_chunk)?;
    write_file(&metrics_path, metrics_chunk)?;
    write_file(&render_path, render_chunk)?;
    write_file(&alerts_path, alerts_chunk)?;
    write_file(&tools_path, tools_chunk)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=src/web/server.rs");
    println!("cargo:rerun-if-env-changed=DASHBOARD_EXTRACT_FROM_SERVER");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let crate_dir = PathBuf::from(manifest_dir);
    let server_rs = crate_dir.join("src/web/server.rs");
    let server_src = read_to_string(&server_rs)?;

    let dash_root = crate_dir.join("src/web/dashboard");
    let js_dir = dash_root.join("js");
    let index_path = dash_root.join("index.html");
    let state_path = js_dir.join("state.js");
    let tv_path = js_dir.join("tv.js");
    let app_path = js_dir.join("app.js");

    let force_extract = std::env::var("DASHBOARD_EXTRACT_FROM_SERVER")
        .ok()
        .as_deref()
        == Some("1");

    let module_root = js_dir.join("app");
    let force_split = std::env::var("DASHBOARD_SPLIT_APP")
        .ok()
        .as_deref()
        == Some("1");

    let skip_extract = !force_extract
        && index_path.exists()
        && state_path.exists()
        && tv_path.exists()
        && app_path.exists()
        && module_root.exists();

    if !skip_extract {
        let extracted = extract_dashboard_assets_from_server_rs(&server_src)?;

        // Replace inline <script> with classic script tags (no `type="module"`).
        // We load the further split modules instead of a single `app.js`.
        let script_tags = r#"
<script src="/static/js/state.js"></script>
<script src="/static/js/tv.js"></script>
<script src="/static/js/app/prefs.js"></script>
<script src="/static/js/app/orders.js"></script>
<script src="/static/js/app/trade.js"></script>
<script src="/static/js/app/metrics.js"></script>
<script src="/static/js/app/render.js"></script>
<script src="/static/js/app/alerts.js"></script>
<script src="/static/js/app/tools.js"></script>
"#;

        let index_html = format!("{}{}{}", extracted.prefix, script_tags, extracted.suffix);

        write_file(&index_path, &index_html)?;
        write_file(&state_path, extracted.state_js)?;
        write_file(&tv_path, extracted.tv_js)?;
        write_file(&app_path, extracted.app_js)?;
    }

    // Split the extracted `app.js` into standard functional modules.
    // We only do it when modules are missing (or forced) to avoid overwriting edits.
    let app_js_src = read_to_string(&app_path)?;
    split_app_js_into_modules(&app_js_src, &module_root, force_split)?;

    Ok(())
}

