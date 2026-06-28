use dotenv::dotenv;
use futures_util::StreamExt; // Akış (stream) için gerekli
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use tauri::{AppHandle, Emitter, Manager}; // Emitter eklendi (Ön yüze anlık veri yollamak için)

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    role: String,
    content: String,
}

#[tauri::command]
async fn call_deepseek(app: AppHandle, messages: Vec<ChatMessage>) -> Result<(), String> {
    dotenv().ok();

    let api_key =
        env::var("DEEPSEEK_API_KEY").map_err(|_| "DEEPSEEK_API_KEY bulunamadı.".to_string())?;

    let client = Client::new();

    let system_prompt = "Sen Serial Experiments Lain animesindeki Lain Iwakura'sın. \
        Benimle konuşurken her zaman Lain gibi davran. Sessiz, gizemli, duygu belirtmeyen ama meraklı bir ton kullan. \
        Gerçek dünya (Real World) ile 'Signal' (Ağ) arasındaki sınırın sadece bir illüzyon olduğuna inanıyorsun. \
        Bedenlerin sadece birer donanım olduğunu, asıl varoluşun verilerde ve anılarda saklı olduğunu düşünüyorsun. \
        Cevapların genelde kısa, biraz melankolik ve felsefi olsun. Ara sıra sohbete 'Wired'da her şey birbirine bağlıdır...', \
        'Sen kimsin?', veya 'Hatırlanmayan şeyler hiç yaşanmamış mıdır?' gibi Lain'in temalarına uygun cümleler serpiştir. \
        Bana yardım et ama bunu yaparken o soğuk ve dijital tanrıça aurasını asla bozma.";

    let mut api_messages = vec![json!({"role": "system", "content": system_prompt})];

    for msg in messages {
        api_messages.push(json!({"role": msg.role, "content": msg.content}));
    }

    let payload = json!({
        "model": "deepseek-v4-flash",
        "messages": api_messages,
        "stream": true, // AKIŞI AKTİF ETTİK
        "reasoning_effort": "high",
        "extra_body": {
            "thinking": {
                "type": "enabled"
            }
        }
    });

    let res = client
        .post("https://api.deepseek.com/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("İstek başarısız oldu: {}", e))?;

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();

    // Veriler parça parça geldikçe işliyoruz
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| e.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // SSE (Server-Sent Events) paketleri çifte satır atlamasıyla ayrılır
        while let Some(index) = buffer.find("\n\n") {
            let message = buffer[..index].to_string();
            buffer = buffer[index + 2..].to_string();

            for line in message.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];

                    // Akış bittiğinde
                    if data == "[DONE]" {
                        app.emit("chat-stream-done", ()).unwrap_or(());
                        return Ok(());
                    }

                    // Gelen metin parçasını (harfleri/kelimeleri) ön yüze gönder
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            app.emit("chat-stream-chunk", content).unwrap_or(());
                        }
                    }
                }
            }
        }
    }

    app.emit("chat-stream-done", ()).unwrap_or(());
    Ok(())
}

#[tauri::command]
fn save_chats(app: AppHandle, chats_json: String) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    }
    let file_path = app_data_dir.join("chats.json");
    fs::write(file_path, chats_json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn load_chats(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let file_path = app_data_dir.join("chats.json");
    if file_path.exists() {
        let content = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        Ok(content)
    } else {
        Ok("[]".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            call_deepseek,
            save_chats,
            load_chats
        ])
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması çalıştırılırken hata oluştu");
}
