use reqwest::Client;
use serde_json::json;
use std::env;

/// Prefer Google `text-embedding-004`, then OpenAI `text-embedding-3-small`.
/// Independent of the user's chat model / provider.
pub async fn get_embedding(client: &Client, text: &str) -> Result<Vec<f32>, String> {
    let google_key = env::var("GOOGLE_API_KEY").ok().filter(|k| !k.is_empty());
    if let Some(api_key) = google_key {
        return embed_google(client, &api_key, text).await;
    }

    let openai_key = env::var("OPENAI_API_KEY").ok().filter(|k| !k.is_empty());
    if let Some(api_key) = openai_key {
        return embed_openai(client, &api_key, text).await;
    }

    Err("No embedding key available; set GOOGLE_API_KEY or OPENAI_API_KEY".into())
}

async fn embed_google(client: &Client, api_key: &str, text: &str) -> Result<Vec<f32>, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key={}",
        api_key
    );
    let body = json!({
        "content": {
            "parts": [{"text": text}]
        }
    });

    let res = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!(
            "Google Embedding Error: {}",
            res.text().await.unwrap_or_default()
        ));
    }

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let embedding = json["embedding"]["values"]
        .as_array()
        .ok_or("Invalid format")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();
    Ok(embedding)
}

async fn embed_openai(client: &Client, api_key: &str, text: &str) -> Result<Vec<f32>, String> {
    let body = json!({
        "model": "text-embedding-3-small",
        "input": text
    });
    let res = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!(
            "OpenAI Embedding Error: {}",
            res.text().await.unwrap_or_default()
        ));
    }

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let embedding = json["data"][0]["embedding"]
        .as_array()
        .ok_or("Invalid format")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();
    Ok(embedding)
}
