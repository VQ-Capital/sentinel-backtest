// ========== DOSYA: sentinel-backtest/build.rs ==========
fn main() -> std::io::Result<()> {
    // sentinel-spec altındaki protokollere erişim sağlar
    println!("cargo:rerun-if-changed=sentinel-spec/proto/sentinel/market/v1/market_data.proto");
    prost_build::compile_protos(
        &["sentinel-spec/proto/sentinel/market/v1/market_data.proto"],
        &["sentinel-spec/proto/"],
    )?;
    Ok(())
}
