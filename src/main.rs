// ========== DOSYA: sentinel-backtest/src/main.rs ==========
use anyhow::{Context, Result};
use bytes::BytesMut;
use clap::Parser;
use prost::Message;
use serde::Deserialize;
use tokio::time::{sleep, Duration, Instant};
use tracing::{info, warn}; // 🔥 CERRAHİ: Kullanılmayan 'error' importu silindi

pub mod sentinel_market {
    include!(concat!(env!("OUT_DIR"), "/sentinel.market.v1.rs"));
}

use sentinel_market::AggTrade;

#[derive(Parser, Debug)]
#[command(author, version, about = "VQ-Capital HFT Backtest Injector", long_about = None)]
struct Args {
    /// İşlenecek geçmiş borsa verisi (CSV)
    #[arg(short, long)]
    csv_file: String,

    /// Hangi sembol olarak sisteme enjekte edilecek? (Örn: BTCUSDT)
    #[arg(short, long, default_value = "BTCUSDT")]
    symbol: String,

    /// Hangi borsadan gelmiş gibi gösterilecek? (Örn: binance)
    #[arg(short, long, default_value = "binance")]
    exchange: String,

    /// NATS Sunucu Adresi
    #[arg(short, long, default_value = "nats://localhost:4222")]
    nats_url: String,

    /// Saniyede maksimum basılacak mesaj limiti (Tıkanmayı önlemek için)
    #[arg(short, long, default_value = "50000")]
    max_mps: u32,
}

/// Binance standart Tick CSV formatı:
/// agg_trade_id, price, quantity, first_trade_id, last_trade_id, timestamp, is_buyer_maker, is_best_match
#[derive(Debug, Deserialize)]
struct HistoricalTick {
    #[serde(rename = "price")]
    price: f64,
    #[serde(rename = "qty")]
    quantity: f64,
    #[serde(rename = "time")]
    timestamp: i64,
    #[serde(rename = "is_buyer_maker")]
    is_buyer_maker: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!(
        "📡 Service: {} | Version: {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let args = Args::parse();

    info!("🦅 VQ-Capital Backtest Injector (Zaman Makinesi) Başlatılıyor...");
    info!("📂 Veri Kaynağı: {}", args.csv_file);
    info!("🎯 Hedef Sembol: {}", args.symbol);

    let nats_client = async_nats::connect(&args.nats_url)
        .await
        .context("CRITICAL: NATS sunucusuna bağlanılamadı.")?;

    let subject = format!("market.trade.{}.{}", args.exchange, args.symbol);
    info!(
        "🔗 Veriler '{}' kanalına asenkron enjekte edilecek.",
        subject
    );

    // CSV'yi belleğe yüklemek (RAM Allocation) YASAKTIR. Streaming okuma yapıyoruz.
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&args.csv_file)
        .context("CSV dosyası okunamadı. Dosya yolunu kontrol edin.")?;

    // Zero-Allocation Prensibi: Her döngüde yeni byte tahsis etmek yerine,
    // tek bir BytesMut buffer'ı tekrar tekrar kullanıyoruz.
    let mut payload_buffer = BytesMut::with_capacity(1024);

    let mut msg_count: u64 = 0;
    let mut batch_count: u32 = 0;
    let start_time = Instant::now();
    let mut cycle_time = Instant::now();

    let batch_size = args.max_mps / 10; // Saniyede 10 kez uyuma/dengeleme kontrolü
    let sleep_duration = Duration::from_millis(100); // 100ms cycle

    info!(
        "🚀 ENJEKSİYON BAŞLIYOR... (Limit: {} msgs/sec)",
        args.max_mps
    );

    for result in reader.deserialize::<HistoricalTick>() {
        match result {
            Ok(tick) => {
                let agg_trade = AggTrade {
                    symbol: args.symbol.clone(),
                    price: tick.price,
                    quantity: tick.quantity,
                    timestamp: tick.timestamp, // Gerçek zaman değil, GEÇMİŞ zamanı basıyoruz
                    is_buyer_maker: tick.is_buyer_maker,
                };

                payload_buffer.clear();
                if agg_trade.encode(&mut payload_buffer).is_ok() {
                    // freeze() -> O(1) maliyetle buffer'ı Bytes'a dönüştürür
                    let _ = nats_client
                        .publish(subject.clone(), payload_buffer.split().freeze())
                        .await;

                    msg_count += 1;
                    batch_count += 1;
                }

                // HFT Darboğaz (OOM) Koruması: NATS'ı ve Storage'ı boğmamak için pacing (hız ayarı)
                if batch_count >= batch_size {
                    let elapsed = cycle_time.elapsed();
                    if elapsed < sleep_duration {
                        sleep(sleep_duration - elapsed).await;
                    }
                    batch_count = 0;
                    cycle_time = Instant::now();
                }

                if msg_count.is_multiple_of(1_000_000) {
                    let total_sec = start_time.elapsed().as_secs();
                    // 🔥 CERRAHİ: Clippy kurallarına uygun güvenli bölüm (Checked Division)
                    let avg_speed = msg_count.checked_div(total_sec).unwrap_or(0);
                    info!(
                        "⏱️ {} Milyon Tick işlendi. Ortalama Hız: {} tick/sn",
                        msg_count / 1_000_000,
                        avg_speed
                    );
                }
            }
            Err(e) => {
                warn!("⚠️ CSV Satır hatası (Atlanıyor): {}", e);
                continue;
            }
        }
    }

    // NATS buffer'ında kalan son mesajların gitmesini bekle
    nats_client.flush().await?;

    let total_time = start_time.elapsed().as_secs_f64();
    info!("✅ BACKTEST ENJEKSİYONU TAMAMLANDI!");
    info!("📊 Toplam Enjekte Edilen Tick: {}", msg_count);
    info!("⏳ Toplam Süre: {:.2} saniye", total_time);

    if total_time > 0.0 {
        info!(
            "⚡ Genel Ortalama Hız: {:.0} tick/sn",
            msg_count as f64 / total_time
        );
    }

    Ok(())
}
