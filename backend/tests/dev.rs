use anyhow::Result;

#[tokio::test]
async fn dev() -> Result<()> {
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:3000/u/test-id/files")
        .header("Content-Type", "application/json")
        .body(r#"{
            "path": "test-path",
            "uuid": "8f664c5d-8751-4b8d-bd07-0b115e97f24a",
            "date": "2021-03-28T00:12:00+02:00",
            "sha256": "f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2"
        }"#)
        .send()
        .await?;
    println!("Response: {:?}", response);
    println!("Response: {:?}", response.text().await?);

    Ok(())
}
