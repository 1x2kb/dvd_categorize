use dioxus::html::button;
use dioxus::prelude::*;
use log::{error, info};
use models::{AiResponse, Role, RoledMessage};
use serde::{Deserialize, Serialize};

use crate::app_data::AppData;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AiAction {
    pub uuid: String,
    pub action: String,
    pub model: Option<String>,
}

#[component]
pub fn AiChat() -> Element {
    let mut app_data = consume_context::<Signal<AppData>>();
    let mut input_value = use_signal(|| String::new());

    rsx! {

        div {
                class: "chat-history",
                {app_data.read().ai_chat.read().as_ref().map(|messages| {
                    rsx! {
                        {app_data.read().ai_chat.read().as_ref().map(|messages| {
                            rsx! {
                                for (index, message) in messages.iter().enumerate() {
                                    div {
                                        key: "{index}-{message.role}",
                                        // Base classes always applied
                                        class: "message white-space-pre",
                                        // Conditional class
                                        class: if message.role == Role::Ai { "right-align" },
                                        h3 { "{message.role}" }
                                        p { "{message.message}" }
                                    }
                                }
                            }
                        })}
                    }
                    })}
            }

        input {
            value: "{input_value}",
            oninput: move |event| input_value.set(event.value()),
        }
        button { onclick: move |_| {
            spawn(async move {
                // Use the current page's hostname instead of hardcoded localhost
                let window = web_sys::window().unwrap();
                let location = window.location();
                let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                let server_port = std::env::var("server_port").unwrap_or("3000".to_string());
                let ai_action = AiAction { uuid: String::new(), action: input_value.read().clone(), model: Some("mistral".to_string()) };

                app_data.write().ai_chat.with_mut(|chat| {
                    if let Some(chat) = chat {
                        chat.push(RoledMessage { message: ai_action.action.to_string(), role: Role::User });
                    } else {
                        *chat = Some(vec![RoledMessage { message: ai_action.action.to_string(), role: Role::User }]);
                    }
                });

                let client = reqwest::Client::new();
                let result = client.post(format!("http://{hostname}:{server_port}/ai/chat")).json(&ai_action).send().await;

                match result {
                    Ok(response) => {
                        let roled_message = response.json::<AiResponse>().await.map(|ai_response| RoledMessage { message: ai_response.message, role: Role::Ai }).unwrap_or_else(|e| {error!("failed to parse the response {e}"); RoledMessage { message: "Failed to generate a response".to_string(), role: Role::Ai }});
                        app_data.write().ai_chat.with_mut(|chat| {
                            if let Some(chat) = chat {
                                chat.push(roled_message);
                            }
                        });
                    },
                    Err(_) => {}, // Do nothing for now
                }
            });
        }, "Submit" }
    }
}
