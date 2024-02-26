```rust

#[derive(Deserialize)]
struct MyRequest {
...
}

#[derive(Serialize)]
struct MyResponse {
...
}

async fn my_handler(axum_bson::Bson(request): axum_bson::Bson<MyRequest>) -> axum_bson::Bson<MyResponse> {
    ...
    MyResponse {...}
}
```
