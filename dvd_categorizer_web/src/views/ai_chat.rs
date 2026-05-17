use dioxus::prelude::*;
use log::error;
use models::{
    AvailableModel, AvailableModelsResponse, ChatRequest, ChatResponse, Role, RoledMessage,
};
use prompts::{DEFAULT_RAG_PROMPT, DEFAULT_TOOL_PROMPT};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::app_data::{AppData, ChatSession};
use crate::components::Markdown;

/// Send a chat message to the non-streaming tool-enabled `/ai/chat` endpoint.
/// The LLM may invoke tools (filter_by_actor / genre / director, get_movie_details)
/// that self-call the API. Returns a single response — no streaming UI.
async fn send_tool_message(
    message: String,
    model_signal: Signal<String>,
    prompt_signal: Signal<String>,
    mut app_data: Signal<AppData>,
    mut is_loading: Signal<bool>,
) {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let user_message = RoledMessage {
        message: message.clone(),
        role: Role::User,
    };

    app_data
        .write()
        .ai_chat
        .with_mut(
            |chat| {
                if let Some(chat) = chat {
                    chat.push(user_message.clone());
                } else {
                    *chat = Some(vec![user_message.clone()]);
                }
            },
        );

    let session_id = app_data
        .read()
        .chat_session_id
        .read()
        .clone();
    let chat_request = ChatRequest {
        session_id,
        messages: vec![user_message],
        model: Some(model_signal()),
        system_prompt: Some(prompt_signal()),
    };

    let url = format!("http://{hostname}:{server_port}/ai/chat");
    let request = match gloo_net::http::Request::post(&url)
        .header(
            "Content-Type",
            "application/json",
        )
        .json(&chat_request)
    {
        Ok(req) => req,
        Err(e) => {
            error!(
                "Failed to build tool chat request: {:?}",
                e
            );
            is_loading.set(false);
            return;
        }
    };

    match request
        .send()
        .await
    {
        Ok(response) => {
            if !response.ok() {
                error!(
                    "Tool chat returned status {}",
                    response.status()
                );
                let error_message = RoledMessage {
                    message: format!(
                        "AI service returned error {}",
                        response.status()
                    ),
                    role: Role::Ai,
                };
                app_data
                    .write()
                    .ai_chat
                    .with_mut(
                        |chat| {
                            if let Some(chat) = chat {
                                chat.push(error_message);
                            }
                        },
                    );
                is_loading.set(false);
                return;
            }

            match response
                .json::<ChatResponse>()
                .await
            {
                Ok(parsed) => {
                    if let Some(sid) = parsed.session_id {
                        app_data
                            .write()
                            .chat_session_id
                            .set(Some(sid));
                    }
                    let ai_message = RoledMessage {
                        message: parsed.message,
                        role: Role::Ai,
                    };
                    app_data
                        .write()
                        .ai_chat
                        .with_mut(
                            |chat| {
                                if let Some(chat) = chat {
                                    chat.push(ai_message);
                                }
                            },
                        );
                }
                Err(e) => {
                    error!(
                        "Failed to parse tool chat response: {:?}",
                        e
                    );
                    let error_message = RoledMessage {
                        message: "Failed to parse AI response".to_string(),
                        role: Role::Ai,
                    };
                    app_data
                        .write()
                        .ai_chat
                        .with_mut(
                            |chat| {
                                if let Some(chat) = chat {
                                    chat.push(error_message);
                                }
                            },
                        );
                }
            }
        }
        Err(e) => {
            error!(
                "Tool chat fetch failed: {:?}",
                e
            );
            let error_message = RoledMessage {
                message: "Failed to connect to AI service".to_string(),
                role: Role::Ai,
            };
            app_data
                .write()
                .ai_chat
                .with_mut(
                    |chat| {
                        if let Some(chat) = chat {
                            chat.push(error_message);
                        }
                    },
                );
        }
    }

    is_loading.set(false);
}

async fn send_streaming_message(
    message: String,
    model_signal: Signal<String>,
    prompt_signal: Signal<String>,
    mut app_data: Signal<AppData>,
    mut streaming_response: Signal<String>,
    mut is_loading: Signal<bool>,
) {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let user_message = RoledMessage {
        message: message.clone(),
        role: Role::User,
    };

    app_data
        .write()
        .ai_chat
        .with_mut(
            |chat| {
                if let Some(chat) = chat {
                    chat.push(user_message.clone());
                } else {
                    *chat = Some(vec![user_message.clone()]);
                }
            },
        );

    streaming_response.set(String::new());

    // Only send new message - backend loads history from DB
    let session_id = app_data
        .read()
        .chat_session_id
        .read()
        .clone();
    let chat_request = ChatRequest {
        session_id,
        messages: vec![user_message.clone()],
        model: Some(model_signal()),
        system_prompt: Some(prompt_signal()),
    };

    // Use fetch API for streaming
    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::Cors);

    let body = match serde_json::to_string(&chat_request) {
        Ok(json) => json,
        Err(e) => {
            error!(
                "Failed to serialize request: {}",
                e
            );
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
            error!(
                "Failed to create request: {:?}",
                e
            );
            is_loading.set(false);
            return;
        }
    };

    if let Err(e) = request
        .headers()
        .set(
            "Content-Type",
            "application/json",
        )
    {
        error!(
            "Failed to set headers: {:?}",
            e
        );
        is_loading.set(false);
        return;
    }

    let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Fetch failed: {:?}",
                e
            );
            let error_message = RoledMessage {
                message: "Failed to connect to AI service".to_string(),
                role: Role::Ai,
            };
            app_data
                .write()
                .ai_chat
                .with_mut(
                    |chat| {
                        if let Some(chat) = chat {
                            chat.push(error_message);
                        }
                    },
                );
            is_loading.set(false);
            return;
        }
    };

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .unwrap();

    if !resp.ok() {
        error!(
            "Response not OK: {}",
            resp.status()
        );
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
                error!(
                    "Read error: {:?}",
                    e
                );
                break;
            }
        };

        let done = js_sys::Reflect::get(
            &chunk,
            &"done".into(),
        )
        .unwrap();
        if done
            .as_bool()
            .unwrap_or(false)
        {
            break;
        }

        let value = js_sys::Reflect::get(
            &chunk,
            &"value".into(),
        )
        .unwrap();
        let uint8_array = js_sys::Uint8Array::new(&value);
        let bytes = uint8_array.to_vec();

        if let Ok(text) = std::str::from_utf8(&bytes) {
            buffer.push_str(text);

            // Process complete SSE lines
            let mut current_event = String::new();
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos]
                    .trim()
                    .to_string();
                buffer.drain(..=newline_pos);

                if line.starts_with("event: ") {
                    current_event = line[7..].to_string();
                } else if line.starts_with("data: ") {
                    let data = &line[6..];

                    if current_event == "message" {
                        accumulated.push_str(data);
                        streaming_response.set(accumulated.clone());
                    } else if current_event == "session" {
                        app_data
                            .write()
                            .chat_session_id
                            .set(Some(data.to_string()));
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
            role: Role::Ai,
        };
        app_data
            .write()
            .ai_chat
            .with_mut(
                |chat| {
                    if let Some(chat) = chat {
                        chat.push(ai_message);
                    }
                },
            );
    }

    is_loading.set(false);
}

#[component]
pub fn AiChat() -> Element {
    let mut app_data = consume_context::<Signal<AppData>>();
    let mut input_value = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut streaming_response = use_signal(String::new);
    let mut show_history = use_signal(|| false);
    // false = streaming RAG (`/ai/chat/stream`), true = non-streaming tool-enabled (`/ai/chat`)
    let mut tool_mode = use_signal(|| false);
    let mut selected_model = use_signal(|| "qwen2.5:3b".to_string());
    let mut available_models = use_signal(Vec::<AvailableModel>::new);
    let mut show_prompt_editor = use_signal(|| false);
    let mut rag_prompt = use_signal(|| DEFAULT_RAG_PROMPT.to_string());
    let mut tool_prompt = use_signal(|| DEFAULT_TOOL_PROMPT.to_string());

    // Fetch available models on mount
    use_effect(
        move || {
            spawn(
                async move {
                    let window = web_sys::window().unwrap();
                    let location = window.location();
                    let hostname = location
                        .hostname()
                        .unwrap_or_else(|_| "127.0.0.1".to_string());
                    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());
                    let url = format!("http://{hostname}:{server_port}/ai/models");

                    match gloo_net::http::Request::get(&url)
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if let Ok(parsed) = response
                                .json::<AvailableModelsResponse>()
                                .await
                            {
                                available_models.set(parsed.models);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to load available models: {:?}",
                                e
                            );
                        }
                    }
                },
            );
        },
    );

    // Load chat sessions on mount
    use_effect(
        move || {
            spawn(
                async move {
                    let window = web_sys::window().unwrap();
                    let location = window.location();
                    let hostname = location
                        .hostname()
                        .unwrap_or_else(|_| "127.0.0.1".to_string());
                    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());
                    let url = format!("http://{hostname}:{server_port}/ai/chat/sessions");

                    match gloo_net::http::Request::get(&url)
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if let Ok(sessions) = response
                                .json::<Vec<ChatSession>>()
                                .await
                            {
                                app_data
                                    .write()
                                    .chat_sessions
                                    .set(sessions);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to load chat sessions: {:?}",
                                e
                            );
                        }
                    }
                },
            );
        },
    );

    use_effect(
        move || {
            if let Some(messages) = app_data
                .read()
                .ai_chat
                .read()
                .as_ref()
            {
                if !messages.is_empty() {
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            if let Some(container) =
                                document.get_element_by_id("chat-messages-container")
                            {
                                container.set_scroll_top(container.scroll_height());
                            }
                        }
                    }
                }
            }
        },
    );

    rsx! {
        div {
            class: "chat-container",

            div {
                class: "chat-header",
                h2 { "Movie Collection Chat" }
                p { class: "chat-subtitle", "Ask me anything about your DVD collection" }
                div { class: "header-actions",
                    if let Some(session_id) = app_data.read().chat_session_id.read().as_ref() {
                        div { class: "session-id-display",
                            "Session: {session_id}"
                        }
                    }
                    div { class: "chat-mode-toggle",
                        button {
                            class: if !tool_mode() { "mode-btn active" } else { "mode-btn" },
                            disabled: *is_loading.read(),
                            onclick: move |_| tool_mode.set(false),
                            title: "Streaming RAG — pre-injects matching movies into the prompt",
                            "RAG"
                        }
                        button {
                            class: if tool_mode() { "mode-btn active" } else { "mode-btn" },
                            disabled: *is_loading.read(),
                            onclick: move |_| tool_mode.set(true),
                            title: "Tool calling — LLM picks tools to query the collection",
                            "Tools"
                        }
                    }
                    div { class: "model-selector-container",
                        span { class: "model-label", "Model:" }
                        select {
                            class: "model-select",
                            disabled: *is_loading.read(),
                            value: "{selected_model}",
                            onchange: move |evt| selected_model.set(evt.value()),
                            // Always include the current default so it stays selectable even if /ai/models hasn't loaded yet
                            if !available_models.read().iter().any(|m| m.name == *selected_model.read()) {
                                option { value: "{selected_model}", "{selected_model}" }
                            }
                            for model in available_models.read().iter() {
                                option {
                                    key: "{model.name}",
                                    value: "{model.name}",
                                    "{model.name}"
                                }
                            }
                        }
                    }
                    button {
                        class: "history-toggle",
                        onclick: move |_| {
                            let new_state = !show_history();
                            show_history.set(new_state);

                            // Toggle body scroll
                            if let Some(window) = web_sys::window() {
                                if let Some(document) = window.document() {
                                    if let Some(body) = document.body() {
                                        if new_state {
                                            body.set_class_name("modal-open");
                                        } else {
                                            body.set_class_name("");
                                        }
                                    }
                                }
                            }

                            spawn(async move {
                                let window = web_sys::window().unwrap();
                                let location = window.location();
                                let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                                let server_port = std::env::var("server_port").unwrap_or("3000".to_string());
                                let url = format!("http://{hostname}:{server_port}/ai/chat/sessions");

                                match gloo_net::http::Request::get(&url).send().await {
                                    Ok(response) => {
                                        if let Ok(sessions) = response.json::<Vec<ChatSession>>().await {
                                            app_data.write().chat_sessions.set(sessions);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to load chat sessions: {:?}", e);
                                    }
                                }
                            });
                        },
                        "Show History"
                    }
                    button {
                        class: "prompt-toggle",
                        onclick: move |_| show_prompt_editor.set(!show_prompt_editor()),
                        "Edit Prompts"
                    }
                }
            }

            if *show_prompt_editor.read() {
                div {
                    class: "prompt-editor-panel",
                    div { class: "prompt-editor-header",
                        h3 { "System Prompts" }
                        button {
                            class: "close-prompt-editor",
                            onclick: move |_| show_prompt_editor.set(false),
                            "×"
                        }
                    }
                    div { class: "prompt-tabs",
                        button {
                            class: if !tool_mode() { "prompt-tab active" } else { "prompt-tab" },
                            onclick: move |_| tool_mode.set(false),
                            "RAG Prompt"
                        }
                        button {
                            class: if tool_mode() { "prompt-tab active" } else { "prompt-tab" },
                            onclick: move |_| tool_mode.set(true),
                            "Tool Prompt"
                        }
                    }
                    if !tool_mode() {
                        div { class: "prompt-textarea-container",
                            label { "RAG Mode System Prompt:" }
                            textarea {
                                class: "prompt-textarea",
                                value: "{rag_prompt}",
                                rows: "15",
                                oninput: move |evt| rag_prompt.set(evt.value()),
                            }
                            div { class: "prompt-actions",
                                button {
                                    class: "reset-prompt-btn",
                                    onclick: move |_| rag_prompt.set(DEFAULT_RAG_PROMPT.to_string()),
                                    "Reset to Default"
                                }
                            }
                        }
                    } else {
                        div { class: "prompt-textarea-container",
                            label { "Tool Mode System Prompt:" }
                            textarea {
                                class: "prompt-textarea",
                                value: "{tool_prompt}",
                                rows: "15",
                                oninput: move |evt| tool_prompt.set(evt.value()),
                            }
                            div { class: "prompt-actions",
                                button {
                                    class: "reset-prompt-btn",
                                    onclick: move |_| tool_prompt.set(DEFAULT_TOOL_PROMPT.to_string()),
                                    "Reset to Default"
                                }
                            }
                        }
                    }
                }
            }

            if *show_history.read() {
                div {
                    class: "modal-overlay",
                    onclick: move |_| {
                        show_history.set(false);
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                if let Some(body) = document.body() {
                                    body.set_class_name("");
                                }
                            }
                        }
                    },
                    div {
                        class: "modal-content",
                        onclick: move |e| e.stop_propagation(),
                        h3 { "Chat History" }
                        button {
                            class: "new-chat-btn",
                            onclick: move |_| {
                                app_data.write().ai_chat.set(None);
                                app_data.write().chat_session_id.set(None);
                                show_history.set(false);
                                if let Some(window) = web_sys::window() {
                                    if let Some(document) = window.document() {
                                        if let Some(body) = document.body() {
                                            body.set_class_name("");
                                        }
                                    }
                                }
                            },
                            "+ New Chat"
                        }
                        div { class: "sessions-list",
                            for session in app_data.read().chat_sessions.read().clone() {
                                {
                                    let session_id_str = session.session_id.clone();
                                    let updated_at_str = session.updated_at.clone();
                                    let first_query = session.first_query.clone();
                                    rsx! {
                                        div {
                                            key: "{session_id_str}",
                                            class: "session-item",
                                            onclick: move |_| {
                                                let session_id = session_id_str.clone();
                                                app_data.write().chat_session_id.set(Some(session_id.clone()));
                                                show_history.set(false);

                                                if let Some(window) = web_sys::window() {
                                                    if let Some(document) = window.document() {
                                                        if let Some(body) = document.body() {
                                                            body.set_class_name("");
                                                        }
                                                    }
                                                }

                                                spawn(async move {
                                                    let window = web_sys::window().unwrap();
                                                    let location = window.location();
                                                    let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                                                    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());
                                                    let url = format!("http://{hostname}:{server_port}/ai/chat/sessions/{session_id}");

                                                    match gloo_net::http::Request::get(&url).send().await {
                                                        Ok(response) => {
                                                            #[derive(serde::Deserialize)]
                                                            struct ChatMessage {
                                                                role: String,
                                                                content: String,
                                                            }

                                                            if let Ok(messages) = response.json::<Vec<ChatMessage>>().await {
                                                                let roled_messages: Vec<RoledMessage> = messages.iter().map(|m| {
                                                                    RoledMessage {
                                                                        message: m.content.clone(),
                                                                        role: if m.role == "user" { Role::User } else { Role::Ai },
                                                                    }
                                                                }).collect();
                                                                app_data.write().ai_chat.set(Some(roled_messages));
                                                            }
                                                        }
                                                        Err(e) => {
                                                            error!("Failed to load session history: {:?}", e);
                                                        }
                                                    }
                                                });
                                            },
                                            if let Some(query) = first_query {
                                                div { class: "session-query",
                                                    "{query}"
                                                }
                                            }
                                            div { class: "session-date",
                                                "{updated_at_str}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
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
                                    if message.role == Role::Ai {
                                        Markdown { content: message.message.clone() }
                                    } else {
                                        div { class: "message-content", "{message.message}" }
                                    }
                                }
                            }
                        }

                        if *is_loading.read() {
                            div {
                                class: "message-wrapper ai-message",
                                div {
                                    class: "message-bubble streaming",
                                    div { class: "message-role", "Ai" }
                                    if streaming_response.read().is_empty() {
                                        div { class: "message-content",
                                            span { class: "cursor", "▋" }
                                        }
                                    } else {
                                        div {
                                            class: "message-content",
                                            style: "white-space: pre-wrap;",
                                            "{streaming_response.read()}"
                                        }
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

                            if tool_mode() {
                                spawn(send_tool_message(message, selected_model, tool_prompt, app_data, is_loading));
                            } else {
                                spawn(send_streaming_message(message, selected_model, rag_prompt, app_data, streaming_response, is_loading));
                            }
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

                        if tool_mode() {
                            spawn(send_tool_message(message, selected_model, tool_prompt, app_data, is_loading));
                        } else {
                            spawn(send_streaming_message(message, selected_model, rag_prompt, app_data, streaming_response, is_loading));
                        }
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
