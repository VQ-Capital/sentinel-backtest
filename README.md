# ? sentinel-backtest

**Domain:** 
**Rol:** 

Sistemin "Alfa" (kârlılık) üretebilmesi için stratejileri 10 yıl beklemeden 2 saat içinde test etmemiz gerekiyor. sentinel-backtest servisi, bir zaman makinesi görevi görecek. Geçmiş borsa verilerini (Tick data CSV) okuyup, saniyede on binlerce mesajlık bir hızla, tamamen orijinal sentinel.market.v1.AggTrade Protobuf formatında NATS omurgasına basacak.

Sistemdeki hiçbir bileşen (Inference, Execution, Storage) geçmişte mi yoksa canlıda mı çalıştığını fark etmeyecek. Her şey gerçekmiş gibi reaksiyon verecek.
