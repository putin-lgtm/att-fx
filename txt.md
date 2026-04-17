# Binance Realtime Stream Demo

Script Rust này kết nối tới Binance WebSocket, theo dõi giá realtime của một cặp tiền và chỉ in ra giá hiện tại khi có thay đổi.

## Cặp mặc định

Script đang mặc định theo dõi cặp `SOLUSDT` vì cặp này thường có thanh khoản lớn và biến động khá rõ để quan sát.

## Cách chạy

Chạy với cặp mặc định:

```bash
cargo run
```

## Đổi sang cặp khác

Ví dụ chạy với `DOGEUSDT`:

```bash
cargo run -- dogeusdt
```

## Nếu muốn cặp biến động mạnh hơn

Bạn có thể thử `PEPEUSDT`:

```bash
cargo run -- pepeusdt
```

## Dữ liệu đang được subscribe

Mỗi lần chạy, script sẽ chỉ subscribe stream ticker của cặp đã chọn:

- `<symbol>@ticker`

Ví dụ với `solusdt` thì sẽ subscribe:

```json
{
  "method": "SUBSCRIBE",
  "params": [
    "solusdt@ticker"
  ],
  "id": 1
}
```

## Output

Terminal sẽ chỉ in giá của đồng coin khi giá thay đổi.

Ví dụ với `SOLUSDT`, bạn sẽ thấy output dạng:

```text
[2026-04-17 14:23:45.100 ms] {"SOL":"88.17000000 USDT"}
```

Ví dụ với `DOGEUSDT`:

```text
[2026-04-17 14:23:46.225 ms] {"DOGE":"0.18456000 USDT"}
```

## Lưu ý

- Symbol nên truyền ở dạng không có dấu gạch dưới, ví dụ `btcusdt`, `dogeusdt`, `pepeusdt`.
- Nếu `cargo run` lỗi khi kết nối, hãy kiểm tra mạng, firewall hoặc khả năng truy cập tới `wss://stream.binance.com:9443/ws`.
- Script chỉ in lại khi giá `c` từ ticker Binance thay đổi, và mỗi dòng có thêm timestamp tới millisecond.