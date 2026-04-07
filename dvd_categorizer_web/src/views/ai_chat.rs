use dioxus::prelude::*;
use log::error;
use models::{ChatRequest, ChatResponse, Role, RoledMessage};

use crate::app_data::AppData;

#[component]
pub fn AiChat() -> Element {
    let mut app_data = consume_context::<Signal<AppData>>();
    let mut input_value = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

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
                                    class: "message-bubble loading",
                                    div { class: "message-role", "Ai" }
                                    div { class: "typing-indicator",
                                        span { class: "dot" }
                                        span { class: "dot" }
                                        span { class: "dot" }
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

                            spawn(async move {
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

                                let messages = app_data.read().ai_chat.read().clone().unwrap_or_default();
                                let chat_request = ChatRequest {
                                    messages,
                                    model: Some("phi3.5".to_string()),
                                };

                                let client = reqwest::Client::new();
                                let result = client
                                    .post(format!("http://{hostname}:{server_port}/ai/chat"))
                                    .json(&chat_request)
                                    .send()
                                    .await;

                                match result {
                                    Ok(response) => {
                                        match response.json::<ChatResponse>().await {
                                            Ok(chat_response) => {
                                                let ai_message = RoledMessage { 
                                                    message: chat_response.message, 
                                                    role: Role::Ai 
                                                };
                                                app_data.write().ai_chat.with_mut(|chat| {
                                                    if let Some(chat) = chat {
                                                        chat.push(ai_message);
                                                    }
                                                });
                                            }
                                            Err(e) => {
                                                error!("Failed to parse response: {}", e);
                                                let error_message = RoledMessage { 
                                                    message: "Failed to parse AI response".to_string(), 
                                                    role: Role::Ai 
                                                };
                                                app_data.write().ai_chat.with_mut(|chat| {
                                                    if let Some(chat) = chat {
                                                        chat.push(error_message);
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to send request: {}", e);
                                        let error_message = RoledMessage { 
                                            message: "Failed to connect to AI service".to_string(), 
                                            role: Role::Ai 
                                        };
                                        app_data.write().ai_chat.with_mut(|chat| {
                                            if let Some(chat) = chat {
                                                chat.push(error_message);
                                            }
                                        });
                                    }
                                }
                                
                                is_loading.set(false);
                            });
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

                        spawn(async move {
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

                            let messages = app_data.read().ai_chat.read().clone().unwrap_or_default();
                            let chat_request = ChatRequest {
                                messages,
                                model: Some("phi3.5".to_string()),
                            };

                            let client = reqwest::Client::new();
                            let result = client
                                .post(format!("http://{hostname}:{server_port}/ai/chat"))
                                .json(&chat_request)
                                .send()
                                .await;

                            match result {
                                Ok(response) => {
                                    match response.json::<ChatResponse>().await {
                                        Ok(chat_response) => {
                                            let ai_message = RoledMessage { 
                                                message: chat_response.message, 
                                                role: Role::Ai 
                                            };
                                            app_data.write().ai_chat.with_mut(|chat| {
                                                if let Some(chat) = chat {
                                                    chat.push(ai_message);
                                                }
                                            });
                                        }
                                        Err(e) => {
                                            error!("Failed to parse response: {}", e);
                                            let error_message = RoledMessage { 
                                                message: "Failed to parse AI response".to_string(), 
                                                role: Role::Ai 
                                            };
                                            app_data.write().ai_chat.with_mut(|chat| {
                                                if let Some(chat) = chat {
                                                    chat.push(error_message);
                                                }
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to send request: {}", e);
                                    let error_message = RoledMessage { 
                                        message: "Failed to connect to AI service".to_string(), 
                                        role: Role::Ai 
                                    };
                                    app_data.write().ai_chat.with_mut(|chat| {
                                        if let Some(chat) = chat {
                                            chat.push(error_message);
                                        }
                                    });
                                }
                            }
                            
                            is_loading.set(false);
                        });
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
