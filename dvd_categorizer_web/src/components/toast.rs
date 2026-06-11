use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ToastMessage {
    pub id: u32,
    pub kind: ToastKind,
    pub message: String,
    /// Milliseconds before auto-dismiss. 0 = sticky (manual dismiss only).
    pub duration_ms: u32,
}

impl ToastMessage {
    pub fn info(id: u32, message: impl Into<String>) -> Self {
        Self {
            id,
            kind: ToastKind::Info,
            message: message.into(),
            duration_ms: 4000,
        }
    }
    pub fn success(id: u32, message: impl Into<String>) -> Self {
        Self {
            id,
            kind: ToastKind::Success,
            message: message.into(),
            duration_ms: 3000,
        }
    }
    pub fn warning(id: u32, message: impl Into<String>) -> Self {
        Self {
            id,
            kind: ToastKind::Warning,
            message: message.into(),
            duration_ms: 0,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct ToastContainerProps {
    pub toasts: ReadSignal<Vec<ToastMessage>>,
    pub on_dismiss: EventHandler<u32>,
}

#[component]
pub fn ToastContainer(props: ToastContainerProps) -> Element {
    rsx! {
        div { class: "toast-container",
            for toast in props.toasts.read().iter() {
                {
                    let toast_id = toast.id;
                    let on_dismiss = props.on_dismiss;
                    let duration = toast.duration_ms;
                    let kind_class = match toast.kind {
                        ToastKind::Info    => "toast toast-info",
                        ToastKind::Success => "toast toast-success",
                        ToastKind::Warning => "toast toast-warning",
                    };
                    let icon = match toast.kind {
                        ToastKind::Info    => "⏳",
                        ToastKind::Success => "✓",
                        ToastKind::Warning => "⚠",
                    };
                    let msg = toast.message.clone();

                    rsx! {
                        ToastItem {
                            key: "{toast_id}",
                            toast_id,
                            kind_class,
                            icon,
                            msg,
                            duration_ms: duration,
                            on_dismiss,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ToastItemProps {
    toast_id: u32,
    kind_class: &'static str,
    icon: &'static str,
    msg: String,
    duration_ms: u32,
    on_dismiss: EventHandler<u32>,
}

#[component]
fn ToastItem(props: ToastItemProps) -> Element {
    let on_dismiss = props.on_dismiss;
    let id = props.toast_id;
    let duration = props.duration_ms;

    use_effect(
        move || {
            if duration > 0 {
                let on_dismiss = on_dismiss;
                spawn(
                    async move {
                        gloo_timers::future::TimeoutFuture::new(duration).await;
                        on_dismiss.call(id);
                    },
                );
            }
        },
    );

    rsx! {
        div {
            class: "{props.kind_class}",
            span { class: "toast-icon", "{props.icon}" }
            span { class: "toast-msg", "{props.msg}" }
            button {
                class: "toast-close",
                onclick: move |_| props.on_dismiss.call(props.toast_id),
                "×"
            }
        }
    }
}
