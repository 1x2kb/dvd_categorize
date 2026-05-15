use dioxus::prelude::*;
use log::error;
use models::{ChatRequest, Role, RoledMessage};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::app_data::AppData;

async fn send_streaming_message(
    message: String,
    mut app_data: Signal<AppData>,
    mut streaming_response: Signal<String>,
    mut is_loading: Signal<bool>,
) {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let user_message = RoledMessage { 
        message: message.clone(), 
        role: Role::User 
    };

    app_data.write().ai_chat.with_mut(|chat| {
        if let Some(chat) = chat {
            chat.push(user_message.clone());
        } else {
            *chat = Some(vec![user_message.clone()]);
        }
    });

    streaming_response.set(String::new());

    // Only send new message - backend loads history from DB
    let session_id = app_data.read().chat_session_id.read().clone();
    let chat_request = ChatRequest {
        session_id,
        messages: vec![user_message.clone()],
        model: Some("phi3.5".to_string()),
    };

    // Use fetch API for streaming
    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::Cors);
    
    let body = match serde_json::to_string(&chat_request) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize request: {}", e);
            is_loading.set(false);
            return;
        }
    };
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let request = match web_sys::Request::new_with_str_and_init(
        &format!("http://{hostname}:{server_port}/ai/chat/stream"),
        &opts,
    ) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to create request: {:?}", e);
            is_loading.set(false);
            return;
        }
    };

    if let Err(e) = request.headers().set("Content-Type", "application/json") {
        error!("Failed to set headers: {:?}", e);
        is_loading.set(false);
        return;
    }

    let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(v) => v,
        Err(e) => {
            error!("Fetch failed: {:?}", e);
            let error_message = RoledMessage { 
                message: "Failed to connect to AI service".to_string(), 
                role: Role::Ai 
            };
            app_data.write().ai_chat.with_mut(|chat| {
                if let Some(chat) = chat {
                    chat.push(error_message);
                }
            });
            is_loading.set(false);
            return;
        }
    };

    let resp: web_sys::Response = resp_value.dyn_into().unwrap();
    
    if !resp.ok() {
        error!("Response not OK: {}", resp.status());
        is_loading.set(false);
        return;
    }

    let body = match resp.body() {
        Some(b) => b,
        None => {
            error!("No response body");
            is_loading.set(false);
            return;
        }
    };

    let reader = body.get_reader();
    let reader: web_sys::ReadableStreamDefaultReader = reader.unchecked_into();

    let mut accumulated = String::new();
    let mut buffer = String::new();

    loop {
        let chunk = match JsFuture::from(reader.read()).await {
            Ok(c) => c,
            Err(e) => {
                error!("Read error: {:?}", e);
                break;
            }
        };

        let done = js_sys::Reflect::get(&chunk, &"done".into()).unwrap();
        if done.as_bool().unwrap_or(false) {
            break;
        }

        let value = js_sys::Reflect::get(&chunk, &"value".into()).unwrap();
        let uint8_array = js_sys::Uint8Array::new(&value);
        let bytes = uint8_array.to_vec();
        
        if let Ok(text) = std::str::from_utf8(&bytes) {
            buffer.push_str(text);
            
            // Process complete SSE lines
            let mut current_event = String::new();
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer.drain(..=newline_pos);
                
                if line.starts_with("event: ") {
                    current_event = line[7..].to_string();
                } else if line.starts_with("data: ") {
                    let data = &line[6..];
                    
                    if current_event == "message" {
                        accumulated.push_str(data);
                        streaming_response.set(accumulated.clone());
                    } else if current_event == "session" {
                        app_data.write().chat_session_id.set(Some(data.to_string()));
                    }
                } else if line.starts_with("event: done") {
                    break;
                } else if line.starts_with("event: error") {
                    error!("Stream error event received");
                    break;
                }
            }
        }
    }

    // Save final message
    if !accumulated.is_empty() {
        let ai_message = RoledMessage { 
            message: accumulated, 
            role: Role::Ai 
        };
        app_data.write().ai_chat.with_mut(|chat| {
            if let Some(chat) = chat {
                chat.push(ai_message);
            }
        });
    }
    
    is_loading.set(false);
}

#[component]
pub fn AiChat() -> Element {
    let mut app_data = consume_context::<Signal<AppData>>();
    let mut input_value = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut streaming_response = use_signal(String::new);

    use_effect(move || {
        if let Some(messages) = app_data.read().ai_chat.read().as_ref() {
            if !messages.is_empty() {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Some(container) = document.get_element_by_id("chat-messages-container") {
                            container.set_scroll_top(container.scroll_height());
                        }
                    }
                }
            }
        }
    });

    rsx! {
        div {
            class: "chat-container",
            
            div {
                class: "chat-header",
                h2 { "Movie Collection Chat" }
                p { class: "chat-subtitle", "Ask me anything about your DVD collection" }
            }

            div {
                class: "chat-messages",
                id: "chat-messages-container",
                
                if let Some(messages) = app_data.read().ai_chat.read().as_ref() {
                    if messages.is_empty() {
                        div {
                            class: "empty-state",
                            div { class: "empty-icon", "💬" }
                            p { "Start a conversation about your movie collection" }
                            div { class: "example-prompts",
                                p { class: "prompt-label", "Try asking:" }
                                div { class: "prompt-item", "• What action movies do I have?" }
                                div { class: "prompt-item", "• Recommend a movie for tonight" }
                                div { class: "prompt-item", "• Where is The Matrix located?" }
                            }
                        }
                    } else {
                        for (index, message) in messages.iter().enumerate() {
                            div {
                                key: "{index}",
                                class: if message.role == Role::User { "message-wrapper user-message" } else { "message-wrapper ai-message" },
                                div {
                                    class: "message-bubble",
                                    div { class: "message-role", "{message.role}" }
                                    div { class: "message-content", "{message.message}" }
                                }
                            }
                        }
                        
                        if *is_loading.read() {
                            div {
                                class: "message-wrapper ai-message",
                                div {
                                    class: "message-bubble streaming",
                                    div { class: "message-role", "Ai" }
                                    div { class: "message-content",
                                        "{streaming_response}"
                                        span { class: "cursor", "▋" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div {
                class: "chat-input-container",
                textarea {
                    class: "chat-input",
                    placeholder: "Type your message...",
                    value: "{input_value}",
                    rows: "3",
                    oninput: move |event| input_value.set(event.value()),
                    onkeypress: move |event| {
                        if event.key() == Key::Enter {
                            event.prevent_default();
                            
                            let message = input_value.read().trim().to_string();
                            if message.is_empty() || *is_loading.read() {
                                return;
                            }

                            input_value.set(String::new());
                            is_loading.set(true);

                            spawn(send_streaming_message(message, app_data, streaming_response, is_loading));
                        }
                    }
                }
                button {
                    class: "send-button",
                    disabled: input_value.read().trim().is_empty() || *is_loading.read(),
                    onclick: move |_| {
                        let message = input_value.read().trim().to_string();
                        if message.is_empty() || *is_loading.read() {
                            return;
                        }

                        input_value.set(String::new());
                        is_loading.set(true);

                        spawn(send_streaming_message(message, app_data, streaming_response, is_loading));
                    },
                    if *is_loading.read() {
                        "Sending..."
                    } else {
                        "Send"
                    }
                }
            }
        }
    }
}
