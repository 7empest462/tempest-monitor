#[cfg(feature = "metrics")]
use metrics_exporter_prometheus::PrometheusBuilder;
use crate::app::App;
use serde_json::json;
use std::net::SocketAddr;
#[cfg(feature = "export")]
use plotters::prelude::*;

#[cfg(feature = "metrics")]
pub fn init_prometheus(port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let builder = PrometheusBuilder::new().with_http_listener(addr);
    
    match builder.install() {
        Ok(_) => log::info!("Prometheus exporter listening on http://{}", addr),
        Err(e) => log::error!("Failed to install Prometheus exporter: {}", e),
    }
}

#[cfg(feature = "metrics")]
pub fn update_metrics(app: &App) {
    let cpu = app.cpu_history.iter().copied().last().unwrap_or(0) as f64;
    let mem = app.sys.used_memory() as f64 / app.sys.total_memory() as f64 * 100.0;
    
    metrics::gauge!("tempest_cpu_usage").set(cpu);
    metrics::gauge!("tempest_mem_usage").set(mem);
    
    if app.gpu_usage >= 0.0 {
        metrics::gauge!("tempest_gpu_usage").set(app.gpu_usage);
    }

    let rx = app.net_rx_history.iter().copied().last().unwrap_or(0) as f64;
    let tx = app.net_tx_history.iter().copied().last().unwrap_or(0) as f64;
    metrics::gauge!("tempest_net_rx_bytes").set(rx);
    metrics::gauge!("tempest_net_tx_bytes").set(tx);
}

pub fn export_json(app: &App) -> String {
    let cpu = app.cpu_history.iter().copied().last().unwrap_or(0);
    let mem_used = app.sys.used_memory();
    let mem_total = app.sys.total_memory();
    
    let j = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "hostname": sysinfo::System::host_name().unwrap_or_default(),
        "metrics": {
            "cpu_percent": cpu,
            "memory": {
                "used_bytes": mem_used,
                "total_bytes": mem_total,
                "percent": (mem_used as f64 / mem_total as f64 * 100.0)
            },
            "gpu": {
                "usage_percent": app.gpu_usage,
                "model": app.gpu_model
            },
            "network": {
                "rx_bytes_sec": app.net_rx_history.iter().copied().last().unwrap_or(0),
                "tx_bytes_sec": app.net_tx_history.iter().copied().last().unwrap_or(0)
            }
        }
    });

    j.to_string()
}


#[cfg(feature = "export")]
pub fn export_chart_png(app: &App, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(path, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Tempest Monitor — Performance History", ("sans-serif", 40).into_font())
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0..120, 0..100)?;

    chart.configure_mesh()
        .x_desc("Time (seconds)")
        .y_desc("Usage %")
        .draw()?;

    let cpu_data: Vec<(i32, i32)> = app.cpu_history.iter().enumerate()
        .map(|(i, &v)| (i as i32, v as i32)).collect();
    
    chart.draw_series(LineSeries::new(cpu_data, &BLUE))?
        .label("CPU %")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    let mem_data: Vec<(i32, i32)> = app.ram_history.iter().enumerate()
        .map(|(i, &v)| (i as i32, v as i32)).collect();

    chart.draw_series(LineSeries::new(mem_data, &MAGENTA))?
        .label("MEM %")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &MAGENTA));

    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    log::info!("Chart exported to {}", path);
    Ok(())
}
