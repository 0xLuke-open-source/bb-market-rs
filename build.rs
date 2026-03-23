use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn read_to_string(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
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

fn rerun_if_changed_dir(dir: &Path) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            rerun_if_changed_dir(&path)?;
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    Ok(())
}

fn build_dashboard_html(partials_dir: &Path, out_path: &Path) -> anyhow::Result<()> {
    let mut partials = Vec::new();

    for entry in fs::read_dir(partials_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            partials.push(path);
        }
    }

    partials.sort();

    if partials.is_empty() {
        anyhow::bail!("no dashboard partials found in {}", partials_dir.display());
    }

    let mut html = String::new();
    for path in partials {
        html.push_str(&read_to_string(&path)?);
        if !html.ends_with('\n') {
            html.push('\n');
        }
    }

    write_file(out_path, &html)
}

fn split_app_js_into_modules(app_js: &str, module_root: &Path, force: bool) -> anyhow::Result<()> {
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
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DASHBOARD_SPLIT_APP");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let crate_dir = PathBuf::from(manifest_dir);

    let dash_root = crate_dir.join("src/web/dashboard");
    let partials_dir = dash_root.join("partials");
    let js_dir = dash_root.join("js");
    let app_path = js_dir.join("app.js");
    let module_root = js_dir.join("app");
    let generated_index = out_dir.join("dashboard_index.html");

    rerun_if_changed_dir(&partials_dir)?;
    println!("cargo:rerun-if-changed={}", app_path.display());

    build_dashboard_html(&partials_dir, &generated_index)?;

    let force_split = std::env::var("DASHBOARD_SPLIT_APP")
        .ok()
        .as_deref()
        == Some("1");

    let app_js_src = read_to_string(&app_path)?;
    split_app_js_into_modules(&app_js_src, &module_root, force_split)?;

    Ok(())
}
