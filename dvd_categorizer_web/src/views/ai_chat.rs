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

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

/// API base URL. Replaces the six duplicated hostname/port blocks.
fn api_base() -> String {
    let hostname = web_sys::window()
        .and_then(|w| w.location().hostname().ok())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = std::env::var("server_port").unwrap_or_else(|_| "3000".to_string());
    format!("http://{hostname}:{port}")
}

/// Every signal the chat flow needs, unwrapped from `AppData` once.
/// `Signal<T>` is `Copy`, so the whole struct is `Copy` — pass it by value
/// instead of threading six positional arguments through every function.
#[derive(Clone, Copy)]
struct ChatCtx {
    ai_chat: Signal<Option<Vec<RoledMessage>>>,
    session_id: Signal<Option<String>>,
    sessions: Signal<Vec<ChatSession>>,
    model: Signal<String>,
    rag_prompt: Signal<String>,
    tool_prompt: Signal<String>,
    streaming: Signal<String>,
    is_loading: Signal<bool>,
}

impl ChatCtx {
    fn loading(&self) -> bool {
        *self.is_loading.read()
    }

    fn model_name(&self) -> String {
        self.model.read().clone()
    }
}

fn use_chat_ctx() -> ChatCtx {
    let app_data = consume_context::<Signal<AppData>>();
    // Take the handles out, then drop the read guard immediately so nothing
    // downstream can collide with it.
    let (ai_chat, session_id, sessions) = {
        let d = app_data.read();
        (d.ai_chat, d.chat_session_id, d.chat_sessions)
    };

    ChatCtx {
        ai_chat,
        session_id,
        sessions,
        model: use_signal(|| "qwen2.5:7b".to_string()),
        rag_prompt: use_signal(|| DEFAULT_RAG_PROMPT.to_string()),
        tool_prompt: use_signal(|| DEFAULT_TOOL_PROMPT.to_string()),
        streaming: use_signal(String::new),
        is_loading: use_signal(|| false),
    }
}

/// Append a message. Takes ownership — callers pass a clone only when they
/// still need the message themselves.
fn push_message(mut ai_chat: Signal<Option<Vec<RoledMessage>>>, message: RoledMessage) {
    ai_chat.with_mut(|chat| match chat {
        Some(chat) => chat.push(message),
        None => *chat = Some(vec![message]),
    });
}

fn push_ai_error(ai_chat: Signal<Option<Vec<RoledMessage>>>, text: impl Into<String>) {
    push_message(
        ai_chat,
        RoledMessage {
            message: text.into(),
            role: Role::Ai,
        },
    );
}

// ---------------------------------------------------------------------------
// Transports
// ---------------------------------------------------------------------------

/// Non-streaming tool-enabled `/ai/chat`. The LLM may invoke tools that
/// self-call the API. One response, no streaming UI.
async fn send_tool_message(mut ctx: ChatCtx, message: String) {
    let user_message = RoledMessage {
        message,
        role: Role::User,
    };
    // Push a clone into history, keep the original for the request body.
    push_message(ctx.ai_chat, user_message.clone());

    let chat_request = ChatRequest {
        session_id: ctx.session_id.read().clone(),
        messages: vec![user_message],
        model: Some(ctx.model_name()),
        system_prompt: Some(ctx.tool_prompt.read().clone()),
    };

    let request = match gloo_net::http::Request::post(&format!("{}/ai/chat", api_base()))
        .header("Content-Type", "application/json")
        .json(&chat_request)
    {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to build tool chat request: {:?}", e);
            ctx.is_loading.set(false);
            return;
        }
    };

    match request.send().await {
        Ok(response) if !response.ok() => {
            error!("Tool chat returned status {}", response.status());
            push_ai_error(
                ctx.ai_chat,
                format!("AI service returned error {}", response.status()),
            );
        }
        Ok(response) => match response.json::<ChatResponse>().await {
            Ok(parsed) => {
                if let Some(sid) = parsed.session_id {
                    ctx.session_id.set(Some(sid));
                }
                push_message(
                    ctx.ai_chat,
                    RoledMessage {
                        message: parsed.message,
                        role: Role::Ai,
                    },
                );
            }
            Err(e) => {
                error!("Failed to parse tool chat response: {:?}", e);
                push_ai_error(ctx.ai_chat, "Failed to parse AI response");
            }
        },
        Err(e) => {
            error!("Tool chat fetch failed: {:?}", e);
            push_ai_error(ctx.ai_chat, "Failed to connect to AI service");
        }
    }

    ctx.is_loading.set(false);
}

/// Streaming RAG via `/ai/chat/stream`.
async fn send_streaming_message(mut ctx: ChatCtx, message: String) {
    let user_message = RoledMessage {
        message,
        role: Role::User,
    };
    push_message(ctx.ai_chat, user_message.clone());

    // The live buffer doubles as the streaming signal — no local accumulator,
    // no clone per chunk.
    ctx.streaming.with_mut(String::clear);

    let chat_request = ChatRequest {
        session_id: ctx.session_id.read().clone(),
        messages: vec![user_message],
        model: Some(ctx.model_name()),
        system_prompt: Some(ctx.rag_prompt.read().clone()),
    };

    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::Cors);

    let body = match serde_json::to_string(&chat_request) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize request: {}", e);
            ctx.is_loading.set(false);
            return;
        }
    };
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let Some(window) = web_sys::window() else {
        ctx.is_loading.set(false);
        return;
    };

    let url = format!("{}/ai/chat/stream", api_base());

    let request = match web_sys::Request::new_with_str_and_init(&url, &opts) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to create request: {:?}", e);
            ctx.is_loading.set(false);
            return;
        }
    };

    if let Err(e) = request.headers().set("Content-Type", "application/json") {
        error!("Failed to set headers: {:?}", e);
        ctx.is_loading.set(false);
        return;
    }

    let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(v) => v,
        Err(e) => {
            error!("Fetch failed: {:?}", e);
            push_ai_error(ctx.ai_chat, "Failed to connect to AI service");
            ctx.is_loading.set(false);
            return;
        }
    };

    let resp: web_sys::Response = resp_value.dyn_into().unwrap();

    if !resp.ok() {
        error!("Response not OK: {}", resp.status());
        ctx.is_loading.set(false);
        return;
    }

    let Some(body) = resp.body() else {
        error!("No response body");
        ctx.is_loading.set(false);
        return;
    };

    let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();

    let mut buffer = String::new();

    loop {
        let chunk = match JsFuture::from(reader.read()).await {
            Ok(c) => c,
            Err(e) => {
                error!("Read error: {:?}", e);
                break;
            }
        };

        let done = js_sys::Reflect::get(&chunk, &"done".into())
            .ok()
            .and_then(|d| d.as_bool())
            .unwrap_or(true);
        if done {
            break;
        }

        let Some(value) = js_sys::Reflect::get(&chunk, &"value".into()).ok() else {
            break;
        };
        let bytes = js_sys::Uint8Array::new(&value).to_vec();

        if let Ok(text) = std::str::from_utf8(&bytes) {
            buffer.push_str(text);

            let mut current_event = String::new();
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer.drain(..=newline_pos);

                if let Some(event) = line.strip_prefix("event: ") {
                    current_event = event.to_string();
                } else if let Some(data) = line.strip_prefix("data: ") {
                    if current_event == "message" {
                        ctx.streaming.with_mut(|s| s.push_str(data));
                    } else if current_event == "session" {
                        ctx.session_id.set(Some(data.to_string()));
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

    // Commit the buffer as a real message first, then clear the live one so the
    // text doesn't flicker.
    let final_text = ctx.streaming.with_mut(std::mem::take);
    if !final_text.is_empty() {
        push_message(
            ctx.ai_chat,
            RoledMessage {
                message: final_text,
                role: Role::Ai,
            },
        );
    }

    ctx.is_loading.set(false);
}

/// Shared by the Enter key and the Send button.
fn do_send(mut ctx: ChatCtx, mut input: Signal<String>, tool_mode: Signal<bool>) {
    let message = input.read().trim().to_string();
    if message.is_empty() || ctx.loading() {
        return;
    }

    input.set(String::new());
    ctx.is_loading.set(true);

    if *tool_mode.read() {
        spawn(send_tool_message(ctx, message));
    } else {
        spawn(send_streaming_message(ctx, message));
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn AiChat() -> Element {
    // `mut` because set/with_mut take &mut self. `ctx` is Copy, so each
    // `move` closure below gets its own copy and mutates that.
    let mut ctx = use_chat_ctx();

    let mut input_value = use_signal(String::new);
    let mut show_history = use_signal(|| false);
    // false = streaming RAG, true = non-streaming tool calling
    let mut tool_mode = use_signal(|| false);
    let mut available_models = use_signal(Vec::<AvailableModel>::new);
    let mut show_prompt_editor = use_signal(|| false);

    // Fetch available models on mount
    use_effect(move || {
        let url = format!("{}/ai/models", api_base());
        spawn(async move {
            match gloo_net::http::Request::get(&url).send().await {
                Ok(response) => {
                    if let Ok(parsed) = response.json::<AvailableModelsResponse>().await {
                        available_models.set(parsed.models);
                    }
                }
                Err(e) => error!("Failed to load available models: {:?}", e),
            }
        });
    });

    // Load chat sessions on mount
    use_effect(move || {
        let url = format!("{}/ai/chat/sessions", api_base());
        spawn(async move {
            match gloo_net::http::Request::get(&url).send().await {
                Ok(response) => {
                    if let Ok(sessions) = response.json::<Vec<ChatSession>>().await {
                        ctx.sessions.set(sessions);
                    }
                }
                Err(e) => error!("Failed to load chat sessions: {:?}", e),
            }
        });
    });

    // Scroll to bottom when messages change
    use_effect(move || {
        let has_messages = ctx.ai_chat.read().as_ref().is_some_and(|m| !m.is_empty());
        if !has_messages {
            return;
        }
        if let Some(container) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("chat-messages-container"))
        {
            container.set_scroll_top(container.scroll_height());
        }
    });

    rsx! {
        div {
            class: "chat-container",

            div {
                class: "chat-header",
                h2 { "Movie Collection Chat" }
                p { class: "chat-subtitle", "Ask me anything about your DVD collection" }
                div { class: "header-actions",
                    if let Some(session_id) = ctx.session_id.read().as_ref() {
                        div { class: "session-id-display",
                            "Session: {session_id}"
                        }
                    }
                    div { class: "chat-mode-toggle",
                        button {
                            class: if !*tool_mode.read() { "mode-btn active" } else { "mode-btn" },
                            disabled: ctx.loading(),
                            onclick: move |_| tool_mode.with_mut(|v| *v = false),
                            title: "Streaming RAG — pre-injects matching movies into the prompt",
                            "RAG"
                        }
                        button {
                            class: if *tool_mode.read() { "mode-btn active" } else { "mode-btn" },
                            disabled: ctx.loading(),
                            onclick: move |_| tool_mode.with_mut(|v| *v = true),
                            title: "Tool calling — LLM picks tools to query the collection",
                            "Tools"
                        }
                    }
                    div { class: "model-selector-container",
                        span { class: "model-label", "Model:" }
                        select {
                            class: "model-select",
                            disabled: ctx.loading(),
                            value: "{ctx.model}",
                            onchange: move |evt| ctx.model.set(evt.value()),
                            // Keep the current default selectable before /ai/models resolves
                            if !available_models.read().iter().any(|m| m.name == *ctx.model.read()) {
                                option { value: "{ctx.model}", "{ctx.model}" }
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
                            let new_state = show_history.with_mut(|v| {
                                *v = !*v;
                                *v
                            });

                            if let Some(body) = web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| d.body())
                            {
                                body.set_class_name(if new_state { "modal-open" } else { "" });
                            }

                            let url = format!("{}/ai/chat/sessions", api_base());
                            spawn(async move {
                                match gloo_net::http::Request::get(&url).send().await {
                                    Ok(response) => {
                                        if let Ok(sessions) = response.json::<Vec<ChatSession>>().await {
                                            ctx.sessions.set(sessions);
                                        }
                                    }
                                    Err(e) => error!("Failed to load chat sessions: {:?}", e),
                                }
                            });
                        },
                        "Show History"
                    }
                    button {
                        class: "prompt-toggle",
                        onclick: move |_| show_prompt_editor.with_mut(|v| *v = !*v),
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
                            onclick: move |_| show_prompt_editor.with_mut(|v| *v = false),
                            "×"
                        }
                    }
                    div { class: "prompt-tabs",
                        button {
                            class: if !*tool_mode.read() { "prompt-tab active" } else { "prompt-tab" },
                            onclick: move |_| tool_mode.with_mut(|v| *v = false),
                            "RAG Prompt"
                        }
                        button {
                            class: if *tool_mode.read() { "prompt-tab active" } else { "prompt-tab" },
                            onclick: move |_| tool_mode.with_mut(|v| *v = true),
                            "Tool Prompt"
                        }
                    }
                    if !*tool_mode.read() {
                        div { class: "prompt-textarea-container",
                            label { "RAG Mode System Prompt:" }
                            textarea {
                                class: "prompt-textarea",
                                value: "{ctx.rag_prompt}",
                                rows: "15",
                                oninput: move |evt| ctx.rag_prompt.set(evt.value()),
                            }
                            div { class: "prompt-actions",
                                button {
                                    class: "reset-prompt-btn",
                                    onclick: move |_| ctx.rag_prompt.set(DEFAULT_RAG_PROMPT.to_string()),
                                    "Reset to Default"
                                }
                            }
                        }
                    } else {
                        div { class: "prompt-textarea-container",
                            label { "Tool Mode System Prompt:" }
                            textarea {
                                class: "prompt-textarea",
                                value: "{ctx.tool_prompt}",
                                rows: "15",
                                oninput: move |evt| ctx.tool_prompt.set(evt.value()),
                            }
                            div { class: "prompt-actions",
                                button {
                                    class: "reset-prompt-btn",
                                    onclick: move |_| ctx.tool_prompt.set(DEFAULT_TOOL_PROMPT.to_string()),
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
                        show_history.with_mut(|v| *v = false);
                        if let Some(body) = web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.body())
                        {
                            body.set_class_name("");
                        }
                    },
                    div {
                        class: "modal-content",
                        onclick: move |e| e.stop_propagation(),
                        h3 { "Chat History" }
                        button {
                            class: "new-chat-btn",
                            onclick: move |_| {
                                ctx.ai_chat.set(None);
                                ctx.session_id.set(None);
                                show_history.with_mut(|v| *v = false);
                                if let Some(body) = web_sys::window()
                                    .and_then(|w| w.document())
                                    .and_then(|d| d.body())
                                {
                                    body.set_class_name("");
                                }
                            },
                            "+ New Chat"
                        }
                        div { class: "sessions-list",
                            // Borrow the list; only the id is cloned, once.
                            for session in ctx.sessions.read().iter() {
                                {
                                    let session_id_str = session.session_id.clone();
                                    rsx! {
                                        div {
                                            key: "{session_id_str}",
                                            class: "session-item",
                                            onclick: move |_| {
                                                ctx.session_id.set(Some(session_id_str.clone()));
                                                show_history.with_mut(|v| *v = false);

                                                if let Some(body) = web_sys::window()
                                                    .and_then(|w| w.document())
                                                    .and_then(|d| d.body())
                                                {
                                                    body.set_class_name("");
                                                }

                                                // Build the URL here so the coroutine
                                                // owns only `url`, never the capture.
                                                let url = format!(
                                                    "{}/ai/chat/sessions/{session_id_str}",
                                                    api_base()
                                                );

                                                spawn(async move {
                                                    match gloo_net::http::Request::get(&url).send().await {
                                                        Ok(response) => {
                                                            #[derive(serde::Deserialize)]
                                                            struct ChatMessage {
                                                                role: String,
                                                                content: String,
                                                            }

                                                            if let Ok(messages) = response.json::<Vec<ChatMessage>>().await {
                                                                // Content is owned by the deserialized
                                                                // messages, so move it out.
                                                                let roled_messages: Vec<RoledMessage> = messages
                                                                    .into_iter()
                                                                    .map(|m| RoledMessage {
                                                                        message: m.content,
                                                                        role: if m.role == "user" {
                                                                            Role::User
                                                                        } else {
                                                                            Role::Ai
                                                                        },
                                                                    })
                                                                    .collect();
                                                                ctx.ai_chat.set(Some(roled_messages));
                                                            }
                                                        }
                                                        Err(e) => error!("Failed to load session history: {:?}", e),
                                                    }
                                                });
                                            },
                                            if let Some(query) = &session.first_query {
                                                div { class: "session-query",
                                                    "{query}"
                                                }
                                            }
                                            div { class: "session-date",
                                                "{session.updated_at}"
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

                if let Some(messages) = ctx.ai_chat.read().as_ref() {
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

                        if ctx.loading() {
                            div {
                                class: "message-wrapper ai-message",
                                div {
                                    class: "message-bubble streaming",
                                    div { class: "message-role", "Ai" }
                                    if ctx.streaming.read().is_empty() {
                                        div { class: "message-content",
                                            span { class: "cursor", "▋" }
                                        }
                                    } else {
                                        div {
                                            class: "message-content",
                                            style: "white-space: pre-wrap;",
                                            "{ctx.streaming.read()}"
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
                            do_send(ctx, input_value, tool_mode);
                        }
                    }
                }
                button {
                    class: "send-button",
                    disabled: input_value.read().trim().is_empty() || ctx.loading(),
                    onclick: move |_| do_send(ctx, input_value, tool_mode),
                    if ctx.loading() { "Sending..." } else { "Send" }
                }
            }
        }
    }
}
